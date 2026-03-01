use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn hello(name: &str) -> String {
    format!("hello, {name}")
}

#[wasm_bindgen]
pub fn farewell(name: &str) -> String {
    format!("goodbye, {name}")
}

#[wasm_bindgen]
pub fn version() -> String {
    "1.0.0".to_string()
}
