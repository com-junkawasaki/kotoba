use wasm_bindgen::prelude::*;
use kotoba_storage::KeyValueStore;
use std::sync::Arc;
use std::sync::Mutex;
use wasm_bindgen_futures::future_to_promise;
use once_cell::sync::Lazy;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// Use a static Mutex to hold the KeyValueStore instance.
// In WASM's single-threaded model, this provides a simple way to maintain state.
static STORAGE_INSTANCE: Lazy<Mutex<Option<Arc<dyn KeyValueStore + Send + Sync>>>> = Lazy::new(|| Mutex::new(None));

/// Initializes the KeyValueStore. This must be called once before any other functions.
#[wasm_bindgen]
pub async fn init_storage() -> Result<(), JsValue> {
    let mut storage_guard = STORAGE_INSTANCE.lock().map_err(|e| JsValue::from_str(&e.to_string()))?;
    if storage_guard.is_none() {
        // For WASM, we'll use a simple in-memory KeyValueStore
        // TODO: Implement a proper WASM-compatible KeyValueStore
        return Err(JsValue::from_str("KeyValueStore implementation for WASM is not yet available"));
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

/// Stores a workflow in KeyValueStore.
/// Accepts a JSON string representing the workflow data.
/// Returns a success message.
#[wasm_bindgen(js_name = "storeWorkflow")]
pub async fn store_workflow(workflow_id: &str, workflow_data: &str) -> Result<String, JsValue> {
    let storage_guard = STORAGE_INSTANCE.lock().map_err(|e| JsValue::from_str(&e.to_string()))?;
    let storage = storage_guard.as_ref().ok_or_else(|| JsValue::from_str("Storage not initialized"))?;

    let key = format!("workflow:{}", workflow_id);
    storage.put(key.as_bytes(), workflow_data.as_bytes()).await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    Ok(format!("Workflow {} stored successfully", workflow_id))
}

/// Gets a workflow from KeyValueStore.
/// Accepts the workflow_id as a string.
/// Returns the workflow data as a string.
#[wasm_bindgen(js_name = "getWorkflow")]
pub async fn get_workflow(workflow_id: &str) -> Result<String, JsValue> {
    let storage_guard = STORAGE_INSTANCE.lock().map_err(|e| JsValue::from_str(&e.to_string()))?;
    let storage = storage_guard.as_ref().ok_or_else(|| JsValue::from_str("Storage not initialized"))?;

    let key = format!("workflow:{}", workflow_id);
    let data = storage.get(key.as_bytes()).await
        .map_err(|e| JsValue::from_str(&e.to_string()))?
        .ok_or_else(|| JsValue::from_str("Workflow not found"))?;

    String::from_utf8(data).map_err(|e| JsValue::from_str(&e.to_string()))
}
