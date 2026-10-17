pub mod backend;
pub mod local;
pub mod manager;
#[cfg(feature = "s3")]
pub mod s3;

pub use backend::{StorageBackend, StorageConfig, StorageStats};
pub use local::LocalStorage;
pub use manager::StorageManager;
#[cfg(feature = "s3")]
pub use s3::S3Storage;
