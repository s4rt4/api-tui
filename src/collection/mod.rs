pub mod build;
pub mod interpolate;
pub mod model;

use crate::error::ApiTesterError;
use model::Collection;
use std::path::Path;

pub fn load(path: &Path) -> Result<Collection, ApiTesterError> {
    if !path.exists() {
        return Err(ApiTesterError::CollectionNotFound(path.to_path_buf()));
    }
    let content = std::fs::read_to_string(path)?;
    let collection: Collection = toml::from_str(&content)?;
    Ok(collection)
}

pub fn save(path: &Path, collection: &Collection) -> Result<(), ApiTesterError> {
    let content = toml::to_string_pretty(collection)?;
    std::fs::write(path, content)?;
    Ok(())
}
