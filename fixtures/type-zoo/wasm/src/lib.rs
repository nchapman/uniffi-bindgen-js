use js_sys::{Date, Uint8Array};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

// --- Primitive echo functions ---

#[wasm_bindgen]
pub fn echo_i8(v: i8) -> i8 {
    v
}

#[wasm_bindgen]
pub fn echo_i16(v: i16) -> i16 {
    v
}

#[wasm_bindgen]
pub fn echo_i32(v: i32) -> i32 {
    v
}

#[wasm_bindgen]
pub fn echo_i64(v: i64) -> i64 {
    v
}

#[wasm_bindgen]
pub fn echo_u8(v: u8) -> u8 {
    v
}

#[wasm_bindgen]
pub fn echo_u16(v: u16) -> u16 {
    v
}

#[wasm_bindgen]
pub fn echo_u32(v: u32) -> u32 {
    v
}

#[wasm_bindgen]
pub fn echo_u64(v: u64) -> u64 {
    v
}

#[wasm_bindgen]
pub fn echo_f32(v: f32) -> f32 {
    v
}

#[wasm_bindgen]
pub fn echo_f64(v: f64) -> f64 {
    v
}

#[wasm_bindgen]
pub fn echo_bool(v: bool) -> bool {
    v
}

#[wasm_bindgen]
pub fn echo_string(v: &str) -> String {
    v.to_string()
}

// --- Bytes ---

#[wasm_bindgen]
pub fn echo_bytes(data: &[u8]) -> Uint8Array {
    Uint8Array::from(data)
}

// --- Duration (seconds as f64) ---

#[wasm_bindgen]
pub fn echo_duration(d: f64) -> f64 {
    d
}

// --- Timestamp (JS Date milliseconds round-trip) ---

#[wasm_bindgen]
pub fn echo_timestamp(t: &Date) -> Date {
    Date::new(&JsValue::from_f64(t.get_time()))
}

// --- Optional string ---

#[wasm_bindgen]
pub fn maybe_string(input: Option<String>) -> Option<String> {
    input
}

// --- Sequences and maps (via serde) ---

#[wasm_bindgen]
pub fn echo_strings(items: JsValue) -> Result<JsValue, JsValue> {
    let v: Vec<String> =
        serde_wasm_bindgen::from_value(items).map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_wasm_bindgen::to_value(&v).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn echo_map(m: JsValue) -> Result<JsValue, JsValue> {
    let map: HashMap<String, i32> =
        serde_wasm_bindgen::from_value(m).map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_wasm_bindgen::to_value(&map).map_err(|e| JsValue::from_str(&e.to_string()))
}

// wasm-bindgen natively handles BigInt[] <-> Box<[i64]> conversion;
// serde_wasm_bindgen does NOT support JavaScript BigInt values.
#[wasm_bindgen]
pub fn echo_bigints(items: Box<[i64]>) -> Box<[i64]> {
    items
}

#[wasm_bindgen]
pub fn echo_bool_map(m: JsValue) -> Result<JsValue, JsValue> {
    let map: HashMap<String, bool> =
        serde_wasm_bindgen::from_value(m).map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_wasm_bindgen::to_value(&map).map_err(|e| JsValue::from_str(&e.to_string()))
}
