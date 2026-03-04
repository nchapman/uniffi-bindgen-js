use uniffi;

uniffi::setup_scaffolding!();

// --- Flat error ---

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum MathError {
    #[error("DivisionByZero")]
    DivisionByZero,
    #[error("Overflow")]
    Overflow,
}

#[uniffi::export]
fn safe_divide(a: f64, b: f64) -> Result<String, MathError> {
    if b == 0.0 {
        return Err(MathError::DivisionByZero);
    }
    let result = a / b;
    if result.is_infinite() {
        return Err(MathError::Overflow);
    }
    Ok(format!("{result}"))
}

// --- Rich error ---

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum NetworkError {
    #[error("NotFound: {url}")]
    NotFound { url: String },
    #[error("Timeout after {after_ms}ms")]
    Timeout { after_ms: u32 },
    #[error("ServerError {code}: {message}")]
    ServerError { code: u16, message: String },
}

#[uniffi::export]
fn fetch_data(url: String) -> Result<String, NetworkError> {
    match url.as_str() {
        "good" => Ok("data for good".to_string()),
        "404" => Err(NetworkError::NotFound { url }),
        "timeout" => Err(NetworkError::Timeout { after_ms: 5000 }),
        "500" => Err(NetworkError::ServerError {
            code: 500,
            message: "Internal Server Error".to_string(),
        }),
        _ => Err(NetworkError::NotFound { url }),
    }
}

// --- Error for object operations ---

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum ParseError {
    #[error("InvalidInput")]
    InvalidInput,
    #[error("MissingSection")]
    MissingSection,
    #[error("SyntaxError")]
    SyntaxError,
}

// --- Object with throwing constructor and method ---

#[derive(uniffi::Object)]
pub struct Parser {
    input: String,
}

#[uniffi::export]
impl Parser {
    #[uniffi::constructor]
    fn new(input: String) -> Result<Self, ParseError> {
        if input.is_empty() {
            return Err(ParseError::InvalidInput);
        }
        Ok(Self { input })
    }

    fn result(&self) -> String {
        self.input.clone()
    }

    fn parse_section(&self, name: String) -> Result<String, ParseError> {
        if let Some(pos) = self.input.find(&name) {
            Ok(self.input[pos..pos + name.len()].to_string())
        } else {
            Err(ParseError::MissingSection)
        }
    }
}
