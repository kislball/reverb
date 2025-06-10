use serde::{Serialize, de::DeserializeOwned};
use std::error::Error;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    #[error("Internal error")]
    Internal(Box<dyn Error>),
}

pub trait Storage {
    fn get<T>(&self, table: &str, key: &str) -> Result<T, StorageError>
    where
        T: DeserializeOwned;
    fn set<T>(&mut self, table: &str, key: &str, val: &T) -> Result<(), StorageError>
    where
        T: Serialize;
}

