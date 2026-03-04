uniffi::setup_scaffolding!();

#[uniffi::export(callback_interface)]
pub trait Formatter: Send + Sync {
    fn format(&self, input: String) -> String;
}

#[derive(uniffi::Object)]
pub struct Processor {
    formatter: Box<dyn Formatter>,
}

#[uniffi::export]
impl Processor {
    #[uniffi::constructor]
    fn new(formatter: Box<dyn Formatter>) -> Self {
        Processor { formatter }
    }

    fn process(&self, input: String) -> String {
        self.formatter.format(input)
    }
}
