#[cfg(feature = "json_schema")]
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
#[cfg(feature = "json_schema")]
use serde_json::{Map, Value};
#[cfg(feature = "json_schema")]
use std::str::FromStr;
use std::{cmp::Ordering, collections::HashMap};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum DataAction {
    Insert {
        key: String,
        incoming_data: DbValue,
        params: HashMap<String, DbValue>,
    },
}

#[derive(Debug, thiserror::Error)]
#[cfg(feature = "json_schema")]
pub enum SchemaError {
    #[error("Invalid schema provided: {0}")]
    InvalidJson(serde_json::Error),
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum DbValue {
    String(String),
    Number(i128),
    Boolean(bool),
    Object(HashMap<String, Box<DbValue>>),
    Array(Vec<Box<DbValue>>),
    None,
}

#[cfg(feature = "json_schema")]
pub fn serialize_schema<T: Serialize>(data: &T) -> DbValue {
    use serde_json::json;
    json!(data).into()
}

#[cfg(feature = "json_schema")]
pub fn into_schema<T: DeserializeOwned>(data: DbValue) -> Result<T, SchemaError> {
    let value = data.into_json();
    serde_json::from_str(&value).map_err(SchemaError::InvalidJson)
}

#[cfg(feature = "json_schema")]
impl DbValue {
    #[must_use] pub fn into_json(self) -> String {
        Into::<Value>::into(self).to_string()
    }

    pub fn from_json(data: &str) -> Result<DbValue, SchemaError> {
        Value::from_str(data)
            .map(Into::into)
            .map_err(SchemaError::InvalidJson)
    }
}

#[cfg(feature = "json_schema")]
fn hashmap_into_json_map(map: HashMap<String, Box<DbValue>>) -> Map<String, Value> {
    let mut serde_map = Map::new();

    for (k, v) in map {
        serde_map.insert(k, (*v).into());
    }

    serde_map
}

#[cfg(feature = "json_schema")]
impl From<Value> for DbValue {
    fn from(value: Value) -> Self {
        match value {
            Value::Null => Self::None,
            Value::Bool(b) => Self::Boolean(b),
            Value::Number(number) => Self::Number(if let Some(n) = number.as_i128() {
                n
            } else if let Some(n) = number.as_f64() {
                n as i128
            } else if let Some(n) = number.as_u128() {
                n as i128
            } else {
                unreachable!()
            }),
            Value::String(str) => Self::String(str),
            Value::Array(values) => DbValue::Array(
                values
                    .into_iter()
                    .map(|x| Box::new(DbValue::from(x)))
                    .collect(),
            ),
            Value::Object(map) => DbValue::Object(HashMap::from_iter(
                map.into_iter().map(|x| (x.0, Box::new(DbValue::from(x.1)))),
            )),
        }
    }
}

#[cfg(feature = "json_schema")]
impl From<DbValue> for Value {
    fn from(val: DbValue) -> Self {
        use serde_json::json;

        match val {
            DbValue::String(str) => json!(str),
            DbValue::Number(number) => json!(number),
            DbValue::Boolean(b) => json!(b),
            DbValue::Object(hash_map) => Value::Object(hashmap_into_json_map(hash_map)),
            DbValue::Array(db_values) => Value::Array(
                db_values
                    .into_iter()
                    .map(|x| Into::<Value>::into(*x))
                    .collect(),
            ),
            DbValue::None => Value::Null,
        }
    }
}

impl Ord for DbValue {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let v1 = rmp_serde::to_vec(self).unwrap();
        let v2 = rmp_serde::to_vec(other).unwrap();
        v1.cmp(&v2)
    }
}

impl PartialOrd for DbValue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Copy, Debug)]
pub enum DumbMergePriority {
    Target,
    From,
    Content,
}

pub fn dumb_merge(
    target: &mut HashMap<String, Box<DbValue>>,
    from: &HashMap<String, Box<DbValue>>,
    priority: DumbMergePriority,
) {
    for (key, from_value) in from {
        if let Some(target_value) = target.get_mut(key) {
            match (&**from_value, &mut **target_value) {
                (DbValue::Object(from_value_map), DbValue::Object(target_value_map)) => {
                    dumb_merge(target_value_map, from_value_map, priority);
                }
                _ => match priority {
                    DumbMergePriority::From => {
                        *target_value = from_value.clone();
                    }
                    DumbMergePriority::Content => {
                        if from_value > target_value {
                            *target_value = from_value.clone();
                        }
                    }
                    _ => {}
                },
            }
        } else {
            target.insert(key.clone(), from_value.clone());
        }
    }
}

pub fn merge(
    target: &mut HashMap<String, Box<DbValue>>,
    from: &HashMap<String, Box<DbValue>>,
    target_state: &HashMap<String, u64>,
    from_state: &HashMap<String, u64>,
) {
    for (key, from_value) in from {
        let (t_state, f_state) = (
            target_state.get(key).copied().unwrap_or(0),
            from_state.get(key).copied().unwrap_or(0),
        );

        if let Some(target_value) = target.get_mut(key) {
            match t_state.cmp(&f_state) {
                Ordering::Equal => {
                    let should_replace = from_value > target_value;
                    match (&mut **target_value, &**from_value) {
                        (DbValue::Object(target_map), DbValue::Object(from_map)) => {
                            dumb_merge(target_map, from_map, DumbMergePriority::Content);
                        }
                        (_, _) if should_replace => {
                            *target_value = from_value.clone();
                        }
                        _ => {}
                    }
                }
                Ordering::Less => match (&mut **target_value, &**from_value) {
                    (DbValue::Object(target_map), DbValue::Object(from_map)) => {
                        dumb_merge(target_map, from_map, DumbMergePriority::From);
                    }
                    _ => *target_value = from_value.clone(),
                },
                _ => {}
            }
        } else {
            target.insert(key.clone(), from_value.clone());
        }
    }
}

#[cfg(all(test, feature = "json_schema"))]
mod json_schema_tests;
#[cfg(test)]
mod merge_tests;
