use std::sync::Arc;

#[uniffi::export(with_foreign)]
pub trait Drawable: Send + Sync {
    fn describe(&self) -> String;
    fn area(&self) -> f64;
}

#[derive(uniffi::Object)]
pub struct Circle {
    radius: f64,
}

#[uniffi::export]
impl Circle {
    #[uniffi::constructor]
    pub fn new(radius: f64) -> Self {
        Self { radius }
    }

    pub fn radius(&self) -> f64 {
        self.radius
    }
}

impl Drawable for Circle {
    fn describe(&self) -> String {
        format!("Circle(radius={})", self.radius)
    }
    fn area(&self) -> f64 {
        std::f64::consts::PI * self.radius * self.radius
    }
}

#[derive(uniffi::Object)]
pub struct Rect {
    width: f64,
    height: f64,
}

#[uniffi::export]
impl Rect {
    #[uniffi::constructor]
    pub fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    pub fn width(&self) -> f64 {
        self.width
    }

    pub fn height(&self) -> f64 {
        self.height
    }
}

impl Drawable for Rect {
    fn describe(&self) -> String {
        format!("Rect({}x{})", self.width, self.height)
    }
    fn area(&self) -> f64 {
        self.width * self.height
    }
}

#[uniffi::export]
pub fn make_shapes() -> Vec<Arc<dyn Drawable>> {
    vec![
        Arc::new(Circle::new(5.0)),
        Arc::new(Rect::new(3.0, 4.0)),
    ]
}

#[uniffi::export]
pub fn describe_all(shapes: Vec<Arc<dyn Drawable>>) -> String {
    shapes
        .iter()
        .map(|s| s.describe())
        .collect::<Vec<_>>()
        .join(", ")
}

#[uniffi::export]
pub fn total_area(shapes: Vec<Arc<dyn Drawable>>) -> f64 {
    shapes.iter().map(|s| s.area()).sum()
}

uniffi::setup_scaffolding!();
