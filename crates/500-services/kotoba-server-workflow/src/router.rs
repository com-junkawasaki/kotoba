//! Workflow router configuration

use axum::Router;
use kotoba_server_core::AppRouter;
use crate::handlers::WorkflowStatusHandler;

/// Workflow router builder
#[derive(Default)]
pub struct WorkflowRouter;

impl WorkflowRouter {
    pub fn new() -> Self {
        Self
    }

    pub fn build(self) -> Router {
        let mut router = Router::new();

        // Basic workflow status endpoint (always available)
        router = router.route("/api/v1/workflow/status", axum::routing::get(WorkflowStatusHandler::health));

        // TODO: Add workflow-specific routes when workflow feature is enabled
        // For now, just provide status endpoint

        router
    }
}

impl From<WorkflowRouter> for Router {
    fn from(router: WorkflowRouter) -> Router {
        router.build()
    }
}

impl From<WorkflowRouter> for AppRouter {
    fn from(router: WorkflowRouter) -> AppRouter {
        AppRouter::new().merge(router.build())
    }
}
