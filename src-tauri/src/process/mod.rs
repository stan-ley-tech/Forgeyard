mod manager;

#[allow(unused_imports)] // ProcessStatus rides along inside ProcessInfo for the frontend
pub use manager::{ProcessInfo, ProcessManager, ProcessStatus};
