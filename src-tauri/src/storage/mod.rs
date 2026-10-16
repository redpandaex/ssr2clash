mod file;
mod paths;

use crate::model::app::{AppError, SavedState};
use async_trait::async_trait;
pub use file::FileStore;
pub use paths::{detect_data_paths, DataPaths};

#[async_trait]
pub trait Storage: Send + Sync {
    async fn load(&self) -> Result<Option<SavedState>, AppError>;
    async fn save(&self, state: &SavedState) -> Result<(), AppError>;
}
