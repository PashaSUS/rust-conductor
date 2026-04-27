//! Official proto (package: conductor.grpc) ↔ Rust model conversions.
//!
//! Mirrors the custom-proto converters but targets the `conductor.grpc.*`
//! message types generated from `conductor_official.proto`.

use serde_json::Value as JsonValue;
use tonic::Status;

use super::proto_conv::{
    hashmap_to_opt_struct, hashmap_to_opt_struct_owned, json_to_opt_struct_owned,
    json_to_struct, opt_struct_to_hashmap, opt_struct_to_json_owned, opt_string,
    opt_string_owned, parse_task_status, struct_to_json,
};
use crate::grpc::official::pb as opb;
use crate::models;

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
        input_parameter_definitions: Vec::new(),
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
        create_time: if p.create_time == 0 {
            None
        } else {
            Some(p.create_time)
        },
        update_time: if p.update_time == 0 {
            None
        } else {
            Some(p.update_time)
        },
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
        saga_enabled: false,
        base_workflow: None,
        base_workflow_version: None,
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
        sub_workflow_param: t
            .sub_workflow_param
            .as_ref()
            .map(|s| opb::OfSubWorkflowParamsPb {
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
        default_case: t
            .default_case
            .iter()
            .map(workflow_task_to_official)
            .collect(),
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
        task_type: if p.r#type.is_empty() {
            "SIMPLE".into()
        } else {
            p.r#type.clone()
        },
        description: opt_string(&p.description),
        input_parameters: opt_struct_to_hashmap(&p.input_parameters),
        optional: p.optional,
        start_delay: p.start_delay,
        sub_workflow_param: p
            .sub_workflow_param
            .as_ref()
            .map(|s| models::SubWorkflowParams {
                name: s.name.clone(),
                version: if s.version == 0 {
                    None
                } else {
                    Some(s.version)
                },
                task_to_domain: if s.task_to_domain.is_empty() {
                    None
                } else {
                    Some(s.task_to_domain.clone())
                },
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
            .map(|(k, v)| {
                (
                    k.clone(),
                    v.tasks.iter().map(workflow_task_from_official).collect(),
                )
            })
            .collect(),
        default_case: p
            .default_case
            .iter()
            .map(workflow_task_from_official)
            .collect(),
        case_expression: opt_string(&p.case_expression),
        case_value_param: opt_string(&p.case_value_param),
        loop_condition: None,
        loop_over: vec![],
        retry_count: if p.retry_count == 0 {
            None
        } else {
            Some(p.retry_count)
        },
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
        ..Default::default()
    }
}

// â”€â”€ TaskDef (official) â”€â”€

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
        concurrent_exec_limit: if p.concurrent_exec_limit == 0 {
            None
        } else {
            Some(p.concurrent_exec_limit)
        },
        input_keys: p.input_keys.clone(),
        output_keys: p.output_keys.clone(),
        input_template: opt_struct_to_hashmap(&p.input_template),
        rate_limit_per_frequency: if p.rate_limit_per_frequency == 0 {
            None
        } else {
            Some(p.rate_limit_per_frequency)
        },
        rate_limit_frequency_in_seconds: if p.rate_limit_frequency_in_seconds == 0 {
            None
        } else {
            Some(p.rate_limit_frequency_in_seconds)
        },
        owner_email: opt_string(&p.owner_email),
        poll_timeout_seconds: if p.poll_timeout_seconds == 0 {
            None
        } else {
            Some(p.poll_timeout_seconds)
        },
        backoff_scale_factor: p.backoff_scale_factor,
        total_timeout_seconds: None,
        owner_app: opt_string(&p.owner_app),
        create_time: if p.create_time == 0 {
            None
        } else {
            Some(p.create_time)
        },
        update_time: if p.update_time == 0 {
            None
        } else {
            Some(p.update_time)
        },
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

// â”€â”€ Official StartWorkflow â”€â”€

pub fn start_workflow_from_official(
    p: opb::OfStartWorkflowRequest,
) -> models::StartWorkflowRequest {
    models::StartWorkflowRequest {
        name: p.name,
        version: p.version,
        input: opt_struct_to_json_owned(p.input),
        correlation_id: opt_string_owned(p.correlation_id),
        priority: p.priority,
        task_to_domain: p.task_to_domain,
        workflow_def: None,
        external_input_payload_storage_path: opt_string_owned(
            p.external_input_payload_storage_path,
        ),
        created_by: opt_string_owned(p.created_by),
        idempotency_key: opt_string_owned(p.idempotency_key),
        idempotency_strategy: None,
        tags: vec![],
        parent_workflow_id: None,
        parent_workflow_task_id: None,
    }
}

// â”€â”€ Official Workflow runtime â”€â”€

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

// â”€â”€ Official PollTask â”€â”€

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

// â”€â”€ Official TaskUpdate â”€â”€

#[allow(clippy::result_large_err)]
pub fn task_update_from_official(
    p: opb::OfUpdateTaskRequest,
) -> Result<models::TaskUpdateRequest, Status> {
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

// â”€â”€ Official Rerun â”€â”€

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

// â”€â”€ Official SkipTask â”€â”€

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

// â”€â”€ Official WorkflowSummary â”€â”€

pub fn workflow_summary_to_official(s: &models::WorkflowSummary) -> opb::OfWorkflowSummaryPb {
    opb::OfWorkflowSummaryPb {
        workflow_id: s.workflow_id.clone(),
        workflow_type: s.workflow_type.clone(),
        version: s.version,
        status: s.status.to_string(),
        start_time: s
            .start_time
            .as_ref()
            .and_then(|t| t.parse::<i64>().ok())
            .unwrap_or(0),
        update_time: s
            .update_time
            .as_ref()
            .and_then(|t| t.parse::<i64>().ok())
            .unwrap_or(0),
        end_time: s
            .end_time
            .as_ref()
            .and_then(|t| t.parse::<i64>().ok())
            .unwrap_or(0),
        correlation_id: s.correlation_id.clone().unwrap_or_default(),
        reason_for_incompletion: s.reason_for_incompletion.clone().unwrap_or_default(),
        priority: s.priority,
        execution_time: s.execution_time.unwrap_or(0),
    }
}

// â”€â”€ Official TaskSummary â”€â”€

pub fn task_summary_to_official(s: &models::TaskSummary) -> opb::OfTaskSummaryPb {
    opb::OfTaskSummaryPb {
        task_id: s.task_id.clone().unwrap_or_default(),
        workflow_instance_id: s.workflow_id.clone().unwrap_or_default(),
        task_type: s.task_type.clone().unwrap_or_default(),
        task_def_name: s.task_def_name.clone().unwrap_or_default(),
        reference_task_name: String::new(),
        status: s
            .status
            .as_ref()
            .map(|st| st.to_string())
            .unwrap_or_default(),
        scheduled_time: s
            .scheduled_time
            .as_ref()
            .and_then(|t| t.parse::<i64>().ok())
            .unwrap_or(0),
        start_time: s
            .start_time
            .as_ref()
            .and_then(|t| t.parse::<i64>().ok())
            .unwrap_or(0),
        end_time: s
            .end_time
            .as_ref()
            .and_then(|t| t.parse::<i64>().ok())
            .unwrap_or(0),
        update_time: s
            .update_time
            .as_ref()
            .and_then(|t| t.parse::<i64>().ok())
            .unwrap_or(0),
    }
}

// â”€â”€ Official EventHandler â”€â”€

pub fn event_handler_to_official(h: &models::EventHandler) -> opb::OfEventHandlerPb {
    opb::OfEventHandlerPb {
        name: h.name.clone(),
        event: h.event.clone(),
        condition: h.condition.clone().unwrap_or_default(),
        actions: h
            .actions
            .iter()
            .map(|a| opb::OfEventActionPb {
                action: a
                    .action
                    .as_ref()
                    .map(|at| format!("{:?}", at))
                    .unwrap_or_default(),
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
            })
            .collect(),
        active: h.active,
        evaluator_type: h.evaluator_type.clone().unwrap_or_default(),
    }
}

pub fn event_handler_from_official(p: &opb::OfEventHandlerPb) -> models::EventHandler {
    models::EventHandler {
        name: p.name.clone(),
        event: p.event.clone(),
        condition: opt_string(&p.condition),
        actions: p
            .actions
            .iter()
            .map(|a| models::EventAction {
                action: None,
                start_workflow: a
                    .start_workflow
                    .as_ref()
                    .and_then(|s| serde_json::from_value(struct_to_json(s)).ok()),
                complete_task: a
                    .complete_task
                    .as_ref()
                    .and_then(|s| serde_json::from_value(struct_to_json(s)).ok()),
                fail_task: a
                    .fail_task
                    .as_ref()
                    .and_then(|s| serde_json::from_value(struct_to_json(s)).ok()),
                expand_inline_json: None,
                terminate_workflow: a
                    .terminate_workflow
                    .as_ref()
                    .and_then(|s| serde_json::from_value(struct_to_json(s)).ok()),
                update_workflow_variables: a
                    .update_workflow_variables
                    .as_ref()
                    .and_then(|s| serde_json::from_value(struct_to_json(s)).ok()),
            })
            .collect(),
        active: p.active,
        evaluator_type: opt_string(&p.evaluator_type),
    }
}
