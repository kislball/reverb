use base64::{Engine, engine::general_purpose};
use rvb_core::storage::{Storage, StorageError};
use serde::{Serialize, de::DeserializeOwned};

pub struct WebStorage {
    storage: web_sys::Storage,
}

impl WebStorage {
    pub fn new() -> Self {
        Self {
            storage: web_sys::window()
                .expect("no window")
                .local_storage()
                .expect("no local_storage")
                .expect("local_storage is None"),
        }
    }

    fn key(&self, table: &str, key: &str) -> String {
        format!("{table}:{key}")
    }
}

impl Storage for WebStorage {
    fn get<T>(&self, table: &str, key: &str) -> Result<T, StorageError>
    where
        T: DeserializeOwned,
    {
        let full_key = self.key(table, key);
        let b64 = self
            .storage
            .get_item(&full_key)
            .map_err(|_| StorageError::KeyNotFound(full_key.clone()))?
            .ok_or(StorageError::KeyNotFound(full_key.clone()))?;

        let data = general_purpose::STANDARD
            .decode(b64)
            .map_err(|x| StorageError::Internal(Box::new(x)))?;
        let fin = bincode::serde::decode_from_slice(&data, bincode::config::standard())
            .map_err(|x| StorageError::Internal(Box::new(x)))?;

        Ok(fin.0)
    }

    fn set<T>(&mut self, table: &str, key: &str, val: &T) -> Result<(), StorageError>
    where
        T: Serialize,
    {
        let key = self.key(table, key);

        let v = bincode::serde::encode_to_vec(val, bincode::config::standard())
            .map_err(|x| StorageError::Internal(Box::new(x)))?;
        let data = general_purpose::STANDARD.encode(&v);
        self.storage
            .set_item(&key, &data)
            .expect("setItem should always work");

        Ok(())
    }
}
