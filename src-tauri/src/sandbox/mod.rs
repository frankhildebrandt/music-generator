#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub mod probe;
#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(not(target_os = "macos"))]
pub fn initialize(_root: &std::path::Path) -> crate::error::Result<()> {
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn remember(_path: &std::path::Path) -> crate::error::Result<()> {
    Ok(())
}
