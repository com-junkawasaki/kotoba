use wasm_bindgen::prelude::*;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

/// Compiles KotobaScript code to TSX code.
#[wasm_bindgen]
pub fn compile(code: &str) -> Result<String, JsValue> {
    // Temporarily disabled the actual compilation logic to isolate build issues.
    // This will just return a placeholder string.
    Ok(format!("/* Compiled from KotobaScript (length: {}): */", code.len()))
}
