mod model;
mod store;

pub use model::RecoverySnapshot;
pub use store::RecoveryStore;

use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RecoveryError {
    #[error("Workspace recovery is unavailable in this environment.")]
    Unavailable,
    #[error("Another Frame window owns workspace recovery for this profile.")]
    ProfileInUse,
    #[error("Could not read or save workspace recovery: {0}")]
    Io(#[from] io::Error),
    #[error("Could not read workspace recovery: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Unsupported workspace recovery version {0}.")]
    UnsupportedVersion(u32),
    #[error("Workspace recovery exceeds its size limit.")]
    TooLarge,
    #[error("Invalid workspace recovery: {0}")]
    Invalid(&'static str),
    #[error("The recovery snapshot has not been resolved yet.")]
    Unresolved,
    #[error("A newer recovery snapshot has already been saved.")]
    StaleRevision,
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod store_tests;
