#!/bin/bash
# ─────────────────────────────────────────────────────────────────────
# rust-conductor Database Backup & Restore Script
#
# Usage:
#   ./scripts/db-backup.sh backup              # Full backup
#   ./scripts/db-backup.sh backup --schema     # Schema only
#   ./scripts/db-backup.sh restore <file>      # Restore from file
#   ./scripts/db-backup.sh list                # List backups in S3
#   ./scripts/db-backup.sh pitr <timestamp>    # Point-in-time recovery
# ─────────────────────────────────────────────────────────────────────
set -euo pipefail

# Defaults (override via environment)
: "${PGHOST:=localhost}"
: "${PGPORT:=5432}"
: "${PGUSER:=conductor}"
: "${PGDATABASE:=conductor}"
: "${BACKUP_DIR:=./backups}"
: "${S3_BUCKET:=}"
: "${S3_PREFIX:=conductor-backups}"
: "${RETENTION_DAYS:=30}"

TIMESTAMP=$(date +%Y%m%d_%H%M%S)
mkdir -p "${BACKUP_DIR}"

log() { echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*"; }

# ── Backup ────────────────────────────────────────────────────────────
do_backup() {
    local schema_only="${1:-false}"
    local suffix="full"
    local extra_args=""

    if [ "${schema_only}" = "true" ]; then
        suffix="schema"
        extra_args="--schema-only"
    fi

    local filename="conductor_${TIMESTAMP}_${suffix}.sql.gz"
    local filepath="${BACKUP_DIR}/${filename}"

    log "Starting ${suffix} backup of ${PGDATABASE}@${PGHOST}:${PGPORT}..."

    pg_dump \
        --host="${PGHOST}" \
        --port="${PGPORT}" \
        --username="${PGUSER}" \
        --dbname="${PGDATABASE}" \
        --format=custom \
        --compress=6 \
        --no-owner \
        --no-privileges \
        ${extra_args} \
        --file="${filepath}"

    local size
    size=$(stat -c%s "${filepath}" 2>/dev/null || stat -f%z "${filepath}")
    log "Backup complete: ${filepath} (${size} bytes)"

    # Upload to S3 if configured
    if [ -n "${S3_BUCKET}" ]; then
        log "Uploading to s3://${S3_BUCKET}/${S3_PREFIX}/${filename}..."
        aws s3 cp "${filepath}" "s3://${S3_BUCKET}/${S3_PREFIX}/${filename}"
        log "Upload complete"
    fi

    echo "${filepath}"
}

# ── Restore ───────────────────────────────────────────────────────────
do_restore() {
    local filepath="$1"

    if [ ! -f "${filepath}" ]; then
        # Try downloading from S3
        if [ -n "${S3_BUCKET}" ]; then
            local filename
            filename=$(basename "${filepath}")
            log "Downloading s3://${S3_BUCKET}/${S3_PREFIX}/${filename}..."
            aws s3 cp "s3://${S3_BUCKET}/${S3_PREFIX}/${filename}" "${BACKUP_DIR}/${filename}"
            filepath="${BACKUP_DIR}/${filename}"
        else
            log "ERROR: File not found: ${filepath}"
            exit 1
        fi
    fi

    log "WARNING: This will overwrite database ${PGDATABASE}. Ctrl+C to abort."
    log "Waiting 5 seconds..."
    sleep 5

    log "Restoring from ${filepath}..."
    pg_restore \
        --host="${PGHOST}" \
        --port="${PGPORT}" \
        --username="${PGUSER}" \
        --dbname="${PGDATABASE}" \
        --no-owner \
        --no-privileges \
        --clean \
        --if-exists \
        "${filepath}" || true  # pg_restore returns non-zero for warnings

    log "Restore complete. Verifying..."
    psql -h "${PGHOST}" -p "${PGPORT}" -U "${PGUSER}" -d "${PGDATABASE}" \
        -c "SELECT COUNT(*) AS workflow_count FROM workflow;" \
        -c "SELECT COUNT(*) AS task_count FROM task;" \
        -c "SELECT COUNT(*) AS workflow_def_count FROM workflow_def;" \
        -c "SELECT COUNT(*) AS task_def_count FROM task_def;"

    log "Restore verification complete"
}

# ── List Backups ──────────────────────────────────────────────────────
do_list() {
    if [ -n "${S3_BUCKET}" ]; then
        log "Listing backups in s3://${S3_BUCKET}/${S3_PREFIX}/..."
        aws s3 ls "s3://${S3_BUCKET}/${S3_PREFIX}/" --human-readable
    fi
    log "Local backups in ${BACKUP_DIR}/:"
    ls -lh "${BACKUP_DIR}/"*.sql.gz 2>/dev/null || echo "  (none)"
}

# ── Point-in-Time Recovery ────────────────────────────────────────────
do_pitr() {
    local target_time="$1"

    log "Point-in-time recovery to: ${target_time}"
    log "This requires WAL archiving and a base backup to be configured."
    log ""
    log "Steps for PostgreSQL PITR:"
    log "  1. Stop PostgreSQL"
    log "  2. Restore base backup to data directory"
    log "  3. Create recovery.signal file"
    log "  4. Set restore_command and recovery_target_time in postgresql.conf:"
    log "     restore_command = 'aws s3 cp s3://${S3_BUCKET}/wal/%f %p'"
    log "     recovery_target_time = '${target_time}'"
    log "     recovery_target_action = 'promote'"
    log "  5. Start PostgreSQL — it will replay WAL to the target time"
    log ""

    # For Kubernetes, create a recovery job
    cat <<EOF
---
# Apply this Job to trigger PITR in Kubernetes:
apiVersion: batch/v1
kind: Job
metadata:
  name: conductor-pitr-$(date +%s)
spec:
  template:
    spec:
      restartPolicy: Never
      containers:
        - name: pitr
          image: postgres:16-alpine
          env:
            - name: PGHOST
              value: "${PGHOST}"
            - name: PGUSER
              value: "${PGUSER}"
            - name: RECOVERY_TARGET
              value: "${target_time}"
          command: ["/bin/sh", "-c", "echo 'PITR requires manual intervention — see DBA runbook'"]
EOF
}

# ── Prune ─────────────────────────────────────────────────────────────
do_prune() {
    log "Pruning backups older than ${RETENTION_DAYS} days..."

    # Local
    find "${BACKUP_DIR}" -name "conductor_*.sql.gz" -mtime "+${RETENTION_DAYS}" -delete -print | while read -r f; do
        log "  Deleted local: ${f}"
    done

    # S3
    if [ -n "${S3_BUCKET}" ]; then
        local cutoff
        cutoff=$(date -d "-${RETENTION_DAYS} days" +%Y%m%d 2>/dev/null || date -v-${RETENTION_DAYS}d +%Y%m%d)
        aws s3 ls "s3://${S3_BUCKET}/${S3_PREFIX}/" | while read -r line; do
            file_date=$(echo "$line" | grep -oP '\d{8}' | head -1 || true)
            if [ -n "${file_date}" ] && [ "${file_date}" -lt "${cutoff}" ]; then
                file_name=$(echo "$line" | awk '{print $4}')
                aws s3 rm "s3://${S3_BUCKET}/${S3_PREFIX}/${file_name}"
                log "  Deleted S3: ${file_name}"
            fi
        done
    fi

    log "Prune complete"
}

# ── Main ──────────────────────────────────────────────────────────────
case "${1:-help}" in
    backup)
        if [ "${2:-}" = "--schema" ]; then
            do_backup true
        else
            do_backup false
        fi
        ;;
    restore)
        [ -z "${2:-}" ] && { log "Usage: $0 restore <backup-file>"; exit 1; }
        do_restore "$2"
        ;;
    list)
        do_list
        ;;
    pitr)
        [ -z "${2:-}" ] && { log "Usage: $0 pitr <timestamp>  (e.g. '2026-03-24 10:00:00')"; exit 1; }
        do_pitr "$2"
        ;;
    prune)
        do_prune
        ;;
    *)
        echo "Usage: $0 {backup|restore|list|pitr|prune}"
        echo ""
        echo "Commands:"
        echo "  backup [--schema]     Create a full or schema-only backup"
        echo "  restore <file>        Restore from a backup file (or S3 key)"
        echo "  list                  List available backups"
        echo "  pitr <timestamp>      Point-in-time recovery instructions"
        echo "  prune                 Delete backups older than RETENTION_DAYS"
        echo ""
        echo "Environment variables:"
        echo "  PGHOST, PGPORT, PGUSER, PGPASSWORD, PGDATABASE"
        echo "  S3_BUCKET, S3_PREFIX, RETENTION_DAYS, BACKUP_DIR"
        exit 1
        ;;
esac
