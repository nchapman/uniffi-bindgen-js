use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Namespace function named `class` — a JS reserved word.
/// UDL: u32 class(u32 switch);
#[wasm_bindgen(js_name = "class")]
pub fn class_(switch_: u32) -> u32 {
    switch_.wrapping_add(1)
}

/// Namespace function named `return` — a JS reserved word.
/// UDL: string return(string var);
#[wasm_bindgen(js_name = "return")]
pub fn return_(var_: &str) -> String {
    format!("returned: {var_}")
}

/// Namespace function named `delete` — a JS reserved word.
/// UDL: boolean delete(boolean static);
#[wasm_bindgen(js_name = "delete")]
pub fn delete_(static_: bool) -> bool {
    !static_
}

/// Object whose methods use reserved-word names.
#[wasm_bindgen]
pub struct SuperWidget {
    counter: u32,
}

impl Default for SuperWidget {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl SuperWidget {
    #[wasm_bindgen(constructor)]
    pub fn new() -> SuperWidget {
        SuperWidget { counter: 0 }
    }

    /// Method named `class` — a JS reserved word.
    #[wasm_bindgen(js_name = "class")]
    pub fn class_(&mut self, var_: &str) -> String {
        self.counter += 1;
        format!("widget({}):{var_}", self.counter)
    }

    /// Method named `return` — a JS reserved word.
    #[wasm_bindgen(js_name = "return")]
    pub fn return_(&self) -> u32 {
        self.counter
    }
}

// --- ReturnValue record (reserved-word field names) ---

/// Mirrors the UDL dictionary ReturnValue with fields: class, return, typeof.
/// Field names use the escaped suffixed form that the generated TS expects.
#[derive(Serialize)]
pub struct ReturnValue {
    pub class_: String,
    pub return_: u32,
    pub typeof_: bool,
}

/// Creates a ReturnValue with the given fields.
#[wasm_bindgen]
pub fn make_return_value(class_: &str, return_: u32, typeof_: bool) -> JsValue {
    let rv = ReturnValue {
        class_: class_.to_string(),
        return_,
        typeof_,
    };
    serde_wasm_bindgen::to_value(&rv).unwrap()
}

// --- AsyncKind flat enum (reserved-word variant names) ---

/// Mirrors the UDL enum AsyncKind with variants: void, yield, await.
/// Flat enums are serialised as plain strings.
#[derive(Serialize, Deserialize)]
pub enum AsyncKind {
    #[serde(rename = "void")]
    Void,
    #[serde(rename = "yield")]
    Yield,
    #[serde(rename = "await")]
    Await,
}

/// Echoes an AsyncKind value back (round-trip through serde).
#[wasm_bindgen]
pub fn echo_async_kind(kind: JsValue) -> Result<JsValue, JsValue> {
    let k: AsyncKind =
        serde_wasm_bindgen::from_value(kind).map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_wasm_bindgen::to_value(&k).map_err(|e| JsValue::from_str(&e.to_string()))
}

// --- ThrowKind error enum (reserved-word variant names) ---

/// Mirrors the UDL [Error] enum ThrowKind with variants: catch, finally.
/// Serialised as a tagged object for the generated TS error class to parse.
#[derive(Serialize)]
#[serde(tag = "tag")]
pub enum ThrowKind {
    #[serde(rename = "catch")]
    Catch,
    #[serde(rename = "finally")]
    Finally,
}

/// Throws a ThrowKind error with the given variant tag.
/// Pass "catch" or "finally" to select the variant.
#[wasm_bindgen]
pub fn throw_kind(tag: &str) -> Result<(), JsValue> {
    let err = match tag {
        "catch" => ThrowKind::Catch,
        "finally" => ThrowKind::Finally,
        _ => return Err(JsValue::from_str(&format!("unknown ThrowKind tag: {tag}"))),
    };
    Err(JsValue::from_str(&serde_json::to_string(&err).unwrap()))
}
