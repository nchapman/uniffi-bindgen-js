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

#[wasm_bindgen]
pub fn greet_optional(name: Option<String>) -> String {
    match name {
        Some(n) => format!("hello, {n}"),
        None => "hello, stranger".to_string(),
    }
}

#[wasm_bindgen]
pub fn add_maybe(a: u32, b: Option<u32>) -> u32 {
    a + b.unwrap_or(0)
}
