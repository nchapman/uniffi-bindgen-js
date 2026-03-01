use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("hello, {name}")
}

#[wasm_bindgen]
pub async fn greet_async(name: String) -> String {
    // Simulate an async operation (wasm_bindgen_futures drives this as a Promise).
    format!("hello, {name}")
}
