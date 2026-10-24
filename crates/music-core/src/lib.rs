pub mod audio;
pub mod download;
pub mod engine {
    pub mod client;
    mod diagnostics;
}
pub mod error;
pub mod loops;
pub mod manifest;
pub mod verification;

pub mod types {
    pub fn id() -> String {
        uuid::Uuid::new_v4().to_string()
    }
}

pub mod process;
