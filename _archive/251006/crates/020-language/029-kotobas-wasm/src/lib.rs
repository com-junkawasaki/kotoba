use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

/// Compiles KotobaScript code to JSON.
/// This is the main entry point for the KotobaScript REPL.
#[wasm_bindgen]
pub fn compile(code: &str) -> Result<String, JsValue> {
    // Use kotoba-jsonnet to evaluate the Jsonnet code
    match kotoba_jsonnet::evaluate_to_json(code) {
        Ok(result) => Ok(result),
        Err(e) => Err(JsValue::from_str(&format!("Compilation error: {:?}", e))),
    }
}

/// Evaluates KotobaScript code and returns the result as a formatted string.
/// Useful for displaying evaluation results in the REPL.
#[wasm_bindgen]
pub fn evaluate(code: &str) -> Result<String, JsValue> {
    // Use kotoba-jsonnet to evaluate the Jsonnet code
    match kotoba_jsonnet::evaluate_to_json(code) {
        Ok(result) => {
            // Try to pretty-print JSON if it's valid JSON
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&result) {
                Ok(serde_json::to_string_pretty(&json_value)
                    .unwrap_or_else(|_| result))
            } else {
                Ok(result)
            }
        }
        Err(e) => Err(JsValue::from_str(&format!("Evaluation error: {:?}", e))),
    }
}
