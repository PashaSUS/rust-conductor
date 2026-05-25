//! Template expression resolution (`${...}` substitution), loop condition
//! evaluation, and composite condition tree evaluation.

use std::collections::HashMap;

use serde_json::Value;

use crate::models::common::{CompareOp, ConditionNode};

// ── Template expression resolution ──────────────────────────────────────────

/// Recursively resolve `${...}` template expressions in a JSON value.
///
/// Supported expressions:
///   ${workflow.input.field.path}  — value from the workflow input
///   ${workflow.workflowId}        — the workflow's ID
///   ${refName.output.field.path}  — output of a completed task by reference name
pub(crate) fn resolve_value(
    val: &Value,
    workflow_input: &Value,
    task_outputs: &HashMap<String, Value>,
    workflow_id: &str,
) -> Value {
    match val {
        Value::String(s) => resolve_string_value(s, workflow_input, task_outputs, workflow_id),
        Value::Object(map) => {
            let resolved: serde_json::Map<String, Value> = map
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        resolve_value(v, workflow_input, task_outputs, workflow_id),
                    )
                })
                .collect();
            Value::Object(resolved)
        }
        Value::Array(arr) => Value::Array(
            arr.iter()
                .map(|v| resolve_value(v, workflow_input, task_outputs, workflow_id))
                .collect(),
        ),
        other => other.clone(),
    }
}

pub(crate) fn resolve_string_value(
    s: &str,
    workflow_input: &Value,
    task_outputs: &HashMap<String, Value>,
    workflow_id: &str,
) -> Value {
    // If the entire string is a single ${...} expression, preserve the resolved type
    let trimmed = s.trim();
    if trimmed.starts_with("${") && trimmed.ends_with('}') {
        let inner = &trimmed[2..trimmed.len() - 1];
        if !inner.contains("${")
            && let Some(resolved) =
                resolve_expression(inner, workflow_input, task_outputs, workflow_id)
        {
            return resolved;
        }
    }

    if !s.contains("${") {
        return Value::String(s.to_string());
    }

    // String interpolation for mixed content like "prefix-${workflow.input.id}-suffix"
    let mut result = String::new();
    let mut remaining = s;

    while let Some(start) = remaining.find("${") {
        result.push_str(&remaining[..start]);
        let after = &remaining[start + 2..];
        if let Some(end) = after.find('}') {
            let expr = &after[..end];
            match resolve_expression(expr, workflow_input, task_outputs, workflow_id) {
                Some(Value::String(v)) => result.push_str(&v),
                Some(Value::Null) => result.push_str("null"),
                Some(v) => result.push_str(&v.to_string()),
                None => {
                    result.push_str("${");
                    result.push_str(expr);
                    result.push('}');
                }
            }
            remaining = &after[end + 1..];
        } else {
            result.push_str(&remaining[start..]);
            remaining = "";
        }
    }
    result.push_str(remaining);
    Value::String(result)
}

pub(crate) fn resolve_expression(
    expr: &str,
    workflow_input: &Value,
    task_outputs: &HashMap<String, Value>,
    _workflow_id: &str,
) -> Option<Value> {
    let parts: Vec<&str> = expr.split('.').collect();
    if parts.len() < 2 {
        return None;
    }

    if parts[0] == "workflow" {
        return match parts[1] {
            "workflowId" => Some(Value::String(_workflow_id.to_string())),
            "input" => {
                if parts.len() == 2 {
                    Some(workflow_input.clone())
                } else {
                    navigate_json(workflow_input, &parts[2..])
                }
            }
            _ => None,
        };
    }

    // Task reference: refName.output.field.path
    if parts[1] == "output"
        && let Some(output) = task_outputs.get(parts[0])
    {
        if parts.len() == 2 {
            return Some(output.clone());
        }
        return navigate_json(output, &parts[2..]);
    }

    None
}

pub(crate) fn navigate_json(val: &Value, path: &[&str]) -> Option<Value> {
    let mut current = val;
    for &segment in path {
        current = current.get(segment)?;
    }
    Some(current.clone())
}

// ── Loop condition evaluation ───────────────────────────────────────────────

/// Evaluate a DO_WHILE loop condition. Supports:
///   - `"iteration < N"` — continue while iteration count is below N
///   - `"true"` / `"false"` — literal
///   - Otherwise: check if the last task output's `result` field is truthy
pub(crate) fn evaluate_loop_condition(
    condition: &str,
    last_output: &Value,
    iteration: usize,
) -> bool {
    let trimmed = condition.trim();

    if trimmed == "false" || trimmed.is_empty() {
        return false;
    }
    if trimmed == "true" {
        return true;
    }

    // Simple "iteration < N" pattern
    if let Some(rest) = trimmed.strip_prefix("iteration") {
        let rest = rest.trim();
        if let Some(n_str) = rest.strip_prefix('<')
            && let Ok(n) = n_str.trim().parse::<usize>()
        {
            return iteration < n;
        }
        if let Some(n_str) = rest.strip_prefix("<=")
            && let Ok(n) = n_str.trim().parse::<usize>()
        {
            return iteration <= n;
        }
    }

    // Check last task output for a truthy "result" field
    match last_output
        .get("shouldContinue")
        .or_else(|| last_output.get("result"))
    {
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => s != "false" && !s.is_empty(),
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0) != 0.0,
        Some(Value::Null) => false,
        _ => false,
    }
}

// ── Composite condition tree evaluation ─────────────────────────────────────

/// Recursively evaluate a `ConditionNode` tree against workflow/task input.
pub(crate) fn evaluate_condition_tree(node: &ConditionNode, input: &Value) -> bool {
    match node {
        ConditionNode::And(children) => children.iter().all(|c| evaluate_condition_tree(c, input)),
        ConditionNode::Or(children) => children.iter().any(|c| evaluate_condition_tree(c, input)),
        ConditionNode::Not(child) => !evaluate_condition_tree(child, input),
        ConditionNode::Compare { field, op, value } => {
            let actual = navigate_json_path(input, field);
            compare_values(&actual, op, value)
        }
    }
}

/// Navigate a dot-separated path in a JSON value.
fn navigate_json_path(val: &Value, path: &str) -> Value {
    let mut current = val;
    for segment in path.split('.') {
        match current.get(segment) {
            Some(v) => current = v,
            None => return Value::Null,
        }
    }
    current.clone()
}

/// Compare an actual JSON value against an expected value using a `CompareOp`.
fn compare_values(actual: &Value, op: &CompareOp, expected: &Value) -> bool {
    match op {
        CompareOp::Eq => json_values_equal(actual, expected),
        CompareOp::Neq => !json_values_equal(actual, expected),
        CompareOp::Gt => json_numeric_cmp(actual, expected).is_some_and(|o| o.is_gt()),
        CompareOp::Gte => json_numeric_cmp(actual, expected).is_some_and(|o| o.is_ge()),
        CompareOp::Lt => json_numeric_cmp(actual, expected).is_some_and(|o| o.is_lt()),
        CompareOp::Lte => json_numeric_cmp(actual, expected).is_some_and(|o| o.is_le()),
        CompareOp::Contains => {
            let haystack = value_to_string(actual);
            let needle = value_to_string(expected);
            haystack.contains(&needle)
        }
        CompareOp::StartsWith => {
            let haystack = value_to_string(actual);
            let needle = value_to_string(expected);
            haystack.starts_with(&needle)
        }
        CompareOp::EndsWith => {
            let haystack = value_to_string(actual);
            let needle = value_to_string(expected);
            haystack.ends_with(&needle)
        }
    }
}

fn json_values_equal(a: &Value, b: &Value) -> bool {
    // Loose equality: compare as strings if types differ
    if std::mem::discriminant(a) == std::mem::discriminant(b) {
        a == b
    } else {
        value_to_string(a) == value_to_string(b)
    }
}

fn json_numeric_cmp(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    let a_f = a
        .as_f64()
        .or_else(|| value_to_string(a).parse::<f64>().ok())?;
    let b_f = b
        .as_f64()
        .or_else(|| value_to_string(b).parse::<f64>().ok())?;
    a_f.partial_cmp(&b_f)
}

fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}
