#[uniffi::export]
pub fn add(a: u32, b: u32) -> u32 {
    a + b
}

#[uniffi::export]
pub fn greet(name: String) -> String {
    format!("Hello, {}!", name)
}

#[uniffi::export]
pub fn get_counter_value(counter: std::sync::Arc<Counter>) -> u64 {
    *counter.value.lock().unwrap()
}

#[uniffi::export]
pub fn clone_counter(counter: std::sync::Arc<Counter>) -> std::sync::Arc<Counter> {
    let val = *counter.value.lock().unwrap();
    std::sync::Arc::new(Counter {
        value: std::sync::Mutex::new(val),
    })
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

    pub fn add_from(&self, other: std::sync::Arc<Counter>) {
        let other_val = *other.value.lock().unwrap();
        *self.value.lock().unwrap() += other_val;
    }
}

uniffi::setup_scaffolding!();
