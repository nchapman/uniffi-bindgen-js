use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "tag")]
pub enum Shape {
    Circle { radius: f64 },
    Rectangle { width: f64, height: f64 },
    Point,
}

/// Matches the flat enum Direction from the UDL (serialised as a string tag).
#[derive(Deserialize)]
pub enum Direction {
    North,
    South,
    East,
    West,
}

/// Translate a point by (dx, dy).
#[wasm_bindgen]
pub fn translate(p: JsValue, dx: f64, dy: f64) -> Result<JsValue, JsValue> {
    let mut point: Point =
        serde_wasm_bindgen::from_value(p).map_err(|e| JsValue::from_str(&e.to_string()))?;
    point.x += dx;
    point.y += dy;
    serde_wasm_bindgen::to_value(&point).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Move a point one unit in the given direction.
#[wasm_bindgen]
pub fn step(p: JsValue, d: JsValue) -> Result<JsValue, JsValue> {
    let mut point: Point =
        serde_wasm_bindgen::from_value(p).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let dir: Direction =
        serde_wasm_bindgen::from_value(d).map_err(|e| JsValue::from_str(&e.to_string()))?;
    match dir {
        Direction::North => point.y += 1.0,
        Direction::South => point.y -= 1.0,
        Direction::East => point.x += 1.0,
        Direction::West => point.x -= 1.0,
    }
    serde_wasm_bindgen::to_value(&point).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Compute the area of a shape.
#[wasm_bindgen]
pub fn area(shape: JsValue) -> Result<f64, JsValue> {
    let s: Shape =
        serde_wasm_bindgen::from_value(shape).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let result = match s {
        Shape::Circle { radius } => std::f64::consts::PI * radius * radius,
        Shape::Rectangle { width, height } => width * height,
        Shape::Point => 0.0,
    };
    Ok(result)
}
