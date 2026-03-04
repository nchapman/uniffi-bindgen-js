// --- Default arguments ---

#[uniffi::export]
pub fn greet(name: Option<String>) -> String {
    match name {
        Some(n) => format!("Hello, {}!", n),
        None => "Hello, world!".to_string(),
    }
}

#[uniffi::export]
pub fn add_maybe(a: u32, b: Option<u32>) -> u32 {
    a + b.unwrap_or(0)
}

// --- Reserved-word identifiers ---

#[uniffi::export(name = "class")]
pub fn class_fn(switch: u32) -> u32 {
    switch + 1
}

#[uniffi::export(name = "return")]
pub fn return_fn(var: String) -> String {
    format!("returned: {var}")
}

#[uniffi::export(name = "delete")]
pub fn delete_fn(r#static: bool) -> bool {
    !r#static
}

#[uniffi::export]
pub fn describe_keywords(rv: ReturnValue) -> String {
    format!("class={}, return={}, typeof={}", rv.class, rv.r#return, rv.r#typeof)
}

// --- Record with reserved-word fields ---

#[derive(uniffi::Record)]
pub struct ReturnValue {
    pub class: String,
    pub r#return: u32,
    pub r#typeof: bool,
}

// --- Record with field defaults ---

#[derive(uniffi::Record)]
pub struct Config {
    pub host: String,
    #[uniffi(default = 8080)]
    pub port: u32,
    #[uniffi(default = false)]
    pub verbose: bool,
    #[uniffi(default = None)]
    pub label: Option<String>,
}

// --- Flat enum ---

#[derive(uniffi::Enum)]
pub enum Status {
    Active,
    Inactive,
}

#[uniffi::export]
pub fn status_name(s: Status) -> String {
    match s {
        Status::Active => "Active".to_string(),
        Status::Inactive => "Inactive".to_string(),
    }
}

// --- Error for named constructor ---

#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum BuildError {
    #[error("invalid input")]
    InvalidInput,
    #[error("overflow")]
    Overflow,
}

// --- Object with named constructors, reserved-word methods, defaults ---

#[derive(uniffi::Object)]
pub struct Widget {
    label: String,
    value: i32,
}

#[uniffi::export]
impl Widget {
    #[uniffi::constructor]
    pub fn new(label: String) -> Self {
        Self { label, value: 0 }
    }

    #[uniffi::constructor(name = "from_positive")]
    pub fn from_positive(value: i32) -> Result<Self, BuildError> {
        if value < 0 {
            Err(BuildError::InvalidInput)
        } else if value > 1_000_000 {
            Err(BuildError::Overflow)
        } else {
            Ok(Self {
                label: format!("widget-{value}"),
                value,
            })
        }
    }

    pub fn get_label(&self) -> String {
        self.label.clone()
    }

    #[uniffi::method(name = "class")]
    pub fn class_method(&self) -> u32 {
        self.value as u32
    }

    pub fn get_config(&self) -> Config {
        Config {
            host: "localhost".to_string(),
            port: 8080,
            verbose: false,
            label: Some(self.label.clone()),
        }
    }

    pub fn format(&self, prefix: Option<String>) -> String {
        match prefix {
            Some(p) => format!("{p}: {}", self.label),
            None => self.label.clone(),
        }
    }
}

uniffi::setup_scaffolding!();
