use wasm_bindgen::prelude::*;
use kotoba_workflow::prelude::*;
use std::sync::Mutex;
use wasm_bindgen_futures::future_to_promise;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// Use a static Mutex to hold the engine instance.
// In a real multi-threaded server, we'd need a more robust solution,
// but for WASM's single-threaded model, this is a simple way to maintain state.
static WORKFLOW_ENGINE: once_cell::sync::Lazy<Mutex<WorkflowEngine>> = once_cell::sync::Lazy::new(|| {
    // This is async, but we can't await in a static initializer.
    // We will initialize it properly via an `init` function.
    // For now, we create a placeholder. A real implementation would need an async builder.
    // Let's assume for now the builder can be constructed synchronously for simplicity.
    // The `build` function is async, so we can't call it here directly.
    // We will use an `Option` and initialize it on first use.
    panic!("Engine should be initialized via init function");
});

// A proper way to handle async initialization.
static ENGINE_INSTANCE: once_cell::sync::Lazy<Mutex<Option<WorkflowEngine>>> = once_cell::sync::Lazy::new(|| Mutex::new(None));

/// Initializes the workflow engine. This must be called once before any other workflow functions.
#[wasm_bindgen]
pub async fn init_workflow_engine() -> Result<(), JsValue> {
    let mut engine_guard = ENGINE_INSTANCE.lock().unwrap();
    if engine_guard.is_none() {
        let engine = WorkflowEngine::builder()
            .with_memory_storage()
            .build()
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        *engine_guard = Some(engine);
    }
    Ok(())
}


/// Compiles KotobaScript code to TSX code.
#[wasm_bindgen]
pub fn compile(code: &str) -> Result<String, JsValue> {
    // Temporarily disabled the actual compilation logic to isolate build issues.
    // This will just return a placeholder string.
    Ok(format!("/* Compiled from KotobaScript (length: {}): */", code.len()))
}

/// Starts a new workflow execution.
/// Accepts a JSON string representing the WorkflowIR.
/// Returns a JSON string with the execution_id.
#[wasm_bindgen(js_name = "startWorkflow")]
pub async fn start_workflow(workflow_ir_json: &str) -> Result<String, JsValue> {
    let ir: WorkflowIR = serde_json::from_str(workflow_ir_json)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let mut engine_guard = ENGINE_INSTANCE.lock().map_err(|e| JsValue::from_str(&e.to_string()))?;
    let engine = engine_guard.as_mut().ok_or_else(|| JsValue::from_str("Engine not initialized"))?;

    let execution_id = engine.start_workflow(&ir, std::collections::HashMap::new())
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    Ok(serde_json::to_string(&serde_json::json!({ "execution_id": execution_id.0 })).unwrap())
}

/// Gets the status of a workflow execution.
/// Accepts the execution_id as a string.
/// Returns a JSON string of the WorkflowExecution details.
#[wasm_bindgen(js_name = "getWorkflowStatus")]
pub async fn get_workflow_status(execution_id: &str) -> Result<String, JsValue> {
    let exec_id = WorkflowExecutionId(execution_id.to_string());
    
    let engine_guard = ENGINE_INSTANCE.lock().map_err(|e| JsValue::from_str(&e.to_string()))?;
    let engine = engine_guard.as_ref().ok_or_else(|| JsValue::from_str("Engine not initialized"))?;

    let execution = engine.get_execution(&exec_id)
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    Ok(serde_json::to_string(&execution).unwrap())
}
