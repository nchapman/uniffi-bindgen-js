use uniffi;
use std::time::{Duration, SystemTime};

uniffi::setup_scaffolding!();

// --- Primitives ---

#[uniffi::export]
fn identity_bool(v: bool) -> bool { v }

#[uniffi::export]
fn identity_i8(v: i8) -> i8 { v }

#[uniffi::export]
fn identity_u8(v: u8) -> u8 { v }

#[uniffi::export]
fn identity_i16(v: i16) -> i16 { v }

#[uniffi::export]
fn identity_u16(v: u16) -> u16 { v }

#[uniffi::export]
fn identity_i32(v: i32) -> i32 { v }

#[uniffi::export]
fn identity_u32(v: u32) -> u32 { v }

#[uniffi::export]
fn identity_i64(v: i64) -> i64 { v }

#[uniffi::export]
fn identity_u64(v: u64) -> u64 { v }

#[uniffi::export]
fn identity_f32(v: f32) -> f32 { v }

#[uniffi::export]
fn identity_f64(v: f64) -> f64 { v }

#[uniffi::export]
fn identity_string(v: String) -> String { v }

#[uniffi::export]
fn identity_bytes(v: Vec<u8>) -> Vec<u8> { v }

// --- Optional ---

#[uniffi::export]
fn identity_optional_string(v: Option<String>) -> Option<String> { v }

#[uniffi::export]
fn identity_optional_i32(v: Option<i32>) -> Option<i32> { v }

// --- Sequence ---

#[uniffi::export]
fn identity_seq_i32(v: Vec<i32>) -> Vec<i32> { v }

#[uniffi::export]
fn identity_seq_string(v: Vec<String>) -> Vec<String> { v }

// --- Map ---

#[uniffi::export]
fn identity_map_string_i32(v: std::collections::HashMap<String, i32>) -> std::collections::HashMap<String, i32> { v }

// --- Duration & Timestamp ---

#[uniffi::export]
fn identity_duration(v: Duration) -> Duration { v }

#[uniffi::export]
fn identity_timestamp(v: SystemTime) -> SystemTime { v }

// --- Record ---

#[derive(uniffi::Record)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[uniffi::export]
fn make_point(x: f64, y: f64) -> Point {
    Point { x, y }
}

#[uniffi::export]
fn point_distance(a: Point, b: Point) -> f64 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

// --- Flat enum ---

#[derive(uniffi::Enum)]
pub enum Color {
    Red,
    Green,
    Blue,
}

#[uniffi::export]
fn color_name(c: Color) -> String {
    match c {
        Color::Red => "red".to_string(),
        Color::Green => "green".to_string(),
        Color::Blue => "blue".to_string(),
    }
}

// --- Data enum ---

#[derive(uniffi::Enum)]
pub enum Shape {
    Circle { radius: f64 },
    Rect { width: f64, height: f64 },
}

#[uniffi::export]
fn shape_area(s: Shape) -> f64 {
    match s {
        Shape::Circle { radius } => std::f64::consts::PI * radius * radius,
        Shape::Rect { width, height } => width * height,
    }
}

// --- Nested compound ---

#[uniffi::export]
fn identity_seq_point(v: Vec<Point>) -> Vec<Point> { v }

#[uniffi::export]
fn identity_map_point(v: std::collections::HashMap<String, Point>) -> std::collections::HashMap<String, Point> { v }

#[uniffi::export]
fn identity_optional_point(v: Option<Point>) -> Option<Point> { v }
