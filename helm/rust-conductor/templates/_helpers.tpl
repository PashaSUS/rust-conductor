{{/*
Expand the name of the chart.
*/}}
{{- define "rust-conductor.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "rust-conductor.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Common labels.
*/}}
{{- define "rust-conductor.labels" -}}
helm.sh/chart: {{ include "rust-conductor.name" . }}-{{ .Chart.Version | replace "+" "_" }}
{{ include "rust-conductor.selectorLabels" . }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels for the backend.
*/}}
{{- define "rust-conductor.selectorLabels" -}}
app.kubernetes.io/name: {{ include "rust-conductor.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Backend selector labels.
*/}}
{{- define "rust-conductor.backend.selectorLabels" -}}
app.kubernetes.io/name: {{ include "rust-conductor.name" . }}-backend
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/component: backend
{{- end }}

{{/*
Frontend selector labels.
*/}}
{{- define "rust-conductor.frontend.selectorLabels" -}}
app.kubernetes.io/name: {{ include "rust-conductor.name" . }}-frontend
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/component: frontend
{{- end }}

{{/*
Postgres connection URL.
*/}}
{{- define "rust-conductor.postgresUrl" -}}
{{- if .Values.postgresql.enabled -}}
postgres://{{ .Values.postgresql.auth.username }}:{{ .Values.postgresql.auth.password }}@{{ include "rust-conductor.fullname" . }}-postgresql:5432/{{ .Values.postgresql.auth.database }}
{{- else -}}
postgres://{{ .Values.externalDatabase.user }}:{{ .Values.externalDatabase.password }}@{{ .Values.externalDatabase.host }}:{{ .Values.externalDatabase.port }}/{{ .Values.externalDatabase.database }}
{{- end -}}
{{- end }}

{{/*
Redis URL.
*/}}
{{- define "rust-conductor.redisUrl" -}}
{{- if .Values.redis.enabled -}}
redis://{{ include "rust-conductor.fullname" . }}-redis-master:6379
{{- else -}}
redis://{{ .Values.externalRedis.host }}:{{ .Values.externalRedis.port }}
{{- end -}}
{{- end }}

{{/*
Kafka brokers.
*/}}
{{- define "rust-conductor.kafkaBrokers" -}}
{{- if .Values.kafka.enabled -}}
{{ include "rust-conductor.fullname" . }}-kafka:9092
{{- else -}}
{{ .Values.externalKafka.brokers }}
{{- end -}}
{{- end }}
