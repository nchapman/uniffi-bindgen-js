use wasm_bindgen::prelude::*;

/// Returns `a / b`, or throws "DivisionByZero" if `b` is zero.
#[wasm_bindgen]
pub fn divide(a: f64, b: f64) -> Result<f64, JsValue> {
    if b == 0.0 {
        Err(JsValue::from_str("DivisionByZero"))
    } else {
        Ok(a / b)
    }
}

/// Returns `sqrt(x)`, or throws "NegativeSquareRoot" if `x` is negative.
#[wasm_bindgen]
pub fn sqrt(x: f64) -> Result<f64, JsValue> {
    if x < 0.0 {
        Err(JsValue::from_str("NegativeSquareRoot"))
    } else {
        Ok(x.sqrt())
    }
}
