use super::GramSettings;
use crate::github::Repository;
use std::error::Error;

pub fn diff(repo: Repository, settings: GramSettings) -> Result<(), Box<dyn Error>> {
    if repo.description != settings.description {
        return Err(format!(
            "Current description [{}] does not match expected description [{}]",
            repo.description.unwrap_or_else(|| "null".to_owned()),
            settings.description.unwrap_or_else(|| "null".to_owned())
        )
        .into());
    }
    Ok(())
}
