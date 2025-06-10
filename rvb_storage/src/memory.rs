use rvb_core::storage::{Storage, StorageError};
use std::collections::HashMap;

#[derive(Default)]
pub struct MemoryStorage {
    data: HashMap<String, HashMap<String, Vec<u8>>>,
}

impl Storage for MemoryStorage {
    fn get<T>(&self, table: &str, key: &str) -> Result<T, StorageError>
    where
        T: serde::de::DeserializeOwned,
    {
        let raw = self
            .data
            .get(table)
            .ok_or(StorageError::KeyNotFound(key.to_owned()))?
            .get(key)
            .ok_or(StorageError::KeyNotFound(key.to_owned()))?;

        let fin = bincode::serde::decode_from_slice(raw, bincode::config::standard())
            .map_err(|x| StorageError::Internal(Box::new(x)))?;
        Ok(fin.0)
    }

    fn set<T>(&mut self, table: &str, key: &str, val: &T) -> Result<(), StorageError>
    where
        T: serde::Serialize,
    {
        let v = bincode::serde::encode_to_vec(val, bincode::config::standard())
            .map_err(|x| StorageError::Internal(Box::new(x)))?;

        let map = self.data.get_mut(table);

        if let Some(m) = map {
            m.insert(key.to_owned(), v);
        } else {
            let mut m = HashMap::new();
            m.insert(key.to_owned(), v);
            self.data.insert(table.to_owned(), m);
        }

        Ok(())
    }
}
