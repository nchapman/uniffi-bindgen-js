#[uniffi::export]
pub fn add(a: u32, b: u32) -> u32 {
    a + b
}

#[uniffi::export]
pub fn greet(name: String) -> String {
    format!("Hello, {}!", name)
}

#[derive(uniffi::Object)]
pub struct Counter {
    value: std::sync::Mutex<u64>,
}

#[uniffi::export]
impl Counter {
    #[uniffi::constructor]
    pub fn new(initial_value: u64) -> Self {
        Self {
            value: std::sync::Mutex::new(initial_value),
        }
    }

    pub fn increment(&self) {
        *self.value.lock().unwrap() += 1;
    }

    pub fn get_value(&self) -> u64 {
        *self.value.lock().unwrap()
    }
}

uniffi::setup_scaffolding!();
