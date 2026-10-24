use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Message(String),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Sql(#[from] rusqlite::Error),
    #[error("{0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Http(#[from] reqwest::Error),
    #[error("{operation}: Zeitüberschreitung nach {minutes} Min. In den Einstellungen (Zahnrad) die Timeout-Zeit erhöhen, besonders bei langen Songs. / Timeout after {minutes} min. Raise the timeout in the settings (gear), especially for long songs.")]
    EngineTimeout { operation: String, minutes: u64 },
    #[error("Operation cancelled")]
    Cancelled,
}

impl Serialize for Error {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn message(text: impl Into<String>) -> Error {
    Error::Message(text.into())
}
