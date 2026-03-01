use wasm_bindgen::prelude::*;

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
