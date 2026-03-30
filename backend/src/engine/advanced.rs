//! Advanced workflow engine features:
//! - Dynamic workflow modification (102)
//! - Workflow inter-communication / signals (103)
//! - Saga pattern compensation (104)
//! - Workflow checkpointing (105)
//! - Workflow inheritance resolution (108)
//! - Task dependency graph validation (109)
//! - Long-running task heartbeat (110)

use std::collections::{HashMap, HashSet};

use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;

use super::WorkflowEngine;
use super::error::EngineError;
use crate::models::*;

impl WorkflowEngine {
    // ══════════════════════════════════════════════════════════════════
    // 102. Dynamic Workflow Modification
    // ══════════════════════════════════════════════════════════════════

    /// Modify a running workflow by adding/removing tasks that have not yet been scheduled.
    pub async fn modify_running_workflow(
        &self,
        workflow_id: &str,
        req: &ModifyWorkflowRequest,
    ) -> Result<Workflow, EngineError> {
        let wf = self.get_workflow(workflow_id).await?;
        if wf.status != WorkflowStatus::Running {
            return Err(EngineError::InvalidState(format!(
                "Workflow {workflow_id} is not RUNNING (status: {})",
                wf.status
            )));
        }

        let Some(mut def) = wf.workflow_definition else {
            return Err(EngineError::InvalidState(
                "Workflow has no embedded definition".into(),
            ));
        };

        // Collect already-scheduled task reference names
        let scheduled_refs: HashSet<String> = wf
            .tasks
            .iter()
            .map(|t| t.reference_task_name.clone())
            .collect();

        // Remove tasks (only unscheduled ones)
        for ref_name in &req.remove_task_refs {
            if scheduled_refs.contains(ref_name) {
                return Err(EngineError::InvalidState(format!(
                    "Cannot remove already-scheduled task: {ref_name}"
                )));
            }
            def.tasks.retain(|t| t.task_reference_name != *ref_name);
        }

        // Add new tasks
        for task in &req.add_tasks {
            if scheduled_refs.contains(&task.task_reference_name) {
                return Err(EngineError::InvalidState(format!(
                    "Task reference name already exists: {}",
                    task.task_reference_name
                )));
            }
            // Check for duplicate among newly-added tasks
            if def
                .tasks
                .iter()
                .any(|t| t.task_reference_name == task.task_reference_name)
            {
                return Err(EngineError::InvalidState(format!(
                    "Duplicate task reference name in definition: {}",
                    task.task_reference_name
                )));
            }
            def.tasks.push(task.clone());
        }

        // Validate the new definition
        let validation = self.validate_workflow_def_internal(&def);
        if !validation.valid {
            return Err(EngineError::InvalidState(format!(
                "Modified workflow definition is invalid: {}",
                validation.errors.join("; ")
            )));
        }

        // Update the embedded definition
        let db = self.shards.shard_for(workflow_id);
        let def_json = serde_json::to_value(&def).map_err(|e| EngineError::Serde(e.to_string()))?;
        sqlx::query(
            "UPDATE workflow SET workflow_def = $2, update_time = NOW() WHERE workflow_id = $1",
        )
        .bind(workflow_id)
        .bind(&def_json)
        .execute(db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        // Re-advance to pick up new tasks
        let _ = self.advance_workflow(workflow_id).await;

        tracing::info!(
            workflow_id = %workflow_id,
            added = req.add_tasks.len(),
            removed = req.remove_task_refs.len(),
            "Workflow dynamically modified"
        );

        self.get_workflow(workflow_id).await
    }

    // ══════════════════════════════════════════════════════════════════
    // 103. Workflow Inter-Communication (Signals)
    // ══════════════════════════════════════════════════════════════════

    /// Send a signal to one or more waiting workflows.
    /// Finds WAIT_FOR_SIGNAL tasks with matching signal_name and completes them.
    pub async fn send_signal(
        &self,
        signal_name: &str,
        payload: &Value,
    ) -> Result<u64, EngineError> {
        let mut completed = 0u64;

        for shard in self.shards.all_shards() {
            // Find IN_PROGRESS tasks of type WAIT_FOR_SIGNAL that match the signal name.
            // The signal name is stored in the task's input_data.signalName field.
            let waiting: Vec<(String, String)> = sqlx::query_as(
                "SELECT task_id, workflow_instance_id FROM task \
                 WHERE status = 'IN_PROGRESS' AND task_type = 'WAIT_FOR_SIGNAL' \
                   AND input_data->>'signalName' = $1",
            )
            .bind(signal_name)
            .fetch_all(shard)
            .await
            .map_err(|e| EngineError::Database(e.to_string()))?;

            let now = Utc::now();
            for (task_id, workflow_id) in &waiting {
                let output = serde_json::json!({
                    "signalName": signal_name,
                    "signalPayload": payload,
                    "receivedAt": now.timestamp_millis()
                });

                let result = sqlx::query(
                    "UPDATE task SET status = 'COMPLETED', output_data = $2, end_time = $3, update_time = $3 \
                     WHERE task_id = $1 AND status = 'IN_PROGRESS'",
                )
                .bind(task_id)
                .bind(&output)
                .bind(now)
                .execute(shard)
                .await
                .map_err(|e| EngineError::Database(e.to_string()))?;

                if result.rows_affected() > 0 {
                    completed += 1;
                    // Advance the parent workflow
                    if let Err(e) = self.advance_workflow(workflow_id).await {
                        tracing::error!(workflow_id = %workflow_id, error = %e, "Failed to advance workflow after signal delivery");
                    }
                }
            }
        }

        tracing::info!(signal = %signal_name, delivered = completed, "Signal sent");
        Ok(completed)
    }

    // ══════════════════════════════════════════════════════════════════
    // 104. Saga Pattern — Compensation on Failure
    // ══════════════════════════════════════════════════════════════════

    /// Run compensation tasks in reverse order for a failed saga workflow.
    /// Called from `fail_workflow` when `saga_enabled = true`.
    pub(crate) async fn run_saga_compensation(
        &self,
        workflow_id: &str,
        def: &WorkflowDef,
        completed_tasks: &[TaskResult],
    ) -> Result<(), EngineError> {
        // Collect completed tasks with compensation, in reverse execution order
        let mut compensations: Vec<(&WorkflowTask, &TaskResult)> = Vec::new();

        for completed in completed_tasks.iter().rev() {
            if let Some(def_task) = def
                .tasks
                .iter()
                .find(|t| t.task_reference_name == completed.reference_task_name)
            {
                if def_task.compensation_task.is_some() {
                    compensations.push((def_task, completed));
                }
            }
        }

        if compensations.is_empty() {
            return Ok(());
        }

        tracing::info!(
            workflow_id = %workflow_id,
            compensations = compensations.len(),
            "Running saga compensation tasks"
        );

        // Run compensations sequentially in reverse order
        for (idx, (def_task, completed)) in compensations.iter().enumerate() {
            let comp = def_task.compensation_task.as_ref().unwrap();
            let comp_input = serde_json::json!({
                "originalTaskRef": completed.reference_task_name,
                "originalTaskOutput": completed.output_data,
                "compensationIndex": idx,
            });

            let seq = 10000 + idx as i32;
            match comp.task_type.as_str() {
                "HTTP" => {
                    if let Err(e) = self
                        .handle_http_task(workflow_id, comp, &comp_input, seq)
                        .await
                    {
                        tracing::error!(
                            workflow_id = %workflow_id,
                            compensation_ref = %comp.task_reference_name,
                            error = %e,
                            "Saga compensation task failed"
                        );
                    }
                }
                "LAMBDA" | "INLINE" => {
                    if let Err(e) = self
                        .handle_lambda_task(workflow_id, comp, &comp_input, seq)
                        .await
                    {
                        tracing::error!(
                            workflow_id = %workflow_id,
                            compensation_ref = %comp.task_reference_name,
                            error = %e,
                            "Saga compensation task failed"
                        );
                    }
                }
                _ => {
                    // Queue as a worker task
                    if let Err(e) = self
                        .create_and_queue_worker_task(workflow_id, comp, &comp_input, seq)
                        .await
                    {
                        tracing::error!(
                            workflow_id = %workflow_id,
                            compensation_ref = %comp.task_reference_name,
                            error = %e,
                            "Saga compensation task failed to queue"
                        );
                    }
                }
            }
        }

        Ok(())
    }

    // ══════════════════════════════════════════════════════════════════
    // 105. Workflow Checkpointing
    // ══════════════════════════════════════════════════════════════════

    /// Create a checkpoint (snapshot) of the current workflow state.
    pub async fn create_checkpoint(
        &self,
        workflow_id: &str,
        label: Option<&str>,
    ) -> Result<WorkflowCheckpoint, EngineError> {
        let wf = self.get_workflow(workflow_id).await?;
        let db = self.shards.shard_for(workflow_id);
        let checkpoint_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let wf_snapshot = serde_json::to_value(&wf).map_err(|e| EngineError::Serde(e.to_string()))?;
        let tasks_snapshot =
            serde_json::to_value(&wf.tasks).map_err(|e| EngineError::Serde(e.to_string()))?;
        let vars_snapshot = serde_json::to_value(&wf.variables)
            .map_err(|e| EngineError::Serde(e.to_string()))?;

        sqlx::query(
            "INSERT INTO workflow_checkpoint (checkpoint_id, workflow_id, created_at, workflow_snapshot, tasks_snapshot, variables_snapshot, label) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&checkpoint_id)
        .bind(workflow_id)
        .bind(now)
        .bind(&wf_snapshot)
        .bind(&tasks_snapshot)
        .bind(&vars_snapshot)
        .bind(label)
        .execute(db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        tracing::info!(workflow_id = %workflow_id, checkpoint_id = %checkpoint_id, "Checkpoint created");

        Ok(WorkflowCheckpoint {
            checkpoint_id,
            workflow_id: workflow_id.to_string(),
            created_at: now.timestamp_millis(),
            workflow_snapshot: wf_snapshot,
            tasks_snapshot,
            variables_snapshot: vars_snapshot,
            label: label.map(|s| s.to_string()),
        })
    }

    /// List all checkpoints for a workflow.
    pub async fn list_checkpoints(
        &self,
        workflow_id: &str,
    ) -> Result<Vec<WorkflowCheckpoint>, EngineError> {
        let db = self.shards.shard_for(workflow_id);
        let rows: Vec<(String, String, chrono::DateTime<Utc>, Value, Value, Value, Option<String>)> =
            sqlx::query_as(
                "SELECT checkpoint_id, workflow_id, created_at, workflow_snapshot, tasks_snapshot, variables_snapshot, label \
                 FROM workflow_checkpoint WHERE workflow_id = $1 ORDER BY created_at DESC",
            )
            .bind(workflow_id)
            .fetch_all(db)
            .await
            .map_err(|e| EngineError::Database(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|(id, wf_id, ts, wf_snap, tasks_snap, vars_snap, label)| WorkflowCheckpoint {
                checkpoint_id: id,
                workflow_id: wf_id,
                created_at: ts.timestamp_millis(),
                workflow_snapshot: wf_snap,
                tasks_snapshot: tasks_snap,
                variables_snapshot: vars_snap,
                label,
            })
            .collect())
    }

    /// Restore a workflow from a checkpoint — terminates current execution
    /// and starts a new one from the checkpoint state.
    pub async fn restore_checkpoint(
        &self,
        workflow_id: &str,
        checkpoint_id: &str,
    ) -> Result<String, EngineError> {
        let db = self.shards.shard_for(workflow_id);

        let row: Option<(Value, Value)> = sqlx::query_as(
            "SELECT workflow_snapshot, variables_snapshot FROM workflow_checkpoint \
             WHERE checkpoint_id = $1 AND workflow_id = $2",
        )
        .bind(checkpoint_id)
        .bind(workflow_id)
        .fetch_optional(db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        let (wf_snapshot, vars_snapshot) = row.ok_or_else(|| {
            EngineError::NotFound(format!(
                "Checkpoint {checkpoint_id} not found for workflow {workflow_id}"
            ))
        })?;

        // Parse the snapshot to get the original definition and input
        let orig_wf: Workflow = serde_json::from_value(wf_snapshot)
            .map_err(|e| EngineError::Serde(format!("Invalid checkpoint snapshot: {e}")))?;

        // Terminate current workflow
        let _ = self.terminate_workflow(workflow_id, Some("Restored from checkpoint")).await;

        // Start a new workflow with the original definition
        let def = orig_wf.workflow_definition.ok_or_else(|| {
            EngineError::InvalidState("Checkpoint missing workflow definition".into())
        })?;

        let new_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let new_db = self.shards.shard_for(&new_id);

        sqlx::query(
            "INSERT INTO workflow (workflow_id, workflow_name, workflow_version, status, input, correlation_id, start_time, update_time, priority, workflow_def, variables) \
             VALUES ($1, $2, $3, 'RUNNING', $4, $5, $6, $6, $7, $8, $9)",
        )
        .bind(&new_id)
        .bind(&orig_wf.workflow_name)
        .bind(orig_wf.workflow_version)
        .bind(&orig_wf.input)
        .bind(&orig_wf.correlation_id)
        .bind(now)
        .bind(orig_wf.priority)
        .bind(serde_json::to_value(&def).ok())
        .bind(&vars_snapshot)
        .execute(new_db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        Box::pin(self.schedule_tasks(&new_id, &def.tasks, &orig_wf.input, 0)).await?;

        tracing::info!(
            workflow_id = %new_id,
            original_id = %workflow_id,
            checkpoint_id = %checkpoint_id,
            "Workflow restored from checkpoint"
        );
        Ok(new_id)
    }

    // ══════════════════════════════════════════════════════════════════
    // 108. Workflow Inheritance
    // ══════════════════════════════════════════════════════════════════

    /// Resolve workflow inheritance by merging parent tasks into the child.
    /// The child's tasks override parent tasks with the same reference name.
    pub async fn resolve_inheritance(
        &self,
        def: &WorkflowDef,
    ) -> Result<WorkflowDef, EngineError> {
        let Some(ref base_name) = def.base_workflow else {
            return Ok(def.clone());
        };

        // Prevent circular inheritance (max depth 10)
        let mut chain = vec![def.name.clone()];
        let mut current = def.clone();
        let mut depth = 0;

        loop {
            let Some(ref parent_name) = current.base_workflow else {
                break;
            };

            if chain.contains(parent_name) {
                return Err(EngineError::InvalidState(format!(
                    "Circular workflow inheritance detected: {} -> {}",
                    chain.join(" -> "),
                    parent_name
                )));
            }

            depth += 1;
            if depth > 10 {
                return Err(EngineError::InvalidState(
                    "Workflow inheritance chain exceeds maximum depth of 10".into(),
                ));
            }

            chain.push(parent_name.clone());
            current = self
                .get_workflow_def(parent_name, current.base_workflow_version)
                .await
                .map_err(|_| {
                    EngineError::NotFound(format!("Base workflow not found: {base_name}"))
                })?;
        }

        // Now merge: walk back from root parent to child, accumulating tasks
        // Child tasks override parent tasks with the same reference name
        let mut resolved = def.clone();
        let mut parent_tasks: Vec<WorkflowTask> = Vec::new();

        // Walk chain from root to child (skip the first entry which is the child)
        let mut ancestors = Vec::new();
        let mut visit = def.clone();
        while let Some(ref pname) = visit.base_workflow {
            let parent = self
                .get_workflow_def(pname, visit.base_workflow_version)
                .await?;
            ancestors.push(parent.clone());
            visit = parent;
        }

        // Process from root ancestor to most immediate parent
        for ancestor in ancestors.into_iter().rev() {
            for task in &ancestor.tasks {
                if !parent_tasks
                    .iter()
                    .any(|t| t.task_reference_name == task.task_reference_name)
                {
                    parent_tasks.push(task.clone());
                }
            }
        }

        // Child tasks override parents; append parent-only tasks at the front
        let child_refs: HashSet<&str> = resolved
            .tasks
            .iter()
            .map(|t| t.task_reference_name.as_str())
            .collect();

        let mut merged: Vec<WorkflowTask> = parent_tasks
            .into_iter()
            .filter(|t| !child_refs.contains(t.task_reference_name.as_str()))
            .collect();

        merged.extend(resolved.tasks.drain(..));
        resolved.tasks = merged;
        resolved.base_workflow = None; // Flatten

        Ok(resolved)
    }

    // ══════════════════════════════════════════════════════════════════
    // 109. Task Dependency Graph Validation
    // ══════════════════════════════════════════════════════════════════

    /// Validate a workflow definition's task graph for correctness.
    pub fn validate_workflow_def(&self, def: &WorkflowDef) -> ValidationResult {
        self.validate_workflow_def_internal(def)
    }

    pub(crate) fn validate_workflow_def_internal(&self, def: &WorkflowDef) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        if def.name.is_empty() {
            errors.push("Workflow name is empty".into());
        }
        if def.tasks.is_empty() {
            errors.push("Workflow has no tasks".into());
            return ValidationResult {
                valid: false,
                errors,
                warnings,
            };
        }

        // Collect all task reference names and check for duplicates
        let mut ref_names: HashSet<String> = HashSet::new();
        let all_tasks = collect_all_tasks(&def.tasks);

        for task in &all_tasks {
            if task.task_reference_name.is_empty() {
                errors.push(format!("Task '{}' has empty reference name", task.name));
            }
            if !ref_names.insert(task.task_reference_name.clone()) {
                errors.push(format!(
                    "Duplicate task reference name: {}",
                    task.task_reference_name
                ));
            }
        }

        // Validate JOIN tasks reference valid tasks
        for task in &all_tasks {
            if task.task_type == "JOIN" {
                for join_ref in &task.join_on {
                    if !ref_names.contains(join_ref) {
                        errors.push(format!(
                            "JOIN task '{}' references non-existent task: {}",
                            task.task_reference_name, join_ref
                        ));
                    }
                }
            }
        }

        // Validate FORK tasks have matching JOIN
        for (idx, task) in def.tasks.iter().enumerate() {
            if task.task_type == "FORK_JOIN" || task.task_type == "FORK" {
                if task.fork_tasks.is_empty() {
                    errors.push(format!(
                        "FORK task '{}' has no branches",
                        task.task_reference_name
                    ));
                }
                // Check if next task is a JOIN
                if idx + 1 < def.tasks.len() {
                    let next = &def.tasks[idx + 1];
                    if next.task_type != "JOIN" {
                        warnings.push(format!(
                            "FORK task '{}' is not immediately followed by a JOIN",
                            task.task_reference_name
                        ));
                    }
                }
            }
        }

        // Validate DECISION cases
        for task in &all_tasks {
            if task.task_type == "DECISION" || task.task_type == "SWITCH" {
                if task.decision_cases.is_empty() && task.default_case.is_empty() {
                    errors.push(format!(
                        "DECISION task '{}' has no cases defined",
                        task.task_reference_name
                    ));
                }
                if task.case_value_param.is_none()
                    && task.case_expression.is_none()
                    && task.condition_tree.is_none()
                {
                    warnings.push(format!(
                        "DECISION task '{}' has no case_value_param, case_expression, or condition_tree",
                        task.task_reference_name
                    ));
                }
            }
        }

        // Validate SUB_WORKFLOW params
        for task in &all_tasks {
            if task.task_type == "SUB_WORKFLOW" && task.sub_workflow_param.is_none() {
                errors.push(format!(
                    "SUB_WORKFLOW task '{}' has no sub_workflow_param",
                    task.task_reference_name
                ));
            }
        }

        // Validate MAP tasks
        for task in &all_tasks {
            if task.task_type == "MAP" {
                if task.map_items_param.is_none() {
                    errors.push(format!(
                        "MAP task '{}' must specify map_items_param",
                        task.task_reference_name
                    ));
                }
                if task.map_task.is_none() {
                    errors.push(format!(
                        "MAP task '{}' must specify map_task template",
                        task.task_reference_name
                    ));
                }
            }
        }

        // Detect cycles using topological sort
        let has_cycle = detect_cycles(&def.tasks);
        if has_cycle {
            errors.push("Task dependency graph contains a cycle".into());
        }

        // Check for unreachable tasks
        let reachable = find_reachable_tasks(&def.tasks);
        for task in &all_tasks {
            if !reachable.contains(&task.task_reference_name) {
                warnings.push(format!(
                    "Task '{}' may be unreachable from the workflow start",
                    task.task_reference_name
                ));
            }
        }

        ValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    // ══════════════════════════════════════════════════════════════════
    // 110. Long-Running Task Heartbeat
    // ══════════════════════════════════════════════════════════════════

    /// Record a heartbeat for a long-running task, extending its lease.
    pub async fn heartbeat_task(&self, task_id: &str) -> Result<(), EngineError> {
        let (wf_id, shard) = self.resolve_task_shard(task_id).await?.ok_or_else(|| {
            EngineError::NotFound(format!("Task not found: {task_id}"))
        })?;

        let now = Utc::now();
        let result = sqlx::query(
            "UPDATE task SET update_time = $2 WHERE task_id = $1 AND status = 'IN_PROGRESS'",
        )
        .bind(task_id)
        .bind(now)
        .execute(shard)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(EngineError::NotFound(format!(
                "No IN_PROGRESS task found: {task_id}"
            )));
        }

        // Also store heartbeat time in Redis for fast lookup by sweeper
        let hb_key = format!("conductor:heartbeat:{}", task_id);
        let pool = self.redis.pool_for_key(&wf_id);
        if let Ok(mut conn) = pool.get().await {
            let _: Result<(), _> = deadpool_redis::redis::cmd("SET")
                .arg(&hb_key)
                .arg(now.timestamp_millis().to_string())
                .arg("EX")
                .arg(600_u32) // 10 min TTL
                .query_async(&mut *conn)
                .await;
        }

        tracing::debug!(task_id = %task_id, "Task heartbeat recorded");
        Ok(())
    }
}

// ── Helper functions ──────────────────────────────────────────────

/// Recursively collect all tasks from a workflow definition (including
/// nested tasks inside FORK branches, DECISION cases, DO_WHILE loops, etc.)
fn collect_all_tasks(tasks: &[WorkflowTask]) -> Vec<&WorkflowTask> {
    let mut result = Vec::new();
    for task in tasks {
        result.push(task);
        // FORK branches
        for branch in &task.fork_tasks {
            result.extend(collect_all_tasks(branch));
        }
        // DECISION cases
        for case_tasks in task.decision_cases.values() {
            result.extend(collect_all_tasks(case_tasks));
        }
        result.extend(collect_all_tasks(&task.default_case));
        // DO_WHILE loop body
        result.extend(collect_all_tasks(&task.loop_over));
        // MAP sub-task
        if let Some(ref map_task) = task.map_task {
            result.push(map_task.as_ref());
        }
    }
    result
}

/// Simple cycle detection using DFS coloring on the sequential task graph.
fn detect_cycles(tasks: &[WorkflowTask]) -> bool {
    // For the top-level linear task list, cycles can only occur via
    // back-references in JOIN.join_on or SUB_WORKFLOW references.
    // We build a graph where each task points to tasks that must execute after it.
    let ref_to_idx: HashMap<&str, usize> = tasks
        .iter()
        .enumerate()
        .map(|(i, t)| (t.task_reference_name.as_str(), i))
        .collect();

    let n = tasks.len();
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];

    // Sequential dependencies: task[i] -> task[i+1]
    for i in 0..n.saturating_sub(1) {
        adj[i].push(i + 1);
    }

    // JOIN dependencies (join_on tasks must complete before JOIN)
    for (i, task) in tasks.iter().enumerate() {
        if task.task_type == "JOIN" {
            for join_ref in &task.join_on {
                if let Some(&dep_idx) = ref_to_idx.get(join_ref.as_str()) {
                    adj[dep_idx].push(i);
                }
            }
        }
    }

    // DFS with coloring: 0=white, 1=gray, 2=black
    let mut color = vec![0u8; n];

    fn dfs(node: usize, adj: &[Vec<usize>], color: &mut [u8]) -> bool {
        color[node] = 1;
        for &next in &adj[node] {
            if color[next] == 1 {
                return true; // Back edge = cycle
            }
            if color[next] == 0 && dfs(next, adj, color) {
                return true;
            }
        }
        color[node] = 2;
        false
    }

    for i in 0..n {
        if color[i] == 0 && dfs(i, &adj, &mut color) {
            return true;
        }
    }

    false
}

/// Find all task reference names reachable from the workflow start.
fn find_reachable_tasks(tasks: &[WorkflowTask]) -> HashSet<String> {
    let mut reachable = HashSet::new();
    mark_reachable(tasks, &mut reachable);
    reachable
}

fn mark_reachable(tasks: &[WorkflowTask], reachable: &mut HashSet<String>) {
    for task in tasks {
        reachable.insert(task.task_reference_name.clone());
        for branch in &task.fork_tasks {
            mark_reachable(branch, reachable);
        }
        for case_tasks in task.decision_cases.values() {
            mark_reachable(case_tasks, reachable);
        }
        mark_reachable(&task.default_case, reachable);
        mark_reachable(&task.loop_over, reachable);
        if let Some(ref map_task) = task.map_task {
            reachable.insert(map_task.task_reference_name.clone());
        }
    }
}
