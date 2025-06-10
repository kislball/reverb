use rvb_core::storage::{Storage, StorageError};

#[derive(Debug, thiserror::Error)]
pub enum DiskStorageError {
    #[error("Sled error {0}")]
    Sled(sled::Error),
}

pub struct DiskStorage {
    db: sled::Db,
}

impl DiskStorage {
    pub fn new(path: &str) -> Result<Self, DiskStorageError> {
        Ok(Self {
            db: sled::open(path).map_err(DiskStorageError::Sled)?,
        })
    }
}

impl Storage for DiskStorage {
    fn get<T>(&self, table: &str, key: &str) -> Result<T, StorageError>
    where
        T: serde::de::DeserializeOwned,
    {
        let tree = self
            .db
            .open_tree(table.as_bytes())
            .map_err(|x| StorageError::Internal(Box::new(x)))?;

        let raw = tree
            .get(key)
            .map_err(|x| StorageError::Internal(Box::new(x)))?
            .ok_or(StorageError::KeyNotFound(key.to_owned()))?;

        let fin = bincode::serde::decode_from_slice(&raw, bincode::config::standard())
            .map_err(|x| StorageError::Internal(Box::new(x)))?;
        Ok(fin.0)
    }

    fn set<T>(&mut self, table: &str, key: &str, val: &T) -> Result<(), StorageError>
    where
        T: serde::Serialize,
    {
        let tree = self
            .db
            .open_tree(table.as_bytes())
            .map_err(|x| StorageError::Internal(Box::new(x)))?;

        let v = bincode::serde::encode_to_vec(val, bincode::config::standard())
            .map_err(|x| StorageError::Internal(Box::new(x)))?;
        tree.insert(key, v)
            .map_err(|x| StorageError::Internal(Box::new(x)))?;

        Ok(())
    }
}
