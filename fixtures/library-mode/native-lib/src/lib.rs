#[derive(uniffi::Record)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(uniffi::Enum)]
pub enum Direction {
    North,
    South,
    East,
    West,
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum MathError {
    #[error("Division by zero")]
    DivisionByZero,
    #[error("Overflow occurred")]
    Overflow,
}

#[uniffi::export]
fn add(a: u32, b: u32) -> u32 {
    a + b
}

#[uniffi::export]
fn translate(point: Point, direction: Direction) -> Point {
    match direction {
        Direction::North => Point { x: point.x, y: point.y + 1.0 },
        Direction::South => Point { x: point.x, y: point.y - 1.0 },
        Direction::East => Point { x: point.x + 1.0, y: point.y },
        Direction::West => Point { x: point.x - 1.0, y: point.y },
    }
}

#[uniffi::export]
fn safe_divide(a: u32, b: u32) -> Result<u32, MathError> {
    if b == 0 {
        Err(MathError::DivisionByZero)
    } else {
        Ok(a / b)
    }
}

uniffi::setup_scaffolding!("library_mode");
