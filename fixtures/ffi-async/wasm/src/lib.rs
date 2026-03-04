#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum AsyncError {
    #[error("DivisionByZero")]
    DivisionByZero,
    #[error("InvalidInput")]
    InvalidInput,
}

#[uniffi::export]
pub async fn async_add(a: u32, b: u32) -> u32 {
    a + b
}

#[uniffi::export]
pub async fn async_greet(name: String) -> String {
    format!("Hello, {}!", name)
}

#[uniffi::export]
pub async fn async_noop() {}

#[uniffi::export]
pub async fn async_divide(a: f64, b: f64) -> Result<String, AsyncError> {
    if b == 0.0 {
        return Err(AsyncError::DivisionByZero);
    }
    Ok(format!("{}", a / b))
}

#[uniffi::export]
pub async fn async_get_counter_value(counter: std::sync::Arc<AsyncCounter>) -> u64 {
    counter.get_value().await
}

#[derive(uniffi::Object)]
pub struct AsyncCounter {
    value: std::sync::Mutex<u64>,
}

#[uniffi::export]
impl AsyncCounter {
    #[uniffi::constructor]
    pub async fn new(initial: u64) -> Self {
        Self {
            value: std::sync::Mutex::new(initial),
        }
    }

    pub async fn increment(&self) {
        *self.value.lock().unwrap() += 1;
    }

    pub async fn get_value(&self) -> u64 {
        *self.value.lock().unwrap()
    }

    pub async fn validate(&self) -> Result<(), AsyncError> {
        let val = *self.value.lock().unwrap();
        if val > 1_000_000 {
            Err(AsyncError::InvalidInput)
        } else {
            Ok(())
        }
    }
}

uniffi::setup_scaffolding!();
