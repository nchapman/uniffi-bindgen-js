use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraitRecord {
    pub name: String,
    pub value: u32,
}

impl std::fmt::Display for TraitRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TraitRecord(name={}, value={})", self.name, self.value)
    }
}

#[wasm_bindgen]
pub fn trait_record_uniffi_trait_display(val: JsValue) -> Result<String, JsValue> {
    let rec: TraitRecord =
        serde_wasm_bindgen::from_value(val).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(rec.to_string())
}

#[wasm_bindgen]
pub fn trait_record_uniffi_trait_eq_eq(a: JsValue, b: JsValue) -> Result<bool, JsValue> {
    let ra: TraitRecord =
        serde_wasm_bindgen::from_value(a).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let rb: TraitRecord =
        serde_wasm_bindgen::from_value(b).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(ra == rb)
}

#[wasm_bindgen]
pub fn trait_record_uniffi_trait_hash(val: JsValue) -> Result<u64, JsValue> {
    let rec: TraitRecord =
        serde_wasm_bindgen::from_value(val).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let mut hasher = DefaultHasher::new();
    rec.hash(&mut hasher);
    Ok(hasher.finish())
}

/// wasm-bindgen cannot expose `Arc<dyn Trait>` directly, so we use a concrete
/// wrapper struct that holds the variant tag and dimensions.
#[wasm_bindgen]
pub struct Drawable {
    kind: DrawableKind,
}

enum DrawableKind {
    Circle { radius: f64 },
    Rect { width: f64, height: f64 },
}

#[wasm_bindgen]
impl Drawable {
    pub fn describe(&self) -> String {
        match &self.kind {
            DrawableKind::Circle { radius } => format!("circle(r={radius})"),
            DrawableKind::Rect { width, height } => format!("rect({width}x{height})"),
        }
    }

    pub fn area(&self) -> f64 {
        match &self.kind {
            DrawableKind::Circle { radius } => std::f64::consts::PI * radius * radius,
            DrawableKind::Rect { width, height } => width * height,
        }
    }
}

#[wasm_bindgen]
pub fn make_circle(radius: f64) -> Drawable {
    Drawable {
        kind: DrawableKind::Circle { radius },
    }
}

#[wasm_bindgen]
pub fn make_rect(width: f64, height: f64) -> Drawable {
    Drawable {
        kind: DrawableKind::Rect { width, height },
    }
}
