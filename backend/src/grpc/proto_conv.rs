//! Bidirectional conversions between Rust model types and generated proto types.
//!
//! `serde_json::Value` ↔ `prost_types::Struct` is the core bridge: proto Struct
//! fields map directly to dynamic JSON payloads without string serialization.

use prost_types::{value::Kind, ListValue, Struct, Value as ProstValue};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use tonic::Status;

// ─────────────────────────────────────────────────────────────────────────────
// serde_json::Value ↔ prost_types::Value
// ─────────────────────────────────────────────────────────────────────────────

pub fn json_to_prost(v: &JsonValue) -> ProstValue {
    ProstValue {
        kind: Some(json_to_kind(v)),
    }
}

fn json_to_kind(v: &JsonValue) -> Kind {
    match v {
        JsonValue::Null => Kind::NullValue(0),
        JsonValue::Bool(b) => Kind::BoolValue(*b),
        JsonValue::Number(n) => Kind::NumberValue(n.as_f64().unwrap_or(0.0)),
        JsonValue::String(s) => Kind::StringValue(s.clone()),
        JsonValue::Array(arr) => Kind::ListValue(ListValue {
            values: arr.iter().map(json_to_prost).collect(),
        }),
        JsonValue::Object(map) => Kind::StructValue(Struct {
            fields: map
                .iter()
                .map(|(k, v)| (k.clone(), json_to_prost(v)))
                .collect(),
        }),
    }
}

pub fn prost_to_json(v: &ProstValue) -> JsonValue {
    match &v.kind {
        None | Some(Kind::NullValue(_)) => JsonValue::Null,
        Some(Kind::BoolValue(b)) => JsonValue::Bool(*b),
        Some(Kind::NumberValue(n)) => {
            serde_json::Number::from_f64(*n)
                .map(JsonValue::Number)
                .unwrap_or(JsonValue::Null)
        }
        Some(Kind::StringValue(s)) => JsonValue::String(s.clone()),
        Some(Kind::ListValue(list)) => {
            JsonValue::Array(list.values.iter().map(prost_to_json).collect())
        }
        Some(Kind::StructValue(st)) => struct_to_json(st),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// serde_json::Value ↔ prost_types::Struct
// ─────────────────────────────────────────────────────────────────────────────

/// Convert a `serde_json::Value` (expected to be an Object) to a `prost_types::Struct`.
/// Non-object values are wrapped in `{"value": <val>}`.
pub fn json_to_struct(v: &JsonValue) -> Struct {
    match v {
        JsonValue::Object(map) => Struct {
            fields: map
                .iter()
                .map(|(k, v)| (k.clone(), json_to_prost(v)))
                .collect(),
        },
        JsonValue::Null => Struct {
            fields: Default::default(),
        },
        other => Struct {
            fields: [("value".to_string(), json_to_prost(other))]
                .into_iter()
                .collect(),
        },
    }
}

/// Convert a `prost_types::Struct` back to `serde_json::Value` (always an Object).
pub fn struct_to_json(st: &Struct) -> JsonValue {
    let map: serde_json::Map<String, JsonValue> = st
        .fields
        .iter()
        .map(|(k, v)| (k.clone(), prost_to_json(v)))
        .collect();
    JsonValue::Object(map)
}

/// Convert `HashMap<String, serde_json::Value>` to `prost_types::Struct`.
pub fn hashmap_to_struct(m: &HashMap<String, JsonValue>) -> Struct {
    Struct {
        fields: m.iter().map(|(k, v)| (k.clone(), json_to_prost(v))).collect(),
    }
}

/// Convert `prost_types::Struct` to `HashMap<String, serde_json::Value>`.
pub fn struct_to_hashmap(st: &Struct) -> HashMap<String, JsonValue> {
    st.fields
        .iter()
        .map(|(k, v)| (k.clone(), prost_to_json(v)))
        .collect()
}

/// Helper: Option<Struct> → serde_json::Value (Object or Null).
pub fn opt_struct_to_json(st: &Option<Struct>) -> JsonValue {
    match st {
        Some(s) => struct_to_json(s),
        None => JsonValue::Object(Default::default()),
    }
}

/// Helper: Option<Struct> → HashMap<String, Value>.
pub fn opt_struct_to_hashmap(st: &Option<Struct>) -> HashMap<String, JsonValue> {
    match st {
        Some(s) => struct_to_hashmap(s),
        None => HashMap::new(),
    }
}

/// Helper: serde_json::Value → Option<Struct> (None for null/empty).
pub fn json_to_opt_struct(v: &JsonValue) -> Option<Struct> {
    match v {
        JsonValue::Null => None,
        JsonValue::Object(m) if m.is_empty() => None,
        _ => Some(json_to_struct(v)),
    }
}

/// Helper: HashMap → Option<Struct>.
pub fn hashmap_to_opt_struct(m: &HashMap<String, JsonValue>) -> Option<Struct> {
    if m.is_empty() {
        None
    } else {
        Some(hashmap_to_struct(m))
    }
}

/// Helper: map<String,String> for proto
pub fn opt_hashmap_string_to_proto(m: &Option<HashMap<String, String>>) -> HashMap<String, String> {
    m.clone().unwrap_or_default()
}

// ─────────────────────────────────────────────────────────────────────────────
// Owned (move) variants of core converters — avoids cloning on the hot path
// ─────────────────────────────────────────────────────────────────────────────

pub fn json_to_prost_owned(v: JsonValue) -> ProstValue {
    ProstValue { kind: Some(json_to_kind_owned(v)) }
}

fn json_to_kind_owned(v: JsonValue) -> Kind {
    match v {
        JsonValue::Null => Kind::NullValue(0),
        JsonValue::Bool(b) => Kind::BoolValue(b),
        JsonValue::Number(n) => Kind::NumberValue(n.as_f64().unwrap_or(0.0)),
        JsonValue::String(s) => Kind::StringValue(s),
        JsonValue::Array(arr) => Kind::ListValue(ListValue {
            values: arr.into_iter().map(json_to_prost_owned).collect(),
        }),
        JsonValue::Object(map) => Kind::StructValue(Struct {
            fields: map.into_iter().map(|(k, v)| (k, json_to_prost_owned(v))).collect(),
        }),
    }
}

pub fn json_to_struct_owned(v: JsonValue) -> Struct {
    match v {
        JsonValue::Object(map) => Struct {
            fields: map.into_iter().map(|(k, v)| (k, json_to_prost_owned(v))).collect(),
        },
        JsonValue::Null => Struct { fields: Default::default() },
        other => Struct {
            fields: [("value".to_string(), json_to_prost_owned(other))].into_iter().collect(),
        },
    }
}

pub fn json_to_opt_struct_owned(v: JsonValue) -> Option<Struct> {
    match &v {
        JsonValue::Null => None,
        JsonValue::Object(m) if m.is_empty() => None,
        _ => Some(json_to_struct_owned(v)),
    }
}

pub fn hashmap_to_struct_owned(m: HashMap<String, JsonValue>) -> Struct {
    Struct {
        fields: m.into_iter().map(|(k, v)| (k, json_to_prost_owned(v))).collect(),
    }
}

pub fn hashmap_to_opt_struct_owned(m: HashMap<String, JsonValue>) -> Option<Struct> {
    if m.is_empty() { None } else { Some(hashmap_to_struct_owned(m)) }
}

pub fn prost_to_json_owned(v: ProstValue) -> JsonValue {
    match v.kind {
        None | Some(Kind::NullValue(_)) => JsonValue::Null,
        Some(Kind::BoolValue(b)) => JsonValue::Bool(b),
        Some(Kind::NumberValue(n)) => {
            serde_json::Number::from_f64(n).map(JsonValue::Number).unwrap_or(JsonValue::Null)
        }
        Some(Kind::StringValue(s)) => JsonValue::String(s),
        Some(Kind::ListValue(list)) => {
            JsonValue::Array(list.values.into_iter().map(prost_to_json_owned).collect())
        }
        Some(Kind::StructValue(st)) => struct_to_json_owned(st),
    }
}

pub fn struct_to_json_owned(st: Struct) -> JsonValue {
    let map: serde_json::Map<String, JsonValue> =
        st.fields.into_iter().map(|(k, v)| (k, prost_to_json_owned(v))).collect();
    JsonValue::Object(map)
}

pub fn opt_struct_to_json_owned(st: Option<Struct>) -> JsonValue {
    match st {
        Some(s) => struct_to_json_owned(s),
        None => JsonValue::Object(Default::default()),
    }
}

pub fn struct_to_hashmap_owned(st: Struct) -> HashMap<String, JsonValue> {
    st.fields.into_iter().map(|(k, v)| (k, prost_to_json_owned(v))).collect()
}

pub fn opt_struct_to_hashmap_owned(st: Option<Struct>) -> HashMap<String, JsonValue> {
    match st {
        Some(s) => struct_to_hashmap_owned(s),
        None => HashMap::new(),
    }
}

fn opt_string_owned(s: String) -> Option<String> {
    if s.is_empty() { None } else { Some(s) }
}

/// Parse task status from string without serde_json roundtrip.
pub fn parse_task_status(s: &str) -> Result<models::TaskStatus, Status> {
    match s {
        "IN_PROGRESS" => Ok(models::TaskStatus::InProgress),
        "CANCELED" => Ok(models::TaskStatus::Canceled),
        "FAILED" => Ok(models::TaskStatus::Failed),
        "FAILED_WITH_TERMINAL_ERROR" => Ok(models::TaskStatus::FailedWithTerminalError),
        "COMPLETED" => Ok(models::TaskStatus::Completed),
        "COMPLETED_WITH_ERRORS" => Ok(models::TaskStatus::CompletedWithErrors),
        "SCHEDULED" => Ok(models::TaskStatus::Scheduled),
        "TIMED_OUT" => Ok(models::TaskStatus::TimedOut),
        "SKIPPED" => Ok(models::TaskStatus::Skipped),
        _ => Err(Status::invalid_argument(format!("Invalid task status: '{s}'"))),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Custom proto (package: conductor) ↔ Rust model conversions
// ─────────────────────────────────────────────────────────────────────────────

use crate::grpc::pb;
use crate::models;

// ── WorkflowDef ──

pub fn workflow_def_to_proto(d: &models::WorkflowDef) -> pb::WorkflowDefPb {
    pb::WorkflowDefPb {
        name: d.name.clone(),
        description: d.description.clone().unwrap_or_default(),
        version: d.version,
        tasks: d.tasks.iter().map(workflow_task_to_proto).collect(),
        input_parameters: d.input_parameters.clone(),
        output_parameters: hashmap_to_opt_struct(&d.output_parameters),
        failure_workflow: d.failure_workflow.clone().unwrap_or_default(),
        schema_version: d.schema_version,
        restartable: d.restartable,
        workflow_status_listener_enabled: d.workflow_status_listener_enabled,
        owner_email: d.owner_email.clone().unwrap_or_default(),
        timeout_policy: format!("{:?}", d.timeout_policy),
        timeout_seconds: d.timeout_seconds,
        variables: hashmap_to_opt_struct(&d.variables),
        input_template: hashmap_to_opt_struct(&d.input_template),
        owner_app: d.owner_app.clone().unwrap_or_default(),
        create_time: d.create_time.unwrap_or(0),
        update_time: d.update_time.unwrap_or(0),
    }
}

pub fn workflow_def_from_proto(p: &pb::WorkflowDefPb) -> models::WorkflowDef {
    models::WorkflowDef {
        name: p.name.clone(),
        description: opt_string(&p.description),
        version: p.version,
        tasks: p.tasks.iter().map(workflow_task_from_proto).collect(),
        input_parameters: p.input_parameters.clone(),
        output_parameters: opt_struct_to_hashmap(&p.output_parameters),
        failure_workflow: opt_string(&p.failure_workflow),
        schema_version: p.schema_version,
        restartable: p.restartable,
        workflow_status_listener_enabled: p.workflow_status_listener_enabled,
        owner_email: opt_string(&p.owner_email),
        timeout_policy: Default::default(),
        timeout_seconds: p.timeout_seconds,
        variables: opt_struct_to_hashmap(&p.variables),
        input_template: opt_struct_to_hashmap(&p.input_template),
        owner_app: opt_string(&p.owner_app),
        create_time: if p.create_time == 0 { None } else { Some(p.create_time) },
        update_time: if p.update_time == 0 { None } else { Some(p.update_time) },
        created_by: None,
        updated_by: None,
        workflow_status_listener_sink: None,
        rate_limit_config: None,
        input_schema: None,
        output_schema: None,
        enforce_schema: false,
        metadata: None,
        cache_config: None,
        masked_fields: vec![],
        on_complete_webhook: None,
        on_failure_webhook: None,
        sla_deadline_seconds: None,
        tags: vec![],
    }
}

// ── WorkflowTask ──

fn workflow_task_to_proto(t: &models::WorkflowTask) -> pb::WorkflowTaskPb {
    pb::WorkflowTaskPb {
        name: t.name.clone(),
        task_reference_name: t.task_reference_name.clone(),
        r#type: t.task_type.clone(),
        description: t.description.clone().unwrap_or_default(),
        input_parameters: hashmap_to_opt_struct(&t.input_parameters),
        optional: t.optional,
        start_delay: t.start_delay,
        sub_workflow_param: t.sub_workflow_param.as_ref().map(sub_wf_to_proto),
        join_on: t.join_on.clone(),
        fork_tasks: t
            .fork_tasks
            .iter()
            .map(|branch| pb::ForkBranchPb {
                tasks: branch.iter().map(workflow_task_to_proto).collect(),
            })
            .collect(),
        decision_cases: t
            .decision_cases
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    pb::TaskListPb {
                        tasks: v.iter().map(workflow_task_to_proto).collect(),
                    },
                )
            })
            .collect(),
        default_case: t.default_case.iter().map(workflow_task_to_proto).collect(),
        case_expression: t.case_expression.clone().unwrap_or_default(),
        case_value_param: t.case_value_param.clone().unwrap_or_default(),
        sink: t.sink.clone().unwrap_or_default(),
        async_complete: t.async_complete,
        retry_count: t.retry_count.unwrap_or(0),
        evaluator_type: t.evaluator_type.clone().unwrap_or_default(),
        expression: t.expression.clone().unwrap_or_default(),
        script_expression: t.script_expression.clone().unwrap_or_default(),
        dynamic_task_name_param: t.dynamic_task_name_param.clone().unwrap_or_default(),
    }
}

fn workflow_task_from_proto(p: &pb::WorkflowTaskPb) -> models::WorkflowTask {
    models::WorkflowTask {
        name: p.name.clone(),
        task_reference_name: p.task_reference_name.clone(),
        task_type: if p.r#type.is_empty() { "SIMPLE".into() } else { p.r#type.clone() },
        description: opt_string(&p.description),
        input_parameters: opt_struct_to_hashmap(&p.input_parameters),
        optional: p.optional,
        start_delay: p.start_delay,
        sub_workflow_param: p.sub_workflow_param.as_ref().map(sub_wf_from_proto),
        join_on: p.join_on.clone(),
        fork_tasks: p
            .fork_tasks
            .iter()
            .map(|b| b.tasks.iter().map(workflow_task_from_proto).collect())
            .collect(),
        decision_cases: p
            .decision_cases
            .iter()
            .map(|(k, v)| (k.clone(), v.tasks.iter().map(workflow_task_from_proto).collect()))
            .collect(),
        default_case: p.default_case.iter().map(workflow_task_from_proto).collect(),
        case_expression: opt_string(&p.case_expression),
        case_value_param: opt_string(&p.case_value_param),
        loop_condition: None,
        loop_over: vec![],
        retry_count: if p.retry_count == 0 { None } else { Some(p.retry_count) },
        evaluator_type: opt_string(&p.evaluator_type),
        expression: opt_string(&p.expression),
        script_expression: opt_string(&p.script_expression),
        dynamic_task_name_param: opt_string(&p.dynamic_task_name_param),
        dynamic_fork_join_tasks_param: None,
        dynamic_fork_tasks_param: None,
        dynamic_fork_tasks_input_param_name: None,
        sink: opt_string(&p.sink),
        task_definition: None,
        rate_limited: None,
        default_exclusive_join_task: vec![],
        async_complete: p.async_complete,
        on_state_change: None,
        join_status: None,
        cache_config: None,
        permissive: None,
    }
}

fn sub_wf_to_proto(s: &models::SubWorkflowParams) -> pb::SubWorkflowParamsPb {
    pb::SubWorkflowParamsPb {
        name: s.name.clone(),
        version: s.version.unwrap_or(0),
        task_to_domain: s.task_to_domain.clone().unwrap_or_default(),
    }
}

fn sub_wf_from_proto(p: &pb::SubWorkflowParamsPb) -> models::SubWorkflowParams {
    models::SubWorkflowParams {
        name: p.name.clone(),
        version: if p.version == 0 { None } else { Some(p.version) },
        task_to_domain: if p.task_to_domain.is_empty() { None } else { Some(p.task_to_domain.clone()) },
        idempotency_key: None,
        idempotency_strategy: None,
        priority: None,
        workflow_definition: None,
    }
}

// ── TaskDef ──

pub fn task_def_to_proto(d: &models::TaskDef) -> pb::TaskDefPb {
    pb::TaskDefPb {
        name: d.name.clone(),
        description: d.description.clone().unwrap_or_default(),
        retry_count: d.retry_count,
        retry_logic: format!("{:?}", d.retry_logic),
        retry_delay_seconds: d.retry_delay_seconds,
        timeout_seconds: d.timeout_seconds,
        timeout_policy: format!("{:?}", d.timeout_policy),
        response_timeout_seconds: d.response_timeout_seconds,
        concurrent_exec_limit: d.concurrent_exec_limit.unwrap_or(0),
        input_keys: d.input_keys.clone(),
        output_keys: d.output_keys.clone(),
        input_template: hashmap_to_opt_struct(&d.input_template),
        rate_limit_per_frequency: d.rate_limit_per_frequency.unwrap_or(0),
        rate_limit_frequency_in_seconds: d.rate_limit_frequency_in_seconds.unwrap_or(0),
        owner_email: d.owner_email.clone().unwrap_or_default(),
        poll_timeout_seconds: d.poll_timeout_seconds.unwrap_or(0),
        backoff_scale_factor: d.backoff_scale_factor,
        owner_app: d.owner_app.clone().unwrap_or_default(),
        create_time: d.create_time.unwrap_or(0),
        update_time: d.update_time.unwrap_or(0),
    }
}

pub fn task_def_from_proto(p: &pb::TaskDefPb) -> models::TaskDef {
    models::TaskDef {
        name: p.name.clone(),
        description: opt_string(&p.description),
        retry_count: p.retry_count,
        retry_logic: Default::default(),
        retry_delay_seconds: p.retry_delay_seconds,
        timeout_seconds: p.timeout_seconds,
        timeout_policy: Default::default(),
        response_timeout_seconds: p.response_timeout_seconds,
        concurrent_exec_limit: if p.concurrent_exec_limit == 0 { None } else { Some(p.concurrent_exec_limit) },
        input_keys: p.input_keys.clone(),
        output_keys: p.output_keys.clone(),
        input_template: opt_struct_to_hashmap(&p.input_template),
        rate_limit_per_frequency: if p.rate_limit_per_frequency == 0 { None } else { Some(p.rate_limit_per_frequency) },
        rate_limit_frequency_in_seconds: if p.rate_limit_frequency_in_seconds == 0 { None } else { Some(p.rate_limit_frequency_in_seconds) },
        owner_email: opt_string(&p.owner_email),
        poll_timeout_seconds: if p.poll_timeout_seconds == 0 { None } else { Some(p.poll_timeout_seconds) },
        backoff_scale_factor: p.backoff_scale_factor,
        total_timeout_seconds: None,
        owner_app: opt_string(&p.owner_app),
        create_time: if p.create_time == 0 { None } else { Some(p.create_time) },
        update_time: if p.update_time == 0 { None } else { Some(p.update_time) },
        created_by: None,
        updated_by: None,
        base_type: None,
        isolation_group_id: None,
        execution_name_space: None,
        input_schema: None,
        output_schema: None,
        enforce_schema: false,
        retry_on_errors: vec![],
        env_vars: None,
    }
}

// ── StartWorkflowRequest ──

pub fn start_workflow_from_proto(p: pb::StartWorkflowRequest) -> models::StartWorkflowRequest {
    models::StartWorkflowRequest {
        name: p.name,
        version: p.version,
        input: opt_struct_to_json_owned(p.input),
        correlation_id: opt_string_owned(p.correlation_id),
        priority: p.priority,
        task_to_domain: p.task_to_domain,
        workflow_def: None,
        external_input_payload_storage_path: opt_string_owned(p.external_input_payload_storage_path),
        created_by: opt_string_owned(p.created_by),
        idempotency_key: opt_string_owned(p.idempotency_key),
        idempotency_strategy: None,
        tags: vec![],
    }
}

// ── Workflow (runtime) ──

pub fn workflow_to_proto(w: models::Workflow) -> pb::WorkflowPb {
    pb::WorkflowPb {
        workflow_id: w.workflow_id,
        workflow_name: w.workflow_name,
        workflow_version: w.workflow_version,
        status: w.status.to_string(),
        input: json_to_opt_struct_owned(w.input),
        output: json_to_opt_struct_owned(w.output),
        tasks: w.tasks.into_iter().map(task_result_to_proto).collect(),
        correlation_id: w.correlation_id.unwrap_or_default(),
        start_time: w.start_time,
        end_time: w.end_time.unwrap_or(0),
        update_time: w.update_time,
        reason_for_incompletion: w.reason_for_incompletion.unwrap_or_default(),
        priority: w.priority,
        variables: hashmap_to_opt_struct_owned(w.variables),
        parent_workflow_id: w.parent_workflow_id.unwrap_or_default(),
        parent_workflow_task_id: w.parent_workflow_task_id.unwrap_or_default(),
        task_to_domain: w.task_to_domain,
        failed_reference_task_names: w.failed_reference_task_names,
        created_by: w.created_by.unwrap_or_default(),
    }
}

// ── TaskResult (runtime) ──

pub fn task_result_to_proto(t: models::TaskResult) -> pb::TaskResultPb {
    pb::TaskResultPb {
        task_id: t.task_id,
        workflow_instance_id: t.workflow_instance_id,
        task_type: t.task_type,
        task_def_name: t.task_def_name,
        reference_task_name: t.reference_task_name,
        status: t.status.to_string(),
        input_data: json_to_opt_struct_owned(t.input_data),
        output_data: json_to_opt_struct_owned(t.output_data),
        reason_for_incompletion: t.reason_for_incompletion.unwrap_or_default(),
        scheduled_time: t.scheduled_time.unwrap_or(0),
        start_time: t.start_time.unwrap_or(0),
        end_time: t.end_time.unwrap_or(0),
        update_time: t.update_time.unwrap_or(0),
        poll_count: t.poll_count,
        worker_id: t.worker_id.unwrap_or_default(),
        seq: t.seq,
        retry_count: t.retry_count,
        callback_after_seconds: t.callback_after_seconds,
        sub_workflow_id: t.sub_workflow_id.unwrap_or_default(),
    }
}

// ── TaskUpdateRequest ──

pub fn task_update_from_proto(p: pb::UpdateTaskRequest) -> Result<models::TaskUpdateRequest, Status> {
    let status = parse_task_status(&p.status)?;
    Ok(models::TaskUpdateRequest {
        task_id: p.task_id,
        workflow_instance_id: p.workflow_instance_id,
        status,
        output_data: opt_struct_to_json_owned(p.output_data),
        reason_for_incompletion: opt_string_owned(p.reason_for_incompletion),
        callback_after_seconds: p.callback_after_seconds,
        worker_id: opt_string_owned(p.worker_id),
        logs: vec![],
        external_output_payload_storage_path: None,
        sub_workflow_id: None,
        extend_lease: false,
    })
}

// ── PollTask ──

pub fn poll_task_to_proto(t: models::PollTask) -> pb::PollTaskPb {
    pb::PollTaskPb {
        task_id: t.task_id,
        workflow_instance_id: t.workflow_instance_id,
        task_type: t.task_type,
        task_def_name: t.task_def_name,
        reference_task_name: t.reference_task_name,
        status: t.status.to_string(),
        input_data: json_to_opt_struct_owned(t.input_data),
        scheduled_time: t.scheduled_time.unwrap_or(0),
        start_time: t.start_time.unwrap_or(0),
        callback_after_seconds: t.callback_after_seconds,
        poll_count: t.poll_count,
        retry_count: t.retry_count,
    }
}

// ── RerunWorkflowRequest ──

pub fn rerun_from_proto(p: &pb::RerunWorkflowRequest) -> models::RerunWorkflowRequest {
    models::RerunWorkflowRequest {
        re_run_from_workflow_id: opt_string(&p.re_run_from_workflow_id),
        workflow_input: {
            let m = opt_struct_to_hashmap(&p.workflow_input);
            if m.is_empty() { None } else { Some(m) }
        },
        re_run_from_task_id: opt_string(&p.re_run_from_task_id),
        task_input: {
            let m = opt_struct_to_hashmap(&p.task_input);
            if m.is_empty() { None } else { Some(m) }
        },
        correlation_id: opt_string(&p.correlation_id),
    }
}

// ── SkipTaskRequest ──

pub fn skip_task_from_proto(p: &pb::SkipTaskRequest) -> models::SkipTaskRequest {
    models::SkipTaskRequest {
        task_input: {
            let m = opt_struct_to_hashmap(&p.task_input);
            if m.is_empty() { None } else { Some(m) }
        },
        task_output: {
            let m = opt_struct_to_hashmap(&p.task_output);
            if m.is_empty() { None } else { Some(m) }
        },
    }
}

// ── WorkflowSummary ──

pub fn workflow_summary_to_proto(s: &models::WorkflowSummary) -> pb::WorkflowSummaryPb {
    pb::WorkflowSummaryPb {
        workflow_id: s.workflow_id.clone(),
        workflow_type: s.workflow_type.clone(),
        version: s.version,
        status: s.status.to_string(),
        start_time: s.start_time.as_ref().and_then(|t| t.parse::<i64>().ok()).unwrap_or(0),
        update_time: s.update_time.as_ref().and_then(|t| t.parse::<i64>().ok()).unwrap_or(0),
        end_time: s.end_time.as_ref().and_then(|t| t.parse::<i64>().ok()).unwrap_or(0),
        correlation_id: s.correlation_id.clone().unwrap_or_default(),
        reason_for_incompletion: s.reason_for_incompletion.clone().unwrap_or_default(),
        priority: s.priority,
        execution_time: s.execution_time.unwrap_or(0),
    }
}

// ── TaskSummary ──

pub fn task_summary_to_proto(s: &models::TaskSummary) -> pb::TaskSummaryPb {
    pb::TaskSummaryPb {
        task_id: s.task_id.clone().unwrap_or_default(),
        workflow_instance_id: s.workflow_id.clone().unwrap_or_default(),
        task_type: s.task_type.clone().unwrap_or_default(),
        task_def_name: s.task_def_name.clone().unwrap_or_default(),
        reference_task_name: String::new(),
        status: s.status.as_ref().map(|st| st.to_string()).unwrap_or_default(),
        scheduled_time: s.scheduled_time.as_ref().and_then(|t| t.parse::<i64>().ok()).unwrap_or(0),
        start_time: s.start_time.as_ref().and_then(|t| t.parse::<i64>().ok()).unwrap_or(0),
        end_time: s.end_time.as_ref().and_then(|t| t.parse::<i64>().ok()).unwrap_or(0),
        update_time: s.update_time.as_ref().and_then(|t| t.parse::<i64>().ok()).unwrap_or(0),
    }
}

// ── EventHandler ──

pub fn event_handler_to_proto(h: &models::EventHandler) -> pb::EventHandlerPb {
    pb::EventHandlerPb {
        name: h.name.clone(),
        event: h.event.clone(),
        condition: h.condition.clone().unwrap_or_default(),
        actions: h.actions.iter().map(event_action_to_proto).collect(),
        active: h.active,
        evaluator_type: h.evaluator_type.clone().unwrap_or_default(),
    }
}

pub fn event_handler_from_proto(p: &pb::EventHandlerPb) -> models::EventHandler {
    models::EventHandler {
        name: p.name.clone(),
        event: p.event.clone(),
        condition: opt_string(&p.condition),
        actions: p.actions.iter().map(event_action_from_proto).collect(),
        active: p.active,
        evaluator_type: opt_string(&p.evaluator_type),
    }
}

fn event_action_to_proto(a: &models::EventAction) -> pb::EventActionPb {
    pb::EventActionPb {
        action: a.action.as_ref().map(|at| format!("{:?}", at)).unwrap_or_default(),
        start_workflow: a
            .start_workflow
            .as_ref()
            .map(|sw| {
                let j = serde_json::to_value(sw).unwrap_or(JsonValue::Null);
                json_to_struct(&j)
            }),
        complete_task: a
            .complete_task
            .as_ref()
            .map(|ct| {
                let j = serde_json::to_value(ct).unwrap_or(JsonValue::Null);
                json_to_struct(&j)
            }),
        fail_task: a
            .fail_task
            .as_ref()
            .map(|ft| {
                let j = serde_json::to_value(ft).unwrap_or(JsonValue::Null);
                json_to_struct(&j)
            }),
        terminate_workflow: a
            .terminate_workflow
            .as_ref()
            .map(|tw| {
                let j = serde_json::to_value(tw).unwrap_or(JsonValue::Null);
                json_to_struct(&j)
            }),
        update_workflow_variables: a
            .update_workflow_variables
            .as_ref()
            .map(|uw| {
                let j = serde_json::to_value(uw).unwrap_or(JsonValue::Null);
                json_to_struct(&j)
            }),
    }
}

fn event_action_from_proto(p: &pb::EventActionPb) -> models::EventAction {
    models::EventAction {
        action: None,
        start_workflow: p
            .start_workflow
            .as_ref()
            .and_then(|s| serde_json::from_value(struct_to_json(s)).ok()),
        complete_task: p
            .complete_task
            .as_ref()
            .and_then(|s| serde_json::from_value(struct_to_json(s)).ok()),
        fail_task: p
            .fail_task
            .as_ref()
            .and_then(|s| serde_json::from_value(struct_to_json(s)).ok()),
        expand_inline_json: None,
        terminate_workflow: p
            .terminate_workflow
            .as_ref()
            .and_then(|s| serde_json::from_value(struct_to_json(s)).ok()),
        update_workflow_variables: p
            .update_workflow_variables
            .as_ref()
            .and_then(|s| serde_json::from_value(struct_to_json(s)).ok()),
    }
}

// ── HealthCheck ──

pub fn health_to_proto(h: &models::HealthCheckStatus) -> pb::HealthCheckResponse {
    let details_json = serde_json::to_value(h).unwrap_or(JsonValue::Null);
    pb::HealthCheckResponse {
        healthy: h.healthy,
        details: Some(json_to_struct(&details_json)),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Official proto (package: conductor.grpc) ↔ Rust model conversions
// ─────────────────────────────────────────────────────────────────────────────

use crate::grpc::official::pb as opb;

// ── WorkflowDef (official) ──

pub fn workflow_def_to_official(d: &models::WorkflowDef) -> opb::OfWorkflowDefPb {
    opb::OfWorkflowDefPb {
        name: d.name.clone(),
        description: d.description.clone().unwrap_or_default(),
        version: d.version,
        tasks: d.tasks.iter().map(workflow_task_to_official).collect(),
        input_parameters: d.input_parameters.clone(),
        output_parameters: hashmap_to_opt_struct(&d.output_parameters),
        failure_workflow: d.failure_workflow.clone().unwrap_or_default(),
        schema_version: d.schema_version,
        restartable: d.restartable,
        workflow_status_listener_enabled: d.workflow_status_listener_enabled,
        owner_email: d.owner_email.clone().unwrap_or_default(),
        timeout_policy: format!("{:?}", d.timeout_policy),
        timeout_seconds: d.timeout_seconds,
        variables: hashmap_to_opt_struct(&d.variables),
        input_template: hashmap_to_opt_struct(&d.input_template),
        owner_app: d.owner_app.clone().unwrap_or_default(),
        create_time: d.create_time.unwrap_or(0),
        update_time: d.update_time.unwrap_or(0),
    }
}

pub fn workflow_def_from_official(p: &opb::OfWorkflowDefPb) -> models::WorkflowDef {
    models::WorkflowDef {
        name: p.name.clone(),
        description: opt_string(&p.description),
        version: p.version,
        tasks: p.tasks.iter().map(workflow_task_from_official).collect(),
        input_parameters: p.input_parameters.clone(),
        output_parameters: opt_struct_to_hashmap(&p.output_parameters),
        failure_workflow: opt_string(&p.failure_workflow),
        schema_version: p.schema_version,
        restartable: p.restartable,
        workflow_status_listener_enabled: p.workflow_status_listener_enabled,
        owner_email: opt_string(&p.owner_email),
        timeout_policy: Default::default(),
        timeout_seconds: p.timeout_seconds,
        variables: opt_struct_to_hashmap(&p.variables),
        input_template: opt_struct_to_hashmap(&p.input_template),
        owner_app: opt_string(&p.owner_app),
        create_time: if p.create_time == 0 { None } else { Some(p.create_time) },
        update_time: if p.update_time == 0 { None } else { Some(p.update_time) },
        created_by: None,
        updated_by: None,
        workflow_status_listener_sink: None,
        rate_limit_config: None,
        input_schema: None,
        output_schema: None,
        enforce_schema: false,
        metadata: None,
        cache_config: None,
        masked_fields: vec![],
        on_complete_webhook: None,
        on_failure_webhook: None,
        sla_deadline_seconds: None,
        tags: vec![],
    }
}

fn workflow_task_to_official(t: &models::WorkflowTask) -> opb::OfWorkflowTaskPb {
    opb::OfWorkflowTaskPb {
        name: t.name.clone(),
        task_reference_name: t.task_reference_name.clone(),
        r#type: t.task_type.clone(),
        description: t.description.clone().unwrap_or_default(),
        input_parameters: hashmap_to_opt_struct(&t.input_parameters),
        optional: t.optional,
        start_delay: t.start_delay,
        sub_workflow_param: t.sub_workflow_param.as_ref().map(|s| opb::OfSubWorkflowParamsPb {
            name: s.name.clone(),
            version: s.version.unwrap_or(0),
            task_to_domain: s.task_to_domain.clone().unwrap_or_default(),
        }),
        join_on: t.join_on.clone(),
        fork_tasks: t
            .fork_tasks
            .iter()
            .map(|branch| opb::OfForkBranchPb {
                tasks: branch.iter().map(workflow_task_to_official).collect(),
            })
            .collect(),
        decision_cases: t
            .decision_cases
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    opb::OfTaskListPb {
                        tasks: v.iter().map(workflow_task_to_official).collect(),
                    },
                )
            })
            .collect(),
        default_case: t.default_case.iter().map(workflow_task_to_official).collect(),
        case_expression: t.case_expression.clone().unwrap_or_default(),
        case_value_param: t.case_value_param.clone().unwrap_or_default(),
        sink: t.sink.clone().unwrap_or_default(),
        async_complete: t.async_complete,
        retry_count: t.retry_count.unwrap_or(0),
        evaluator_type: t.evaluator_type.clone().unwrap_or_default(),
        expression: t.expression.clone().unwrap_or_default(),
        script_expression: t.script_expression.clone().unwrap_or_default(),
        dynamic_task_name_param: t.dynamic_task_name_param.clone().unwrap_or_default(),
    }
}

fn workflow_task_from_official(p: &opb::OfWorkflowTaskPb) -> models::WorkflowTask {
    models::WorkflowTask {
        name: p.name.clone(),
        task_reference_name: p.task_reference_name.clone(),
        task_type: if p.r#type.is_empty() { "SIMPLE".into() } else { p.r#type.clone() },
        description: opt_string(&p.description),
        input_parameters: opt_struct_to_hashmap(&p.input_parameters),
        optional: p.optional,
        start_delay: p.start_delay,
        sub_workflow_param: p.sub_workflow_param.as_ref().map(|s| models::SubWorkflowParams {
            name: s.name.clone(),
            version: if s.version == 0 { None } else { Some(s.version) },
            task_to_domain: if s.task_to_domain.is_empty() { None } else { Some(s.task_to_domain.clone()) },
            idempotency_key: None,
            idempotency_strategy: None,
            priority: None,
            workflow_definition: None,
        }),
        join_on: p.join_on.clone(),
        fork_tasks: p
            .fork_tasks
            .iter()
            .map(|b| b.tasks.iter().map(workflow_task_from_official).collect())
            .collect(),
        decision_cases: p
            .decision_cases
            .iter()
            .map(|(k, v)| (k.clone(), v.tasks.iter().map(workflow_task_from_official).collect()))
            .collect(),
        default_case: p.default_case.iter().map(workflow_task_from_official).collect(),
        case_expression: opt_string(&p.case_expression),
        case_value_param: opt_string(&p.case_value_param),
        loop_condition: None,
        loop_over: vec![],
        retry_count: if p.retry_count == 0 { None } else { Some(p.retry_count) },
        evaluator_type: opt_string(&p.evaluator_type),
        expression: opt_string(&p.expression),
        script_expression: opt_string(&p.script_expression),
        dynamic_task_name_param: opt_string(&p.dynamic_task_name_param),
        dynamic_fork_join_tasks_param: None,
        dynamic_fork_tasks_param: None,
        dynamic_fork_tasks_input_param_name: None,
        sink: opt_string(&p.sink),
        task_definition: None,
        rate_limited: None,
        default_exclusive_join_task: vec![],
        async_complete: p.async_complete,
        on_state_change: None,
        join_status: None,
        cache_config: None,
        permissive: None,
    }
}

// ── TaskDef (official) ──

pub fn task_def_to_official(d: &models::TaskDef) -> opb::OfTaskDefPb {
    opb::OfTaskDefPb {
        name: d.name.clone(),
        description: d.description.clone().unwrap_or_default(),
        retry_count: d.retry_count,
        retry_logic: format!("{:?}", d.retry_logic),
        retry_delay_seconds: d.retry_delay_seconds,
        timeout_seconds: d.timeout_seconds,
        timeout_policy: format!("{:?}", d.timeout_policy),
        response_timeout_seconds: d.response_timeout_seconds,
        concurrent_exec_limit: d.concurrent_exec_limit.unwrap_or(0),
        input_keys: d.input_keys.clone(),
        output_keys: d.output_keys.clone(),
        input_template: hashmap_to_opt_struct(&d.input_template),
        rate_limit_per_frequency: d.rate_limit_per_frequency.unwrap_or(0),
        rate_limit_frequency_in_seconds: d.rate_limit_frequency_in_seconds.unwrap_or(0),
        owner_email: d.owner_email.clone().unwrap_or_default(),
        poll_timeout_seconds: d.poll_timeout_seconds.unwrap_or(0),
        backoff_scale_factor: d.backoff_scale_factor,
        owner_app: d.owner_app.clone().unwrap_or_default(),
        create_time: d.create_time.unwrap_or(0),
        update_time: d.update_time.unwrap_or(0),
    }
}

pub fn task_def_from_official(p: &opb::OfTaskDefPb) -> models::TaskDef {
    models::TaskDef {
        name: p.name.clone(),
        description: opt_string(&p.description),
        retry_count: p.retry_count,
        retry_logic: Default::default(),
        retry_delay_seconds: p.retry_delay_seconds,
        timeout_seconds: p.timeout_seconds,
        timeout_policy: Default::default(),
        response_timeout_seconds: p.response_timeout_seconds,
        concurrent_exec_limit: if p.concurrent_exec_limit == 0 { None } else { Some(p.concurrent_exec_limit) },
        input_keys: p.input_keys.clone(),
        output_keys: p.output_keys.clone(),
        input_template: opt_struct_to_hashmap(&p.input_template),
        rate_limit_per_frequency: if p.rate_limit_per_frequency == 0 { None } else { Some(p.rate_limit_per_frequency) },
        rate_limit_frequency_in_seconds: if p.rate_limit_frequency_in_seconds == 0 { None } else { Some(p.rate_limit_frequency_in_seconds) },
        owner_email: opt_string(&p.owner_email),
        poll_timeout_seconds: if p.poll_timeout_seconds == 0 { None } else { Some(p.poll_timeout_seconds) },
        backoff_scale_factor: p.backoff_scale_factor,
        total_timeout_seconds: None,
        owner_app: opt_string(&p.owner_app),
        create_time: if p.create_time == 0 { None } else { Some(p.create_time) },
        update_time: if p.update_time == 0 { None } else { Some(p.update_time) },
        created_by: None,
        updated_by: None,
        base_type: None,
        isolation_group_id: None,
        execution_name_space: None,
        input_schema: None,
        output_schema: None,
        enforce_schema: false,
        retry_on_errors: vec![],
        env_vars: None,
    }
}

// ── Official StartWorkflow ──

pub fn start_workflow_from_official(p: opb::OfStartWorkflowRequest) -> models::StartWorkflowRequest {
    models::StartWorkflowRequest {
        name: p.name,
        version: p.version,
        input: opt_struct_to_json_owned(p.input),
        correlation_id: opt_string_owned(p.correlation_id),
        priority: p.priority,
        task_to_domain: p.task_to_domain,
        workflow_def: None,
        external_input_payload_storage_path: opt_string_owned(p.external_input_payload_storage_path),
        created_by: opt_string_owned(p.created_by),
        idempotency_key: opt_string_owned(p.idempotency_key),
        idempotency_strategy: None,
        tags: vec![],
    }
}

// ── Official Workflow runtime ──

pub fn workflow_to_official(w: models::Workflow) -> opb::OfWorkflowPb {
    opb::OfWorkflowPb {
        workflow_id: w.workflow_id,
        workflow_name: w.workflow_name,
        workflow_version: w.workflow_version,
        status: w.status.to_string(),
        input: json_to_opt_struct_owned(w.input),
        output: json_to_opt_struct_owned(w.output),
        tasks: w.tasks.into_iter().map(task_result_to_official).collect(),
        correlation_id: w.correlation_id.unwrap_or_default(),
        start_time: w.start_time,
        end_time: w.end_time.unwrap_or(0),
        update_time: w.update_time,
        reason_for_incompletion: w.reason_for_incompletion.unwrap_or_default(),
        priority: w.priority,
        variables: hashmap_to_opt_struct_owned(w.variables),
        parent_workflow_id: w.parent_workflow_id.unwrap_or_default(),
        parent_workflow_task_id: w.parent_workflow_task_id.unwrap_or_default(),
        task_to_domain: w.task_to_domain,
        failed_reference_task_names: w.failed_reference_task_names,
        created_by: w.created_by.unwrap_or_default(),
    }
}

pub fn task_result_to_official(t: models::TaskResult) -> opb::OfTaskResultPb {
    opb::OfTaskResultPb {
        task_id: t.task_id,
        workflow_instance_id: t.workflow_instance_id,
        task_type: t.task_type,
        task_def_name: t.task_def_name,
        reference_task_name: t.reference_task_name,
        status: t.status.to_string(),
        input_data: json_to_opt_struct_owned(t.input_data),
        output_data: json_to_opt_struct_owned(t.output_data),
        reason_for_incompletion: t.reason_for_incompletion.unwrap_or_default(),
        scheduled_time: t.scheduled_time.unwrap_or(0),
        start_time: t.start_time.unwrap_or(0),
        end_time: t.end_time.unwrap_or(0),
        update_time: t.update_time.unwrap_or(0),
        poll_count: t.poll_count,
        worker_id: t.worker_id.unwrap_or_default(),
        seq: t.seq,
        retry_count: t.retry_count,
        callback_after_seconds: t.callback_after_seconds,
        sub_workflow_id: t.sub_workflow_id.unwrap_or_default(),
    }
}

// ── Official PollTask ──

pub fn poll_task_to_official(t: models::PollTask) -> opb::OfPollTaskPb {
    opb::OfPollTaskPb {
        task_id: t.task_id,
        workflow_instance_id: t.workflow_instance_id,
        task_type: t.task_type,
        task_def_name: t.task_def_name,
        reference_task_name: t.reference_task_name,
        status: t.status.to_string(),
        input_data: json_to_opt_struct_owned(t.input_data),
        scheduled_time: t.scheduled_time.unwrap_or(0),
        start_time: t.start_time.unwrap_or(0),
        callback_after_seconds: t.callback_after_seconds,
        poll_count: t.poll_count,
        retry_count: t.retry_count,
    }
}

// ── Official TaskUpdate ──

pub fn task_update_from_official(p: opb::OfUpdateTaskRequest) -> Result<models::TaskUpdateRequest, Status> {
    let status = parse_task_status(&p.status)?;
    Ok(models::TaskUpdateRequest {
        task_id: p.task_id,
        workflow_instance_id: p.workflow_instance_id,
        status,
        output_data: opt_struct_to_json_owned(p.output_data),
        reason_for_incompletion: opt_string_owned(p.reason_for_incompletion),
        callback_after_seconds: p.callback_after_seconds,
        worker_id: opt_string_owned(p.worker_id),
        logs: vec![],
        external_output_payload_storage_path: None,
        sub_workflow_id: None,
        extend_lease: false,
    })
}

// ── Official Rerun ──

pub fn rerun_from_official(p: &opb::OfRerunWorkflowRequest) -> models::RerunWorkflowRequest {
    models::RerunWorkflowRequest {
        re_run_from_workflow_id: opt_string(&p.re_run_from_workflow_id),
        workflow_input: {
            let m = opt_struct_to_hashmap(&p.workflow_input);
            if m.is_empty() { None } else { Some(m) }
        },
        re_run_from_task_id: opt_string(&p.re_run_from_task_id),
        task_input: {
            let m = opt_struct_to_hashmap(&p.task_input);
            if m.is_empty() { None } else { Some(m) }
        },
        correlation_id: opt_string(&p.correlation_id),
    }
}

// ── Official SkipTask ──

pub fn skip_task_from_official(p: &opb::OfSkipTaskRequest) -> models::SkipTaskRequest {
    models::SkipTaskRequest {
        task_input: {
            let m = opt_struct_to_hashmap(&p.task_input);
            if m.is_empty() { None } else { Some(m) }
        },
        task_output: {
            let m = opt_struct_to_hashmap(&p.task_output);
            if m.is_empty() { None } else { Some(m) }
        },
    }
}

// ── Official WorkflowSummary ──

pub fn workflow_summary_to_official(s: &models::WorkflowSummary) -> opb::OfWorkflowSummaryPb {
    opb::OfWorkflowSummaryPb {
        workflow_id: s.workflow_id.clone(),
        workflow_type: s.workflow_type.clone(),
        version: s.version,
        status: s.status.to_string(),
        start_time: s.start_time.as_ref().and_then(|t| t.parse::<i64>().ok()).unwrap_or(0),
        update_time: s.update_time.as_ref().and_then(|t| t.parse::<i64>().ok()).unwrap_or(0),
        end_time: s.end_time.as_ref().and_then(|t| t.parse::<i64>().ok()).unwrap_or(0),
        correlation_id: s.correlation_id.clone().unwrap_or_default(),
        reason_for_incompletion: s.reason_for_incompletion.clone().unwrap_or_default(),
        priority: s.priority,
        execution_time: s.execution_time.unwrap_or(0),
    }
}

// ── Official TaskSummary ──

pub fn task_summary_to_official(s: &models::TaskSummary) -> opb::OfTaskSummaryPb {
    opb::OfTaskSummaryPb {
        task_id: s.task_id.clone().unwrap_or_default(),
        workflow_instance_id: s.workflow_id.clone().unwrap_or_default(),
        task_type: s.task_type.clone().unwrap_or_default(),
        task_def_name: s.task_def_name.clone().unwrap_or_default(),
        reference_task_name: String::new(),
        status: s.status.as_ref().map(|st| st.to_string()).unwrap_or_default(),
        scheduled_time: s.scheduled_time.as_ref().and_then(|t| t.parse::<i64>().ok()).unwrap_or(0),
        start_time: s.start_time.as_ref().and_then(|t| t.parse::<i64>().ok()).unwrap_or(0),
        end_time: s.end_time.as_ref().and_then(|t| t.parse::<i64>().ok()).unwrap_or(0),
        update_time: s.update_time.as_ref().and_then(|t| t.parse::<i64>().ok()).unwrap_or(0),
    }
}

// ── Official EventHandler ──

pub fn event_handler_to_official(h: &models::EventHandler) -> opb::OfEventHandlerPb {
    opb::OfEventHandlerPb {
        name: h.name.clone(),
        event: h.event.clone(),
        condition: h.condition.clone().unwrap_or_default(),
        actions: h.actions.iter().map(|a| {
            opb::OfEventActionPb {
                action: a.action.as_ref().map(|at| format!("{:?}", at)).unwrap_or_default(),
                start_workflow: a.start_workflow.as_ref().map(|sw| {
                    let j = serde_json::to_value(sw).unwrap_or(JsonValue::Null);
                    json_to_struct(&j)
                }),
                complete_task: a.complete_task.as_ref().map(|ct| {
                    let j = serde_json::to_value(ct).unwrap_or(JsonValue::Null);
                    json_to_struct(&j)
                }),
                fail_task: a.fail_task.as_ref().map(|ft| {
                    let j = serde_json::to_value(ft).unwrap_or(JsonValue::Null);
                    json_to_struct(&j)
                }),
                terminate_workflow: a.terminate_workflow.as_ref().map(|tw| {
                    let j = serde_json::to_value(tw).unwrap_or(JsonValue::Null);
                    json_to_struct(&j)
                }),
                update_workflow_variables: a.update_workflow_variables.as_ref().map(|uw| {
                    let j = serde_json::to_value(uw).unwrap_or(JsonValue::Null);
                    json_to_struct(&j)
                }),
            }
        }).collect(),
        active: h.active,
        evaluator_type: h.evaluator_type.clone().unwrap_or_default(),
    }
}

pub fn event_handler_from_official(p: &opb::OfEventHandlerPb) -> models::EventHandler {
    models::EventHandler {
        name: p.name.clone(),
        event: p.event.clone(),
        condition: opt_string(&p.condition),
        actions: p.actions.iter().map(|a| {
            models::EventAction {
                action: None,
                start_workflow: a.start_workflow.as_ref().and_then(|s| serde_json::from_value(struct_to_json(s)).ok()),
                complete_task: a.complete_task.as_ref().and_then(|s| serde_json::from_value(struct_to_json(s)).ok()),
                fail_task: a.fail_task.as_ref().and_then(|s| serde_json::from_value(struct_to_json(s)).ok()),
                expand_inline_json: None,
                terminate_workflow: a.terminate_workflow.as_ref().and_then(|s| serde_json::from_value(struct_to_json(s)).ok()),
                update_workflow_variables: a.update_workflow_variables.as_ref().and_then(|s| serde_json::from_value(struct_to_json(s)).ok()),
            }
        }).collect(),
        active: p.active,
        evaluator_type: opt_string(&p.evaluator_type),
    }
}

// ── Health (official) ──

pub fn health_to_official(_h: &models::HealthCheckStatus) -> opb::OfWorkflowPb {
    // Not needed — official proto doesn't have a Health service
    // Placeholder to keep symmetry
    unreachable!()
}

// ─────────────────────────────────────────────────────────────────────────────
// Utility
// ─────────────────────────────────────────────────────────────────────────────

fn opt_string(s: &str) -> Option<String> {
    if s.is_empty() { None } else { Some(s.to_string()) }
}
