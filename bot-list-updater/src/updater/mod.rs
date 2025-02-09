use crate::UpdaterError;
use async_trait::async_trait;

mod dbl_updater;
pub use dbl_updater::DblUpdater;

#[async_trait]
pub trait Updater {
    async fn update(&self, count: usize) -> Result<(), UpdaterError>;
}
