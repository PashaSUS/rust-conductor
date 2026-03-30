//! Bulk workflow operations — pause, resume, retry, restart, terminate multiple
//! workflows concurrently and collect per-workflow success/failure results.

use std::collections::HashMap;

use futures::future::join_all;

use super::WorkflowEngine;
use super::error::EngineError;
use crate::models::*;

impl WorkflowEngine {
    // ── Bulk Operations ──

    pub async fn bulk_pause(&self, workflow_ids: &[String]) -> Result<BulkResponse, EngineError> {
        let futs: Vec<_> = workflow_ids
            .iter()
            .map(|id| {
                let id = id.clone();
                async move { (id.clone(), self.pause_workflow(&id).await) }
            })
            .collect();
        Ok(collect_bulk_results(join_all(futs).await))
    }

    pub async fn bulk_resume(&self, workflow_ids: &[String]) -> Result<BulkResponse, EngineError> {
        let futs: Vec<_> = workflow_ids
            .iter()
            .map(|id| {
                let id = id.clone();
                async move { (id.clone(), self.resume_workflow(&id).await) }
            })
            .collect();
        Ok(collect_bulk_results(join_all(futs).await))
    }

    pub async fn bulk_retry(&self, workflow_ids: &[String]) -> Result<BulkResponse, EngineError> {
        let futs: Vec<_> = workflow_ids
            .iter()
            .map(|id| {
                let id = id.clone();
                async move {
                    let result = self.retry_workflow(&id).await.map(|_| ());
                    (id, result)
                }
            })
            .collect();
        Ok(collect_bulk_results(join_all(futs).await))
    }

    pub async fn bulk_restart(&self, workflow_ids: &[String]) -> Result<BulkResponse, EngineError> {
        let futs: Vec<_> = workflow_ids
            .iter()
            .map(|id| {
                let id = id.clone();
                async move {
                    let result = self.restart_workflow(&id).await.map(|_| ());
                    (id, result)
                }
            })
            .collect();
        Ok(collect_bulk_results(join_all(futs).await))
    }

    pub async fn bulk_terminate(
        &self,
        workflow_ids: &[String],
        reason: Option<&str>,
    ) -> Result<BulkResponse, EngineError> {
        let futs: Vec<_> = workflow_ids
            .iter()
            .map(|id| {
                let id = id.clone();
                async move { (id.clone(), self.terminate_workflow(&id, reason).await) }
            })
            .collect();
        Ok(collect_bulk_results(join_all(futs).await))
    }
}

fn collect_bulk_results(results: Vec<(String, Result<(), EngineError>)>) -> BulkResponse {
    let mut successful = vec![];
    let mut errors = HashMap::new();
    for (id, result) in results {
        match result {
            Ok(()) => successful.push(id),
            Err(e) => {
                errors.insert(id, e.to_string());
            }
        }
    }
    BulkResponse {
        bulk_successful_results: successful,
        bulk_error_results: errors,
    }
}
