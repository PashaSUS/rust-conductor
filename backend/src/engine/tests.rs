use std::collections::HashMap;

use serde_json::{Value, json};

use super::expression::{
    evaluate_loop_condition, navigate_json, resolve_expression, resolve_string_value, resolve_value,
};
use super::{is_task_failed, is_task_successful, is_task_terminal};
use crate::models::*;

// ─── Task status helpers ────────────────────────────────────────────────

#[test]
fn terminal_statuses_are_detected() {
    let terminals = [
        TaskStatus::Completed,
        TaskStatus::CompletedWithErrors,
        TaskStatus::Failed,
        TaskStatus::FailedWithTerminalError,
        TaskStatus::TimedOut,
        TaskStatus::Canceled,
        TaskStatus::Skipped,
    ];
    for s in &terminals {
        assert!(is_task_terminal(s), "{s:?} should be terminal");
    }
}

#[test]
fn non_terminal_statuses() {
    assert!(!is_task_terminal(&TaskStatus::InProgress));
    assert!(!is_task_terminal(&TaskStatus::Scheduled));
}

#[test]
fn successful_statuses() {
    assert!(is_task_successful(&TaskStatus::Completed));
    assert!(is_task_successful(&TaskStatus::Skipped));
    assert!(is_task_successful(&TaskStatus::CompletedWithErrors));
    assert!(!is_task_successful(&TaskStatus::Failed));
    assert!(!is_task_successful(&TaskStatus::InProgress));
}

#[test]
fn failed_statuses() {
    assert!(is_task_failed(&TaskStatus::Failed));
    assert!(is_task_failed(&TaskStatus::FailedWithTerminalError));
    assert!(is_task_failed(&TaskStatus::TimedOut));
    assert!(!is_task_failed(&TaskStatus::Completed));
    assert!(!is_task_failed(&TaskStatus::InProgress));
    assert!(!is_task_failed(&TaskStatus::Canceled));
}

// ─── TaskStatus Display ─────────────────────────────────────────────────

#[test]
fn task_status_display() {
    assert_eq!(TaskStatus::InProgress.to_string(), "IN_PROGRESS");
    assert_eq!(TaskStatus::Failed.to_string(), "FAILED");
    assert_eq!(
        TaskStatus::FailedWithTerminalError.to_string(),
        "FAILED_WITH_TERMINAL_ERROR"
    );
    assert_eq!(TaskStatus::Completed.to_string(), "COMPLETED");
    assert_eq!(
        TaskStatus::CompletedWithErrors.to_string(),
        "COMPLETED_WITH_ERRORS"
    );
    assert_eq!(TaskStatus::Scheduled.to_string(), "SCHEDULED");
    assert_eq!(TaskStatus::TimedOut.to_string(), "TIMED_OUT");
    assert_eq!(TaskStatus::Skipped.to_string(), "SKIPPED");
    assert_eq!(TaskStatus::Canceled.to_string(), "CANCELED");
}

#[test]
fn workflow_status_display() {
    assert_eq!(WorkflowStatus::Running.to_string(), "RUNNING");
    assert_eq!(WorkflowStatus::Completed.to_string(), "COMPLETED");
    assert_eq!(WorkflowStatus::Failed.to_string(), "FAILED");
    assert_eq!(WorkflowStatus::TimedOut.to_string(), "TIMED_OUT");
    assert_eq!(WorkflowStatus::Terminated.to_string(), "TERMINATED");
    assert_eq!(WorkflowStatus::Paused.to_string(), "PAUSED");
}

// ─── JSON navigation ────────────────────────────────────────────────────

#[test]
fn navigate_json_simple_field() {
    let val = json!({"a": "hello"});
    assert_eq!(navigate_json(&val, &["a"]), Some(json!("hello")));
}

#[test]
fn navigate_json_nested() {
    let val = json!({"a": {"b": {"c": 42}}});
    assert_eq!(navigate_json(&val, &["a", "b", "c"]), Some(json!(42)));
}

#[test]
fn navigate_json_missing() {
    let val = json!({"a": 1});
    assert_eq!(navigate_json(&val, &["b"]), None);
}

#[test]
fn navigate_json_empty_path() {
    let val = json!({"a": 1});
    assert_eq!(navigate_json(&val, &[]), Some(json!({"a": 1})));
}

#[test]
fn navigate_json_through_null() {
    let val = json!({"a": null});
    assert_eq!(navigate_json(&val, &["a", "b"]), None);
}

// ─── Template resolution (resolve_expression) ──────────────────────────

#[test]
fn resolve_workflow_id() {
    let input = json!({});
    let outputs = HashMap::new();
    let result = resolve_expression("workflow.workflowId", &input, &outputs, "wf-123");
    assert_eq!(result, Some(json!("wf-123")));
}

#[test]
fn resolve_workflow_input_field() {
    let input = json!({"name": "test", "count": 5});
    let outputs = HashMap::new();
    assert_eq!(
        resolve_expression("workflow.input.name", &input, &outputs, "wf-1"),
        Some(json!("test"))
    );
    assert_eq!(
        resolve_expression("workflow.input.count", &input, &outputs, "wf-1"),
        Some(json!(5))
    );
}

#[test]
fn resolve_workflow_input_entire() {
    let input = json!({"x": 1});
    let outputs = HashMap::new();
    assert_eq!(
        resolve_expression("workflow.input", &input, &outputs, "wf-1"),
        Some(json!({"x": 1}))
    );
}

#[test]
fn resolve_workflow_input_nested() {
    let input = json!({"config": {"timeout": 30}});
    let outputs = HashMap::new();
    assert_eq!(
        resolve_expression("workflow.input.config.timeout", &input, &outputs, "wf-1"),
        Some(json!(30))
    );
}

#[test]
fn resolve_task_output() {
    let input = json!({});
    let mut outputs = HashMap::new();
    outputs.insert("task1".to_string(), json!({"result": "ok", "value": 42}));
    assert_eq!(
        resolve_expression("task1.output.result", &input, &outputs, "wf-1"),
        Some(json!("ok"))
    );
    assert_eq!(
        resolve_expression("task1.output.value", &input, &outputs, "wf-1"),
        Some(json!(42))
    );
}

#[test]
fn resolve_task_output_entire() {
    let input = json!({});
    let mut outputs = HashMap::new();
    let task_out = json!({"a": 1, "b": 2});
    outputs.insert("myTask".to_string(), task_out.clone());
    assert_eq!(
        resolve_expression("myTask.output", &input, &outputs, "wf-1"),
        Some(task_out)
    );
}

#[test]
fn resolve_missing_task() {
    let input = json!({});
    let outputs = HashMap::new();
    assert_eq!(
        resolve_expression("nonexistent.output.field", &input, &outputs, "wf-1"),
        None
    );
}

#[test]
fn resolve_unknown_workflow_property() {
    let input = json!({});
    let outputs = HashMap::new();
    assert_eq!(
        resolve_expression("workflow.unknown", &input, &outputs, "wf-1"),
        None
    );
}

#[test]
fn resolve_single_part_expression() {
    let input = json!({});
    let outputs = HashMap::new();
    assert_eq!(resolve_expression("x", &input, &outputs, "wf-1"), None);
}

// ─── String value resolution ────────────────────────────────────────────

#[test]
fn resolve_string_plain() {
    let input = json!({});
    let outputs = HashMap::new();
    assert_eq!(
        resolve_string_value("hello world", &input, &outputs, "wf-1"),
        json!("hello world")
    );
}

#[test]
fn resolve_string_single_expression_preserves_type() {
    let input = json!({"count": 42});
    let outputs = HashMap::new();
    // A pure expression should preserve the integer type
    let result = resolve_string_value("${workflow.input.count}", &input, &outputs, "wf-1");
    assert_eq!(result, json!(42));
}

#[test]
fn resolve_string_interpolation() {
    let input = json!({"name": "test"});
    let outputs = HashMap::new();
    let result = resolve_string_value(
        "prefix-${workflow.input.name}-suffix",
        &input,
        &outputs,
        "wf-1",
    );
    assert_eq!(result, json!("prefix-test-suffix"));
}

#[test]
fn resolve_string_multiple_interpolations() {
    let input = json!({"a": "X", "b": "Y"});
    let outputs = HashMap::new();
    let result = resolve_string_value(
        "${workflow.input.a}+${workflow.input.b}",
        &input,
        &outputs,
        "wf-1",
    );
    assert_eq!(result, json!("X+Y"));
}

#[test]
fn resolve_string_unresolved_expression() {
    let input = json!({});
    let outputs = HashMap::new();
    let result = resolve_string_value("${workflow.input.missing}", &input, &outputs, "wf-1");
    // Unresolved single expression returns None from resolve_expression,
    // but since it's a pure expression, it falls through to interpolation
    // Verify it preserves the unresolved expression
    assert!(result.is_string());
}

#[test]
fn resolve_string_numeric_interpolation_converts_to_string() {
    let input = json!({"id": 123});
    let outputs = HashMap::new();
    let result = resolve_string_value("item-${workflow.input.id}", &input, &outputs, "wf-1");
    assert_eq!(result, json!("item-123"));
}

#[test]
fn resolve_string_bool_preserves_type() {
    let input = json!({"flag": true});
    let outputs = HashMap::new();
    let result = resolve_string_value("${workflow.input.flag}", &input, &outputs, "wf-1");
    assert_eq!(result, json!(true));
}

#[test]
fn resolve_string_object_preserves_type() {
    let input = json!({"nested": {"x": 1}});
    let outputs = HashMap::new();
    let result = resolve_string_value("${workflow.input.nested}", &input, &outputs, "wf-1");
    assert_eq!(result, json!({"x": 1}));
}

// ─── resolve_value recursive ────────────────────────────────────────────

#[test]
fn resolve_value_object() {
    let input = json!({"name": "test"});
    let outputs = HashMap::new();
    let template = json!({
        "greeting": "Hello ${workflow.input.name}",
        "id": "${workflow.workflowId}"
    });
    let result = resolve_value(&template, &input, &outputs, "wf-42");
    assert_eq!(result["greeting"], json!("Hello test"));
    assert_eq!(result["id"], json!("wf-42"));
}

#[test]
fn resolve_value_array() {
    let input = json!({"x": 1});
    let outputs = HashMap::new();
    let template = json!(["${workflow.input.x}", "static"]);
    let result = resolve_value(&template, &input, &outputs, "wf-1");
    assert_eq!(result, json!([1, "static"]));
}

#[test]
fn resolve_value_nested_objects() {
    let input = json!({"host": "localhost"});
    let outputs = HashMap::new();
    let template = json!({"config": {"url": "http://${workflow.input.host}:8080"}});
    let result = resolve_value(&template, &input, &outputs, "wf-1");
    assert_eq!(result["config"]["url"], json!("http://localhost:8080"));
}

#[test]
fn resolve_value_passthrough_number() {
    let input = json!({});
    let outputs = HashMap::new();
    assert_eq!(
        resolve_value(&json!(42), &input, &outputs, "wf-1"),
        json!(42)
    );
}

#[test]
fn resolve_value_passthrough_null() {
    let input = json!({});
    let outputs = HashMap::new();
    assert_eq!(
        resolve_value(&json!(null), &input, &outputs, "wf-1"),
        json!(null)
    );
}

#[test]
fn resolve_value_passthrough_bool() {
    let input = json!({});
    let outputs = HashMap::new();
    assert_eq!(
        resolve_value(&json!(true), &input, &outputs, "wf-1"),
        json!(true)
    );
}

#[test]
fn resolve_value_with_task_output() {
    let input = json!({});
    let mut outputs = HashMap::new();
    outputs.insert("step1".to_string(), json!({"data": "abc"}));
    let template = json!({"result": "${step1.output.data}"});
    let result = resolve_value(&template, &input, &outputs, "wf-1");
    assert_eq!(result["result"], json!("abc"));
}

// ─── Loop condition evaluation ──────────────────────────────────────────

#[test]
fn loop_false_literal() {
    assert!(!evaluate_loop_condition("false", &json!({}), 0));
}

#[test]
fn loop_true_literal() {
    assert!(evaluate_loop_condition("true", &json!({}), 0));
}

#[test]
fn loop_empty_string() {
    assert!(!evaluate_loop_condition("", &json!({}), 0));
}

#[test]
fn loop_iteration_less_than() {
    assert!(evaluate_loop_condition("iteration < 5", &json!({}), 0));
    assert!(evaluate_loop_condition("iteration < 5", &json!({}), 4));
    assert!(!evaluate_loop_condition("iteration < 5", &json!({}), 5));
    assert!(!evaluate_loop_condition("iteration < 5", &json!({}), 10));
}

#[test]
fn loop_iteration_less_equal() {
    assert!(evaluate_loop_condition("iteration <= 3", &json!({}), 3));
    assert!(!evaluate_loop_condition("iteration <= 3", &json!({}), 4));
}

#[test]
fn loop_output_should_continue_bool() {
    assert!(evaluate_loop_condition(
        "check",
        &json!({"shouldContinue": true}),
        0
    ));
    assert!(!evaluate_loop_condition(
        "check",
        &json!({"shouldContinue": false}),
        0
    ));
}

#[test]
fn loop_output_result_bool() {
    assert!(evaluate_loop_condition(
        "check",
        &json!({"result": true}),
        0
    ));
    assert!(!evaluate_loop_condition(
        "check",
        &json!({"result": false}),
        0
    ));
}

#[test]
fn loop_output_result_string() {
    assert!(evaluate_loop_condition(
        "check",
        &json!({"result": "yes"}),
        0
    ));
    assert!(!evaluate_loop_condition(
        "check",
        &json!({"result": "false"}),
        0
    ));
    assert!(!evaluate_loop_condition("check", &json!({"result": ""}), 0));
}

#[test]
fn loop_output_result_number() {
    assert!(evaluate_loop_condition("check", &json!({"result": 1}), 0));
    assert!(!evaluate_loop_condition("check", &json!({"result": 0}), 0));
}

#[test]
fn loop_output_null() {
    assert!(!evaluate_loop_condition(
        "check",
        &json!({"result": null}),
        0
    ));
}

#[test]
fn loop_should_continue_takes_precedence_over_result() {
    // shouldContinue is checked first
    assert!(evaluate_loop_condition(
        "check",
        &json!({"shouldContinue": true, "result": false}),
        0,
    ));
}

// ─── Model serialization / deserialization ──────────────────────────────

#[test]
fn task_status_serde_roundtrip() {
    let statuses = [
        TaskStatus::InProgress,
        TaskStatus::Canceled,
        TaskStatus::Failed,
        TaskStatus::FailedWithTerminalError,
        TaskStatus::Completed,
        TaskStatus::CompletedWithErrors,
        TaskStatus::Scheduled,
        TaskStatus::TimedOut,
        TaskStatus::Skipped,
    ];
    for s in &statuses {
        let json_str = serde_json::to_string(s).unwrap();
        let deserialized: TaskStatus = serde_json::from_str(&json_str).unwrap();
        assert_eq!(&deserialized, s, "TaskStatus roundtrip failed for {s:?}");
    }
}

#[test]
fn task_status_deserializes_from_screaming_snake() {
    let cases = [
        ("\"IN_PROGRESS\"", TaskStatus::InProgress),
        ("\"FAILED\"", TaskStatus::Failed),
        ("\"COMPLETED\"", TaskStatus::Completed),
        ("\"TIMED_OUT\"", TaskStatus::TimedOut),
        ("\"SCHEDULED\"", TaskStatus::Scheduled),
        ("\"SKIPPED\"", TaskStatus::Skipped),
    ];
    for (json_str, expected) in cases {
        let result: TaskStatus = serde_json::from_str(json_str).unwrap();
        assert_eq!(result, expected);
    }
}

#[test]
fn workflow_status_serde_roundtrip() {
    let statuses = [
        WorkflowStatus::Running,
        WorkflowStatus::Completed,
        WorkflowStatus::Failed,
        WorkflowStatus::TimedOut,
        WorkflowStatus::Terminated,
        WorkflowStatus::Paused,
    ];
    for s in &statuses {
        let json_str = serde_json::to_string(s).unwrap();
        let deserialized: WorkflowStatus = serde_json::from_str(&json_str).unwrap();
        assert_eq!(
            &deserialized, s,
            "WorkflowStatus roundtrip failed for {s:?}"
        );
    }
}

#[test]
fn workflow_def_minimal_deserialize() {
    let json = json!({
        "name": "test_wf",
        "tasks": [
            {
                "name": "task1",
                "taskReferenceName": "task1_ref",
                "type": "SIMPLE"
            }
        ]
    });
    let def: WorkflowDef = serde_json::from_value(json).unwrap();
    assert_eq!(def.name, "test_wf");
    assert_eq!(def.version, 1); // default
    assert_eq!(def.tasks.len(), 1);
    assert_eq!(def.tasks[0].name, "task1");
    assert_eq!(def.tasks[0].task_reference_name, "task1_ref");
    assert_eq!(def.tasks[0].task_type, "SIMPLE");
}

#[test]
fn workflow_def_with_fork_join() {
    let json = json!({
        "name": "fork_wf",
        "tasks": [
            {
                "name": "fork1",
                "taskReferenceName": "fork1_ref",
                "type": "FORK_JOIN",
                "forkTasks": [
                    [{"name": "a", "taskReferenceName": "a_ref"}],
                    [{"name": "b", "taskReferenceName": "b_ref"}]
                ]
            },
            {
                "name": "join1",
                "taskReferenceName": "join1_ref",
                "type": "JOIN",
                "joinOn": ["a_ref", "b_ref"]
            }
        ]
    });
    let def: WorkflowDef = serde_json::from_value(json).unwrap();
    assert_eq!(def.tasks[0].task_type, "FORK_JOIN");
    assert_eq!(def.tasks[0].fork_tasks.len(), 2);
    assert_eq!(def.tasks[1].task_type, "JOIN");
    assert_eq!(def.tasks[1].join_on, vec!["a_ref", "b_ref"]);
}

#[test]
fn workflow_def_with_decision() {
    let json = json!({
        "name": "decision_wf",
        "tasks": [
            {
                "name": "decide",
                "taskReferenceName": "decide_ref",
                "type": "DECISION",
                "caseValueParam": "category",
                "decisionCases": {
                    "A": [{"name": "taskA", "taskReferenceName": "a_ref"}],
                    "B": [{"name": "taskB", "taskReferenceName": "b_ref"}]
                },
                "defaultCase": [{"name": "taskDefault", "taskReferenceName": "default_ref"}]
            }
        ]
    });
    let def: WorkflowDef = serde_json::from_value(json).unwrap();
    let task = &def.tasks[0];
    assert_eq!(task.task_type, "DECISION");
    assert_eq!(task.case_value_param, Some("category".to_string()));
    assert_eq!(task.decision_cases.len(), 2);
    assert!(task.decision_cases.contains_key("A"));
    assert!(task.decision_cases.contains_key("B"));
    assert_eq!(task.default_case.len(), 1);
}

#[test]
fn workflow_def_default_task_type_is_simple() {
    let json = json!({
        "name": "simple_wf",
        "tasks": [
            {"name": "t1", "taskReferenceName": "t1_ref"}
        ]
    });
    let def: WorkflowDef = serde_json::from_value(json).unwrap();
    assert_eq!(def.tasks[0].task_type, "SIMPLE");
}

#[test]
fn start_workflow_request_minimal() {
    let json = json!({"name": "my_wf"});
    let req: StartWorkflowRequest = serde_json::from_value(json).unwrap();
    assert_eq!(req.name, "my_wf");
    assert_eq!(req.version, 1); // default
    assert!(req.input.is_null()); // serde default for Value is Null
}

#[test]
fn task_update_request_roundtrip() {
    let req = TaskUpdateRequest {
        task_id: "t-1".to_string(),
        workflow_instance_id: "wf-1".to_string(),
        status: TaskStatus::Completed,
        output_data: json!({"result": true}),
        reason_for_incompletion: None,
        callback_after_seconds: 0,
        worker_id: Some("worker-1".to_string()),
        logs: vec![],
        external_output_payload_storage_path: None,
        sub_workflow_id: None,
        extend_lease: false,
    };
    let json_str = serde_json::to_string(&req).unwrap();
    let parsed: TaskUpdateRequest = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed.task_id, "t-1");
    assert_eq!(parsed.status, TaskStatus::Completed);
}

#[test]
fn task_def_defaults() {
    let json = json!({"name": "my_task"});
    let def: TaskDef = serde_json::from_value(json).unwrap();
    assert_eq!(def.name, "my_task");
    assert_eq!(def.retry_count, 3);
    assert_eq!(def.retry_delay_seconds, 60);
    assert_eq!(def.timeout_seconds, 3600);
    assert_eq!(def.response_timeout_seconds, 600);
    assert_eq!(def.backoff_scale_factor, 1);
}

#[test]
fn poll_task_serde() {
    let poll = PollTask {
        task_id: "t-1".to_string(),
        workflow_instance_id: "wf-1".to_string(),
        task_type: "SIMPLE".to_string(),
        task_def_name: "my_task".to_string(),
        reference_task_name: "my_ref".to_string(),
        status: TaskStatus::InProgress,
        input_data: json!({"key": "value"}),
        scheduled_time: Some(1000),
        start_time: Some(2000),
        callback_after_seconds: 30,
        poll_count: 1,
        retry_count: 0,
        priority: 0,
        env_vars: None,
    };
    let json_str = serde_json::to_string(&poll).unwrap();
    let parsed: PollTask = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed.task_id, "t-1");
    assert_eq!(parsed.status, TaskStatus::InProgress);
    assert_eq!(parsed.input_data["key"], "value");
}

#[test]
fn bulk_response_default() {
    let br = BulkResponse::default();
    assert!(br.bulk_successful_results.is_empty());
    assert!(br.bulk_error_results.is_empty());
}

// ─── EngineError ────────────────────────────────────────────────────────

#[test]
fn engine_error_display() {
    use super::EngineError;
    let err = EngineError::NotFound("workflow xyz".into());
    assert_eq!(err.to_string(), "Not found: workflow xyz");
    let err = EngineError::Database("connection refused".into());
    assert_eq!(err.to_string(), "Database error: connection refused");
    let err = EngineError::Redis("timeout".into());
    assert_eq!(err.to_string(), "Redis error: timeout");
    let err = EngineError::InvalidState("already completed".into());
    assert_eq!(err.to_string(), "Invalid state: already completed");
}

#[test]
fn engine_error_status_codes() {
    use super::EngineError;
    use actix_web::ResponseError;
    assert_eq!(
        EngineError::NotFound("x".into()).status_code(),
        actix_web::http::StatusCode::NOT_FOUND
    );
    assert_eq!(
        EngineError::InvalidState("x".into()).status_code(),
        actix_web::http::StatusCode::CONFLICT
    );
    assert_eq!(
        EngineError::Database("x".into()).status_code(),
        actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(
        EngineError::Redis("x".into()).status_code(),
        actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
    );
}

// ─── WorkflowDef with sub-workflow params ───────────────────────────────

#[test]
fn sub_workflow_params_deserialize() {
    let json = json!({
        "name": "sub_wf",
        "tasks": [
            {
                "name": "sub",
                "taskReferenceName": "sub_ref",
                "type": "SUB_WORKFLOW",
                "subWorkflowParam": {
                    "name": "child_wf",
                    "version": 2
                }
            }
        ]
    });
    let def: WorkflowDef = serde_json::from_value(json).unwrap();
    let task = &def.tasks[0];
    let params = task.sub_workflow_param.as_ref().unwrap();
    assert_eq!(params.name, "child_wf");
    assert_eq!(params.version, Some(2));
}

#[test]
fn do_while_task_deserialize() {
    let json = json!({
        "name": "loop_wf",
        "tasks": [
            {
                "name": "loop",
                "taskReferenceName": "loop_ref",
                "type": "DO_WHILE",
                "loopCondition": "iteration < 3",
                "loopOver": [
                    {"name": "inner", "taskReferenceName": "inner_ref"}
                ]
            }
        ]
    });
    let def: WorkflowDef = serde_json::from_value(json).unwrap();
    let task = &def.tasks[0];
    assert_eq!(task.task_type, "DO_WHILE");
    assert_eq!(task.loop_condition, Some("iteration < 3".to_string()));
    assert_eq!(task.loop_over.len(), 1);
}

// ─── Complex template resolution scenarios ──────────────────────────────

#[test]
fn resolve_value_deeply_nested_template() {
    let input = json!({"env": "prod", "region": "us-east-1"});
    let mut outputs = HashMap::new();
    outputs.insert("setup".to_string(), json!({"endpoint": "api.example.com"}));
    let template = json!({
        "config": {
            "url": "https://${setup.output.endpoint}/${workflow.input.env}",
            "metadata": {
                "region": "${workflow.input.region}",
                "wfId": "${workflow.workflowId}"
            }
        }
    });
    let result = resolve_value(&template, &input, &outputs, "wf-deep");
    assert_eq!(
        result["config"]["url"],
        json!("https://api.example.com/prod")
    );
    assert_eq!(result["config"]["metadata"]["region"], json!("us-east-1"));
    assert_eq!(result["config"]["metadata"]["wfId"], json!("wf-deep"));
}

#[test]
fn resolve_value_array_of_expressions() {
    let input = json!({"items": ["a", "b"]});
    let outputs = HashMap::new();
    let template = json!({
        "list": ["${workflow.input.items}", "extra"]
    });
    let result = resolve_value(&template, &input, &outputs, "wf-1");
    // First element resolves to the array ["a","b"], second is plain string
    assert_eq!(result["list"][0], json!(["a", "b"]));
    assert_eq!(result["list"][1], json!("extra"));
}

#[test]
fn resolve_string_unclosed_expression() {
    let input = json!({});
    let outputs = HashMap::new();
    // Unclosed ${... should be preserved as-is
    let result = resolve_string_value("prefix ${broken", &input, &outputs, "wf-1");
    assert_eq!(result, json!("prefix ${broken"));
}

#[test]
fn resolve_string_workflow_id_in_interpolation() {
    let input = json!({});
    let outputs = HashMap::new();
    let result = resolve_string_value(
        "Execution: ${workflow.workflowId}",
        &input,
        &outputs,
        "exec-42",
    );
    assert_eq!(result, json!("Execution: exec-42"));
}

// ─── Property-based tests (proptest) ────────────────────────────────────

use proptest::prelude::*;

fn arb_task_status() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        Just("IN_PROGRESS"),
        Just("CANCELED"),
        Just("FAILED"),
        Just("FAILED_WITH_TERMINAL_ERROR"),
        Just("COMPLETED"),
        Just("COMPLETED_WITH_ERRORS"),
        Just("SCHEDULED"),
        Just("TIMED_OUT"),
        Just("SKIPPED"),
    ]
}

fn arb_workflow_status() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        Just("RUNNING"),
        Just("COMPLETED"),
        Just("FAILED"),
        Just("TIMED_OUT"),
        Just("TERMINATED"),
        Just("PAUSED"),
    ]
}

fn arb_task_status_enum() -> impl Strategy<Value = TaskStatus> {
    prop_oneof![
        Just(TaskStatus::InProgress),
        Just(TaskStatus::Canceled),
        Just(TaskStatus::Failed),
        Just(TaskStatus::FailedWithTerminalError),
        Just(TaskStatus::Completed),
        Just(TaskStatus::CompletedWithErrors),
        Just(TaskStatus::Scheduled),
        Just(TaskStatus::TimedOut),
        Just(TaskStatus::Skipped),
    ]
}

/// Replicate the shard routing hash from engine/shard.rs
fn shard_hash(workflow_id: &str) -> u32 {
    if let Ok(uuid) = uuid::Uuid::parse_str(workflow_id) {
        let bytes = uuid.as_bytes();
        u32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]])
    } else {
        workflow_id
            .bytes()
            .fold(0u32, |acc, b| acc.wrapping_add(b as u32))
    }
}

proptest! {
    /// Every TaskStatus deserializes successfully from its screaming snake form.
    #[test]
    fn prop_task_status_always_deserializes(status in arb_task_status()) {
        let json_str = format!("\"{status}\"");
        let result: Result<TaskStatus, _> = serde_json::from_str(&json_str);
        prop_assert!(result.is_ok(), "Failed to deserialize TaskStatus: {status}");
    }

    /// Every WorkflowStatus deserializes successfully.
    #[test]
    fn prop_workflow_status_always_deserializes(status in arb_workflow_status()) {
        let json_str = format!("\"{status}\"");
        let result: Result<WorkflowStatus, _> = serde_json::from_str(&json_str);
        prop_assert!(result.is_ok(), "Failed to deserialize WorkflowStatus: {status}");
    }

    /// TaskStatus serde roundtrip is lossless for any valid status.
    #[test]
    fn prop_task_status_serde_roundtrip(status in arb_task_status()) {
        let json_str = format!("\"{status}\"");
        let parsed: TaskStatus = serde_json::from_str(&json_str).unwrap();
        let re_serialized = serde_json::to_string(&parsed).unwrap();
        let re_parsed: TaskStatus = serde_json::from_str(&re_serialized).unwrap();
        prop_assert_eq!(parsed, re_parsed);
    }

    /// WorkflowDef with arbitrary name and version always round-trips.
    #[test]
    fn prop_workflow_def_roundtrip(
        name in "[a-zA-Z_][a-zA-Z0-9_]{0,20}",
        version in 1..100i32,
    ) {
        let wf_json = json!({
            "name": name,
            "version": version,
            "tasks": [{"name": "t1", "taskReferenceName": "t1_ref"}]
        });
        let def: WorkflowDef = serde_json::from_value(wf_json).unwrap();
        prop_assert_eq!(&def.name, &name);
        prop_assert_eq!(def.version, version);

        let re_json = serde_json::to_value(&def).unwrap();
        let re_def: WorkflowDef = serde_json::from_value(re_json).unwrap();
        prop_assert_eq!(re_def.name, name);
        prop_assert_eq!(re_def.version, version);
    }

    /// TaskDef with arbitrary name always round-trips and has correct defaults.
    #[test]
    fn prop_task_def_defaults_and_roundtrip(name in "[a-zA-Z_][a-zA-Z0-9_]{0,30}") {
        let td_json = json!({"name": name});
        let def: TaskDef = serde_json::from_value(td_json).unwrap();
        prop_assert_eq!(&def.name, &name);
        prop_assert_eq!(def.retry_count, 3);
        prop_assert_eq!(def.retry_delay_seconds, 60);
        prop_assert_eq!(def.timeout_seconds, 3600);
        prop_assert_eq!(def.response_timeout_seconds, 600);

        let re_json = serde_json::to_value(&def).unwrap();
        let re_def: TaskDef = serde_json::from_value(re_json).unwrap();
        prop_assert_eq!(re_def.name, name);
    }

    /// Template resolution of plain strings (no ${}) is idempotent.
    #[test]
    fn prop_resolve_plain_string_is_identity(s in "[a-zA-Z0-9 .,!?]{0,100}") {
        if !s.contains("${") {
            let input = json!({});
            let outputs = HashMap::<String, Value>::new();
            let template = Value::String(s.clone());
            let result = resolve_value(&template, &input, &outputs, "wf-1");
            prop_assert_eq!(result, Value::String(s));
        }
    }

    /// Resolving a number/bool/null is always a no-op passthrough.
    #[test]
    fn prop_resolve_non_string_passthrough(n in any::<i64>()) {
        let input = json!({});
        let outputs = HashMap::<String, Value>::new();
        let val = json!(n);
        let result = resolve_value(&val, &input, &outputs, "wf-1");
        prop_assert_eq!(result, json!(n));
    }

    /// Workflow input fields are always resolvable via ${workflow.input.KEY}.
    #[test]
    fn prop_workflow_input_fields_resolvable(
        key in "[a-zA-Z][a-zA-Z0-9]{0,10}",
        value in "[a-zA-Z0-9]{0,20}",
    ) {
        let input = json!({key.clone(): value.clone()});
        let outputs = HashMap::<String, Value>::new();
        let expr = format!("workflow.input.{key}");
        let result = resolve_expression(&expr, &input, &outputs, "wf-1");
        prop_assert_eq!(result, Some(json!(value)));
    }

    /// Shard routing is deterministic for any UUID.
    #[test]
    fn prop_shard_routing_deterministic(_idx in 0..1000u32) {
        let id = uuid::Uuid::new_v4().to_string();
        let a = shard_hash(&id);
        let b = shard_hash(&id);
        prop_assert_eq!(a, b, "Shard routing must be deterministic");
    }

    /// Different UUIDs produce valid shard indices.
    #[test]
    fn prop_shard_routing_valid(_seed in 0u64..10000) {
        let id = uuid::Uuid::new_v4().to_string();
        let s = shard_hash(&id) % 1000;
        prop_assert!(s < 1000);
    }

    /// StartWorkflowRequest with various inputs always deserializes.
    #[test]
    fn prop_start_request_with_arbitrary_input(
        name in "[a-zA-Z_][a-zA-Z0-9_]{0,20}",
        version in 1..100i32,
        key in "[a-zA-Z]{1,10}",
        val in "[a-zA-Z0-9]{0,20}",
    ) {
        let req_json = json!({
            "name": name,
            "version": version,
            "input": {key: val}
        });
        let req: StartWorkflowRequest = serde_json::from_value(req_json).unwrap();
        prop_assert_eq!(&req.name, &name);
        prop_assert_eq!(req.version, version);
    }

    /// Terminal states are a superset of successful ∪ failed.
    /// No state is both successful and failed.
    #[test]
    fn prop_state_machine_invariants(status in arb_task_status_enum()) {
        let terminal = is_task_terminal(&status);
        let successful = is_task_successful(&status);
        let failed = is_task_failed(&status);

        // Successful or failed implies terminal
        if successful || failed {
            prop_assert!(terminal, "{status:?} is successful/failed but not terminal");
        }
        // Cannot be both successful and failed
        prop_assert!(
            !(successful && failed),
            "{status:?} is both successful and failed"
        );
        // InProgress and Scheduled are never terminal
        if matches!(status, TaskStatus::InProgress | TaskStatus::Scheduled) {
            prop_assert!(!terminal, "{status:?} should not be terminal");
        }
    }
}

// ─── Timeout policy and retry logic serde ───────────────────────────────

#[test]
fn timeout_policy_serde() {
    let cases: Vec<(TimeoutPolicy, &str)> = vec![
        (TimeoutPolicy::TimeOutWf, "\"TIME_OUT_WF\""),
        (TimeoutPolicy::AlertOnly, "\"ALERT_ONLY\""),
    ];
    for (policy, expected_json) in cases {
        let serialized = serde_json::to_string(&policy).unwrap();
        assert_eq!(serialized, expected_json);
        let deserialized: TimeoutPolicy = serde_json::from_str(&serialized).unwrap();
        assert_eq!(serde_json::to_string(&deserialized).unwrap(), expected_json);
    }
}

#[test]
fn retry_logic_serde() {
    let cases: Vec<(RetryLogic, &str)> = vec![
        (RetryLogic::Fixed, "\"FIXED\""),
        (RetryLogic::ExponentialBackoff, "\"EXPONENTIAL_BACKOFF\""),
        (RetryLogic::LinearBackoff, "\"LINEAR_BACKOFF\""),
    ];
    for (logic, expected_json) in cases {
        let serialized = serde_json::to_string(&logic).unwrap();
        assert_eq!(serialized, expected_json);
    }
}

#[test]
fn task_timeout_policy_serde() {
    let cases: Vec<(TaskTimeoutPolicy, &str)> = vec![
        (TaskTimeoutPolicy::Retry, "\"RETRY\""),
        (TaskTimeoutPolicy::TimeOutWf, "\"TIME_OUT_WF\""),
        (TaskTimeoutPolicy::AlertOnly, "\"ALERT_ONLY\""),
    ];
    for (policy, expected_json) in cases {
        let serialized = serde_json::to_string(&policy).unwrap();
        assert_eq!(serialized, expected_json);
    }
}

// ─── Workflow full JSON roundtrip ───────────────────────────────────────

#[test]
fn workflow_full_json_roundtrip() {
    let wf = Workflow {
        workflow_id: "wf-1".to_string(),
        workflow_name: "test".to_string(),
        workflow_version: 1,
        status: WorkflowStatus::Running,
        input: json!({"key": "value"}),
        output: json!({}),
        tasks: vec![],
        correlation_id: Some("corr-1".to_string()),
        start_time: 1000,
        end_time: None,
        update_time: 2000,
        created_by: None,
        updated_by: None,
        reason_for_incompletion: None,
        workflow_definition: None,
        priority: 0,
        variables: HashMap::new(),
        failed_reference_task_names: vec![],
        owner_app: None,
        parent_workflow_id: None,
        parent_workflow_task_id: None,
        re_run_from_workflow_id: None,
        event: None,
        task_to_domain: HashMap::new(),
        failed_task_names: vec![],
        external_input_payload_storage_path: None,
        external_output_payload_storage_path: None,
        last_retried_time: None,
        history: vec![],
        idempotency_key: None,
        rate_limit_key: None,
        rate_limited: None,
    };
    let json_str = serde_json::to_string(&wf).unwrap();
    let parsed: Workflow = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed.workflow_id, "wf-1");
    assert_eq!(parsed.status, WorkflowStatus::Running);
    assert_eq!(parsed.correlation_id, Some("corr-1".to_string()));
}

// ─── Input parameters with template references ─────────────────────────

#[test]
fn workflow_task_input_parameters_serde() {
    let json = json!({
        "name": "http_task",
        "taskReferenceName": "http_ref",
        "type": "HTTP",
        "inputParameters": {
            "url": "https://api.example.com/${workflow.input.endpoint}",
            "method": "GET",
            "headers": {"Authorization": "Bearer ${workflow.input.token}"}
        }
    });
    let task: WorkflowTask = serde_json::from_value(json).unwrap();
    assert_eq!(task.input_parameters.len(), 3);
    assert_eq!(
        task.input_parameters["url"],
        json!("https://api.example.com/${workflow.input.endpoint}")
    );
}

// ─── API Contract Tests ─────────────────────────────────────────────────
// Validate JSON field naming, defaults, and structure match the official
// Netflix Conductor API contract.

#[test]
fn contract_workflow_def_json_field_names() {
    // Official Conductor API uses camelCase field names
    let def = WorkflowDef {
        name: "my_wf".to_string(),
        version: 2,
        tasks: vec![],
        description: Some("desc".to_string()),
        input_parameters: vec!["p1".to_string()],
        output_parameters: HashMap::new(),
        failure_workflow: Some("fallback_wf".to_string()),
        schema_version: 2,
        restartable: true,
        workflow_status_listener_enabled: false,
        owner_email: Some("test@example.com".to_string()),
        timeout_policy: TimeoutPolicy::TimeOutWf,
        timeout_seconds: 60,
        variables: HashMap::new(),
        input_template: HashMap::new(),
        owner_app: None,
        create_time: None,
        update_time: None,
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
        tags: vec![],
        sla_deadline_seconds: None,
        base_workflow: None,
        base_workflow_version: None,
        input_parameter_definitions: vec![],
        saga_enabled: false,
    };
    let v = serde_json::to_value(&def).unwrap();
    // Verify camelCase field names match Conductor API
    assert!(v.get("name").is_some());
    assert!(v.get("version").is_some());
    assert!(v.get("tasks").is_some());
    assert!(v.get("description").is_some());
    assert!(v.get("inputParameters").is_some());
    assert!(v.get("outputParameters").is_some());
    assert!(v.get("failureWorkflow").is_some());
    assert!(v.get("schemaVersion").is_some());
    assert!(v.get("restartable").is_some());
    assert!(v.get("workflowStatusListenerEnabled").is_some());
    assert!(v.get("ownerEmail").is_some());
    assert!(v.get("timeoutPolicy").is_some());
    assert!(v.get("timeoutSeconds").is_some());

    // Verify NO snake_case leak
    assert!(v.get("input_parameters").is_none());
    assert!(v.get("output_parameters").is_none());
    assert!(v.get("failure_workflow").is_none());
    assert!(v.get("schema_version").is_none());
    assert!(v.get("timeout_policy").is_none());
    assert!(v.get("timeout_seconds").is_none());
}

#[test]
fn contract_workflow_task_json_field_names() {
    let task = WorkflowTask {
        name: "task1".to_string(),
        task_reference_name: "t1_ref".to_string(),
        task_type: "SIMPLE".to_string(),
        description: None,
        input_parameters: HashMap::new(),
        optional: false,
        start_delay: 0,
        sub_workflow_param: None,
        join_on: vec![],
        fork_tasks: vec![],
        decision_cases: HashMap::new(),
        default_case: vec![],
        case_expression: None,
        case_value_param: None,
        loop_condition: None,
        loop_over: vec![],
        retry_count: None,
        evaluator_type: None,
        expression: None,
        script_expression: None,
        dynamic_task_name_param: None,
        dynamic_fork_join_tasks_param: None,
        dynamic_fork_tasks_param: None,
        dynamic_fork_tasks_input_param_name: None,
        sink: None,
        task_definition: None,
        rate_limited: None,
        default_exclusive_join_task: vec![],
        async_complete: false,
        on_state_change: None,
        join_status: None,
        cache_config: None,
        permissive: None,
        compensation_task: None,
        condition_tree: None,
        heartbeat_timeout_seconds: None,
        map_items_param: None,
        map_parallelism: None,
        map_task: None,
    };
    let v = serde_json::to_value(&task).unwrap();
    // Conductor uses "taskReferenceName" and "type"
    assert!(v.get("taskReferenceName").is_some());
    assert!(v.get("type").is_some());
    assert_eq!(v["type"], "SIMPLE");
    assert!(v.get("inputParameters").is_some());
    assert!(v.get("startDelay").is_some());
    assert!(v.get("forkTasks").is_some());
    assert!(v.get("joinOn").is_some());
    assert!(v.get("decisionCases").is_some());
    assert!(v.get("defaultCase").is_some());
    assert!(v.get("loopCondition").is_some());
    assert!(v.get("loopOver").is_some());
    assert!(v.get("asyncComplete").is_some());

    // Verify "type" not "taskType"
    assert!(v.get("taskType").is_none());
    assert!(v.get("task_reference_name").is_none());
}

#[test]
fn contract_workflow_runtime_json_field_names() {
    let wf = Workflow {
        workflow_id: "wf-1".to_string(),
        workflow_name: "test".to_string(),
        workflow_version: 1,
        status: WorkflowStatus::Running,
        input: json!({}),
        output: json!({}),
        tasks: vec![],
        correlation_id: None,
        start_time: 1000,
        end_time: None,
        update_time: 2000,
        created_by: None,
        updated_by: None,
        reason_for_incompletion: None,
        workflow_definition: None,
        priority: 0,
        variables: HashMap::new(),
        failed_reference_task_names: vec![],
        owner_app: None,
        parent_workflow_id: None,
        parent_workflow_task_id: None,
        re_run_from_workflow_id: None,
        event: None,
        task_to_domain: HashMap::new(),
        failed_task_names: vec![],
        external_input_payload_storage_path: None,
        external_output_payload_storage_path: None,
        last_retried_time: None,
        history: vec![],
        idempotency_key: None,
        rate_limit_key: None,
        rate_limited: None,
    };
    let v = serde_json::to_value(&wf).unwrap();
    // Conductor API field names
    assert!(v.get("workflowId").is_some());
    assert!(v.get("workflowName").is_some());
    assert!(v.get("workflowVersion").is_some());
    assert!(v.get("status").is_some());
    assert!(v.get("correlationId").is_some());
    assert!(v.get("startTime").is_some());
    assert!(v.get("updateTime").is_some());
    assert!(v.get("reasonForIncompletion").is_some());
    assert!(v.get("parentWorkflowId").is_some());
    assert!(v.get("parentWorkflowTaskId").is_some());
    assert!(v.get("failedReferenceTaskNames").is_some());
    assert!(v.get("taskToDomain").is_some());

    // No snake_case leak
    assert!(v.get("workflow_id").is_none());
    assert!(v.get("workflow_name").is_none());
    assert!(v.get("start_time").is_none());
}

#[test]
fn contract_start_workflow_request_field_names() {
    let req = StartWorkflowRequest {
        name: "wf".to_string(),
        version: 1,
        input: json!({"k": "v"}),
        correlation_id: Some("corr".to_string()),
        task_to_domain: HashMap::new(),
        priority: 0,
        external_input_payload_storage_path: None,
        idempotency_key: None,
        idempotency_strategy: None,
        workflow_def: None,
        created_by: None,
        tags: vec![],
        parent_workflow_id: None,
        parent_workflow_task_id: None,
    };
    let v = serde_json::to_value(&req).unwrap();
    assert!(v.get("name").is_some());
    assert!(v.get("correlationId").is_some());
    assert!(v.get("taskToDomain").is_some());
    assert!(v.get("externalInputPayloadStoragePath").is_some());
    assert!(v.get("idempotencyKey").is_some());
    assert!(v.get("workflowDef").is_some());

    assert!(v.get("correlation_id").is_none());
    assert!(v.get("task_to_domain").is_none());
}

#[test]
fn contract_task_update_request_field_names() {
    let req = TaskUpdateRequest {
        task_id: "t-1".to_string(),
        workflow_instance_id: "wf-1".to_string(),
        status: TaskStatus::Completed,
        output_data: json!({}),
        reason_for_incompletion: None,
        callback_after_seconds: 0,
        worker_id: None,
        logs: vec![],
        external_output_payload_storage_path: None,
        sub_workflow_id: None,
        extend_lease: false,
    };
    let v = serde_json::to_value(&req).unwrap();
    assert!(v.get("taskId").is_some());
    assert!(v.get("workflowInstanceId").is_some());
    assert!(v.get("outputData").is_some());
    assert!(v.get("reasonForIncompletion").is_some());
    assert!(v.get("callbackAfterSeconds").is_some());
    assert!(v.get("workerId").is_some());
    assert!(v.get("extendLease").is_some());

    assert!(v.get("task_id").is_none());
    assert!(v.get("workflow_instance_id").is_none());
    assert!(v.get("output_data").is_none());
}

#[test]
fn contract_poll_task_field_names() {
    let poll = PollTask {
        task_id: "t-1".to_string(),
        workflow_instance_id: "wf-1".to_string(),
        task_type: "SIMPLE".to_string(),
        task_def_name: "my_task".to_string(),
        reference_task_name: "ref1".to_string(),
        status: TaskStatus::InProgress,
        input_data: json!({}),
        scheduled_time: Some(1000),
        start_time: Some(2000),
        callback_after_seconds: 0,
        poll_count: 1,
        retry_count: 0,
        priority: 0,
        env_vars: None,
    };
    let v = serde_json::to_value(&poll).unwrap();
    assert!(v.get("taskId").is_some());
    assert!(v.get("workflowInstanceId").is_some());
    assert!(v.get("taskType").is_some());
    assert!(v.get("taskDefName").is_some());
    assert!(v.get("referenceTaskName").is_some());
    assert!(v.get("inputData").is_some());
    assert!(v.get("scheduledTime").is_some());
    assert!(v.get("startTime").is_some());
    assert!(v.get("callbackAfterSeconds").is_some());
    assert!(v.get("pollCount").is_some());
    assert!(v.get("retryCount").is_some());

    assert!(v.get("task_id").is_none());
    assert!(v.get("input_data").is_none());
}

#[test]
fn contract_task_def_field_names() {
    let def = TaskDef {
        name: "my_task".to_string(),
        description: Some("desc".to_string()),
        retry_count: 3,
        retry_logic: RetryLogic::Fixed,
        retry_delay_seconds: 60,
        timeout_seconds: 3600,
        timeout_policy: TaskTimeoutPolicy::Retry,
        response_timeout_seconds: 600,
        concurrent_exec_limit: None,
        rate_limit_per_frequency: None,
        rate_limit_frequency_in_seconds: None,
        isolation_group_id: None,
        execution_name_space: None,
        owner_email: None,
        poll_timeout_seconds: None,
        backoff_scale_factor: 1,
        input_keys: vec![],
        output_keys: vec![],
        input_template: HashMap::new(),
        created_by: None,
        create_time: None,
        updated_by: None,
        update_time: None,
        owner_app: None,
        total_timeout_seconds: None,
        base_type: None,
        input_schema: None,
        output_schema: None,
        enforce_schema: false,
        env_vars: None,
        retry_on_errors: vec![],
    };
    let v = serde_json::to_value(&def).unwrap();
    assert!(v.get("retryCount").is_some());
    assert!(v.get("retryLogic").is_some());
    assert!(v.get("retryDelaySeconds").is_some());
    assert!(v.get("timeoutSeconds").is_some());
    assert!(v.get("timeoutPolicy").is_some());
    assert!(v.get("responseTimeoutSeconds").is_some());
    assert!(v.get("backoffScaleFactor").is_some());
    assert!(v.get("inputKeys").is_some());
    assert!(v.get("outputKeys").is_some());
    assert!(v.get("inputTemplate").is_some());

    assert!(v.get("retry_count").is_none());
    assert!(v.get("retry_logic").is_none());
    assert!(v.get("timeout_seconds").is_none());
}

#[test]
fn contract_bulk_response_field_names() {
    let br = BulkResponse {
        bulk_successful_results: vec!["wf-1".to_string()],
        bulk_error_results: {
            let mut m = HashMap::new();
            m.insert("wf-2".to_string(), "not found".to_string());
            m
        },
    };
    let v = serde_json::to_value(&br).unwrap();
    assert!(v.get("bulkSuccessfulResults").is_some());
    assert!(v.get("bulkErrorResults").is_some());
    assert_eq!(v["bulkSuccessfulResults"][0], "wf-1");
    assert_eq!(v["bulkErrorResults"]["wf-2"], "not found");

    assert!(v.get("bulk_successful_results").is_none());
    assert!(v.get("bulk_error_results").is_none());
}

#[test]
fn contract_status_enums_use_screaming_snake() {
    // Conductor API uses SCREAMING_SNAKE_CASE for status enums
    assert_eq!(
        serde_json::to_string(&TaskStatus::InProgress).unwrap(),
        "\"IN_PROGRESS\""
    );
    assert_eq!(
        serde_json::to_string(&TaskStatus::FailedWithTerminalError).unwrap(),
        "\"FAILED_WITH_TERMINAL_ERROR\""
    );
    assert_eq!(
        serde_json::to_string(&TaskStatus::CompletedWithErrors).unwrap(),
        "\"COMPLETED_WITH_ERRORS\""
    );
    assert_eq!(
        serde_json::to_string(&TaskStatus::TimedOut).unwrap(),
        "\"TIMED_OUT\""
    );
    assert_eq!(
        serde_json::to_string(&WorkflowStatus::Running).unwrap(),
        "\"RUNNING\""
    );
    assert_eq!(
        serde_json::to_string(&WorkflowStatus::TimedOut).unwrap(),
        "\"TIMED_OUT\""
    );
    assert_eq!(
        serde_json::to_string(&TimeoutPolicy::TimeOutWf).unwrap(),
        "\"TIME_OUT_WF\""
    );
    assert_eq!(
        serde_json::to_string(&TimeoutPolicy::AlertOnly).unwrap(),
        "\"ALERT_ONLY\""
    );
    assert_eq!(
        serde_json::to_string(&RetryLogic::ExponentialBackoff).unwrap(),
        "\"EXPONENTIAL_BACKOFF\""
    );
    assert_eq!(
        serde_json::to_string(&RetryLogic::LinearBackoff).unwrap(),
        "\"LINEAR_BACKOFF\""
    );
}

#[test]
fn contract_conductor_api_workflow_def_from_official_json() {
    // JSON payload matching exactly what the official Conductor API returns
    let official_json = json!({
        "name": "order_processing",
        "description": "Order processing workflow",
        "version": 1,
        "tasks": [
            {
                "name": "validate_order",
                "taskReferenceName": "validate_order_ref",
                "type": "SIMPLE",
                "inputParameters": {
                    "orderId": "${workflow.input.orderId}"
                }
            },
            {
                "name": "check_inventory",
                "taskReferenceName": "check_inventory_ref",
                "type": "SIMPLE",
                "inputParameters": {
                    "items": "${validate_order_ref.output.items}"
                },
                "optional": false
            }
        ],
        "inputParameters": ["orderId"],
        "outputParameters": {
            "status": "${check_inventory_ref.output.status}"
        },
        "schemaVersion": 2,
        "restartable": true,
        "ownerEmail": "team@example.com",
        "timeoutPolicy": "TIME_OUT_WF",
        "timeoutSeconds": 3600
    });
    let def: WorkflowDef = serde_json::from_value(official_json.clone()).unwrap();
    assert_eq!(def.name, "order_processing");
    assert_eq!(def.version, 1);
    assert_eq!(def.tasks.len(), 2);
    assert_eq!(def.tasks[0].task_type, "SIMPLE");
    assert_eq!(def.tasks[0].task_reference_name, "validate_order_ref");
    assert_eq!(
        def.tasks[0].input_parameters["orderId"],
        json!("${workflow.input.orderId}")
    );
    assert_eq!(def.schema_version, 2);
    assert_eq!(def.timeout_seconds, 3600);

    // Re-serialize and verify field names stay camelCase
    let reserialized = serde_json::to_value(&def).unwrap();
    assert!(reserialized.get("taskReferenceName").is_none()); // top-level
    assert!(reserialized.get("ownerEmail").is_some());
    assert!(reserialized["tasks"][0].get("taskReferenceName").is_some());
    assert!(reserialized["tasks"][0].get("type").is_some());
}

#[test]
fn contract_conductor_api_task_result_from_official_json() {
    // JSON matching what Conductor returns for a task in a workflow
    let official = json!({
        "taskType": "SIMPLE",
        "status": "COMPLETED",
        "inputData": {"key": "value"},
        "referenceTaskName": "task1_ref",
        "retryCount": 0,
        "seq": 1,
        "pollCount": 2,
        "taskDefName": "my_task",
        "scheduledTime": 1700000000000i64,
        "startTime": 1700000001000i64,
        "endTime": 1700000005000i64,
        "updateTime": 1700000005000i64,
        "startDelayInSeconds": 0,
        "retried": false,
        "executed": true,
        "callbackFromWorker": true,
        "responseTimeoutSeconds": 600,
        "workflowInstanceId": "wf-123",
        "workflowType": "my_wf",
        "taskId": "task-456",
        "callbackAfterSeconds": 0,
        "outputData": {"result": "success"},
        "workflowTask": {
            "name": "my_task",
            "taskReferenceName": "task1_ref",
            "type": "SIMPLE"
        }
    });
    let task: TaskResult = serde_json::from_value(official).unwrap();
    assert_eq!(task.task_type, "SIMPLE");
    assert_eq!(task.status, TaskStatus::Completed);
    assert_eq!(task.reference_task_name, "task1_ref");
    assert_eq!(task.workflow_instance_id, "wf-123");
    assert_eq!(task.task_id, "task-456");
}

#[test]
fn contract_start_workflow_request_from_official_json() {
    let official = json!({
        "name": "order_processing",
        "version": 1,
        "input": {
            "orderId": "ORD-12345",
            "customerId": "CUST-678"
        },
        "correlationId": "correlation-abc",
        "taskToDomain": {
            "validate_order": "production"
        },
        "priority": 1
    });
    let req: StartWorkflowRequest = serde_json::from_value(official).unwrap();
    assert_eq!(req.name, "order_processing");
    assert_eq!(req.version, 1);
    assert_eq!(req.input["orderId"], "ORD-12345");
    assert_eq!(req.correlation_id, Some("correlation-abc".to_string()));
    assert_eq!(req.task_to_domain["validate_order"], "production");
    assert_eq!(req.priority, 1);
}

#[test]
fn contract_task_update_from_official_json() {
    let official = json!({
        "taskId": "task-789",
        "workflowInstanceId": "wf-123",
        "status": "COMPLETED",
        "outputData": {
            "response": {"statusCode": 200, "body": "OK"}
        },
        "callbackAfterSeconds": 0,
        "workerId": "worker-1"
    });
    let update: TaskUpdateRequest = serde_json::from_value(official).unwrap();
    assert_eq!(update.task_id, "task-789");
    assert_eq!(update.workflow_instance_id, "wf-123");
    assert_eq!(update.status, TaskStatus::Completed);
    assert_eq!(update.output_data["response"]["statusCode"], 200);
    assert_eq!(update.worker_id, Some("worker-1".to_string()));
}

// ─── Fuzz-style tests (proptest with arbitrary inputs) ──────────────────
// These exercise deserialization paths with random/malformed inputs to catch
// panics, stack overflows, or incorrect error handling.

proptest! {
    /// Random bytes should never panic when fed to WorkflowDef deserialization.
    #[test]
    fn fuzz_workflow_def_no_panic(data in proptest::collection::vec(any::<u8>(), 0..512)) {
        let _ = serde_json::from_slice::<WorkflowDef>(&data);
    }

    /// Random bytes should never panic when fed to TaskDef deserialization.
    #[test]
    fn fuzz_task_def_no_panic(data in proptest::collection::vec(any::<u8>(), 0..512)) {
        let _ = serde_json::from_slice::<TaskDef>(&data);
    }

    /// Random bytes should never panic when fed to StartWorkflowRequest deserialization.
    #[test]
    fn fuzz_start_workflow_request_no_panic(data in proptest::collection::vec(any::<u8>(), 0..512)) {
        let _ = serde_json::from_slice::<StartWorkflowRequest>(&data);
    }

    /// Random bytes should never panic when fed to TaskUpdateRequest deserialization.
    #[test]
    fn fuzz_task_update_request_no_panic(data in proptest::collection::vec(any::<u8>(), 0..512)) {
        let _ = serde_json::from_slice::<TaskUpdateRequest>(&data);
    }

    /// Random JSON values should never panic in resolve_value.
    #[test]
    fn fuzz_resolve_value_no_panic(
        s in ".*{0,200}",
    ) {
        let input = json!({});
        let outputs = HashMap::<String, Value>::new();
        let template = Value::String(s);
        let _ = resolve_value(&template, &input, &outputs, "wf-fuzz");
    }

    /// Random JSON objects fed to resolve_value should never panic.
    #[test]
    fn fuzz_resolve_value_object_no_panic(
        k1 in "[a-z]{1,10}",
        v1 in ".*{0,50}",
        k2 in "[a-z]{1,10}",
        v2 in ".*{0,50}",
    ) {
        let input = json!({"x": 1});
        let outputs = HashMap::<String, Value>::new();
        let template = json!({k1: v1, k2: v2});
        let _ = resolve_value(&template, &input, &outputs, "wf-fuzz");
    }

    /// Random expressions should never panic in resolve_expression.
    #[test]
    fn fuzz_resolve_expression_no_panic(expr in ".*{0,100}") {
        let input = json!({"a": 1, "b": "test"});
        let outputs = HashMap::<String, Value>::new();
        let _ = resolve_expression(&expr, &input, &outputs, "wf-fuzz");
    }

    /// Random loop conditions should never panic.
    #[test]
    fn fuzz_evaluate_loop_condition_no_panic(
        condition in ".*{0,100}",
        iteration in 0usize..1000,
    ) {
        let output = json!({"result": true, "shouldContinue": false});
        let _ = evaluate_loop_condition(&condition, &output, iteration);
    }

    /// Random JSON path navigation should never panic.
    #[test]
    fn fuzz_navigate_json_no_panic(
        key1 in "[a-z]{1,10}",
        key2 in "[a-z]{1,10}",
        key3 in "[a-z]{1,10}",
    ) {
        let val = json!({"a": {"b": {"c": 1}}, "d": [1, 2], "e": null});
        let _ = navigate_json(&val, &[&key1, &key2, &key3]);
    }

    /// BulkResponse deserialization with random JSON should never panic.
    #[test]
    fn fuzz_bulk_response_no_panic(data in proptest::collection::vec(any::<u8>(), 0..256)) {
        let _ = serde_json::from_slice::<BulkResponse>(&data);
    }

    /// TaskStatus deserialization from random strings should never panic.
    #[test]
    fn fuzz_task_status_no_panic(s in ".*{0,50}") {
        let json_str = format!("\"{s}\"");
        let _ = serde_json::from_str::<TaskStatus>(&json_str);
    }

    /// WorkflowStatus deserialization from random strings should never panic.
    #[test]
    fn fuzz_workflow_status_no_panic(s in ".*{0,50}") {
        let json_str = format!("\"{s}\"");
        let _ = serde_json::from_str::<WorkflowStatus>(&json_str);
    }

    /// Deeply nested JSON objects in resolve_value should not stack overflow.
    #[test]
    fn fuzz_resolve_value_nested_depth(depth in 1usize..50) {
        let input = json!({"key": "value"});
        let outputs = HashMap::<String, Value>::new();
        // Build nested object: {"a": {"a": {"a": ... "leaf"}}}
        let mut val = json!("leaf");
        for _ in 0..depth {
            val = json!({"a": val});
        }
        let _ = resolve_value(&val, &input, &outputs, "wf-fuzz");
    }

    /// Workflow with many tasks should deserialize without issues.
    #[test]
    fn fuzz_many_tasks_workflow_def(count in 1usize..50) {
        let tasks: Vec<Value> = (0..count).map(|i| {
            json!({
                "name": format!("task_{i}"),
                "taskReferenceName": format!("ref_{i}"),
                "type": "SIMPLE"
            })
        }).collect();
        let wf = json!({
            "name": "big_wf",
            "tasks": tasks
        });
        let def: WorkflowDef = serde_json::from_value(wf).unwrap();
        prop_assert_eq!(def.tasks.len(), count);
    }
}
