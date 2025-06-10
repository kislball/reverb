use serde::{Deserialize, Serialize};
use std::{
    cmp::{Ordering, max},
    collections::HashMap,
};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ContractAction {
    pub test: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum DbValue {
    String(String),
    Number(i128),
    Binary(Vec<u8>),
    Boolean(bool),
    Object(HashMap<String, Box<DbValue>>),
    Array(Vec<Box<DbValue>>),
}

impl Ord for DbValue {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl PartialOrd for DbValue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let v1 = rmp_serde::to_vec(self).unwrap();
        let v2 = rmp_serde::to_vec(other).unwrap();
        Some(v1.cmp(&v2))
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
    for (key, from_value) in from.iter() {
        if let Some(target_value) = target.get_mut(key) {
            match (&**from_value, &mut **target_value) {
                (DbValue::Object(from_value_map), DbValue::Object(target_value_map)) => {
                    dumb_merge(target_value_map, &from_value_map, priority);
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

fn both_objects(a: &DbValue, b: &DbValue) -> bool {
    match (a, b) {
        (DbValue::Object(_), DbValue::Object(_)) => true,
        _ => false,
    }
}

pub fn merge(
    target: &mut HashMap<String, Box<DbValue>>,
    from: &HashMap<String, Box<DbValue>>,
    target_state: &HashMap<String, u64>,
    from_state: &HashMap<String, u64>,
) {
    for (key, from_value) in from.iter() {
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
                            dumb_merge(target_map, from_map, DumbMergePriority::Content)
                        }
                        (_, _) if should_replace => {
                            *target_value = from_value.clone();
                        }
                        _ => {}
                    };
                }
                Ordering::Less => {
                    match (&mut **target_value, &**from_value) {
                        (DbValue::Object(target_map), DbValue::Object(from_map)) => {
                            dumb_merge(target_map, from_map, DumbMergePriority::From)
                        }
                        _ => *target_value = from_value.clone(),
                    };
                }
                _ => {}
            }
        } else {
            target.insert(key.clone(), from_value.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db_str(s: &str) -> Box<DbValue> {
        Box::new(DbValue::String(s.to_string()))
    }

    fn db_num(n: i128) -> Box<DbValue> {
        Box::new(DbValue::Number(n))
    }

    fn db_bool(b: bool) -> Box<DbValue> {
        Box::new(DbValue::Boolean(b))
    }

    fn db_obj(map: HashMap<String, Box<DbValue>>) -> Box<DbValue> {
        Box::new(DbValue::Object(map))
    }

    #[test]
    fn test_merge_equal_state_content_priority() {
        let mut target = HashMap::new();
        target.insert("a".to_string(), db_num(1));
        let mut from = HashMap::new();
        from.insert("a".to_string(), db_num(2));
        let mut target_state = HashMap::new();
        target_state.insert("a".to_string(), 5);
        let mut from_state = HashMap::new();
        from_state.insert("a".to_string(), 5);

        merge(&mut target, &from, &target_state, &from_state);

        assert_eq!(target.get("a"), Some(&db_num(2)));
    }

    #[test]
    fn test_merge_equal_state_content_priority_no_replace() {
        let mut target = HashMap::new();
        target.insert("a".to_string(), db_num(5));
        let mut from = HashMap::new();
        from.insert("a".to_string(), db_num(2));
        let mut target_state = HashMap::new();
        target_state.insert("a".to_string(), 1);
        let mut from_state = HashMap::new();
        from_state.insert("a".to_string(), 1);

        merge(&mut target, &from, &target_state, &from_state);

        assert_eq!(target.get("a"), Some(&db_num(5)));
    }

    #[test]
    fn test_merge_from_state_greater() {
        let mut target = HashMap::new();
        target.insert("a".to_string(), db_num(1));
        let mut from = HashMap::new();
        from.insert("a".to_string(), db_num(2));
        let mut target_state = HashMap::new();
        target_state.insert("a".to_string(), 1);
        let mut from_state = HashMap::new();
        from_state.insert("a".to_string(), 2);

        merge(&mut target, &from, &target_state, &from_state);

        assert_eq!(target.get("a"), Some(&db_num(2)));
    }

    #[test]
    fn test_merge_target_state_greater() {
        let mut target = HashMap::new();
        target.insert("a".to_string(), db_num(10));
        let mut from = HashMap::new();
        from.insert("a".to_string(), db_num(20));
        let mut target_state = HashMap::new();
        target_state.insert("a".to_string(), 5);
        let mut from_state = HashMap::new();
        from_state.insert("a".to_string(), 2);

        merge(&mut target, &from, &target_state, &from_state);

        assert_eq!(target.get("a"), Some(&db_num(10)));
    }

    #[test]
    fn test_merge_insert_new_key() {
        let mut target = HashMap::new();
        target.insert("a".to_string(), db_num(1));
        let mut from = HashMap::new();
        from.insert("b".to_string(), db_num(2));
        let target_state = HashMap::new();
        let from_state = HashMap::new();

        merge(&mut target, &from, &target_state, &from_state);

        assert_eq!(target.get("a"), Some(&db_num(1)));
        assert_eq!(target.get("b"), Some(&db_num(2)));
    }

    #[test]
    fn test_merge_nested_object_with_state() {
        let mut target_inner = HashMap::new();
        target_inner.insert("x".to_string(), db_num(1));
        let mut target = HashMap::new();
        target.insert("obj".to_string(), db_obj(target_inner));
        let mut from_inner = HashMap::new();
        from_inner.insert("y".to_string(), db_num(2));
        let mut from = HashMap::new();
        from.insert("obj".to_string(), db_obj(from_inner));
        let mut target_state = HashMap::new();
        target_state.insert("obj".to_string(), 1);
        let mut from_state = HashMap::new();
        from_state.insert("obj".to_string(), 1);

        merge(&mut target, &from, &target_state, &from_state);

        let obj = match target.get("obj").unwrap().as_ref() {
            DbValue::Object(map) => map,
            _ => panic!("Expected object"),
        };
        assert_eq!(obj.get("x"), Some(&db_num(1)));
        assert_eq!(obj.get("y"), Some(&db_num(2)));
    }

    #[test]
    fn test_merge_empty_from_map() {
        let mut target = HashMap::new();
        target.insert("a".to_string(), db_num(1));
        let from = HashMap::new();
        let target_state = HashMap::new();
        let from_state = HashMap::new();

        merge(&mut target, &from, &target_state, &from_state);

        assert_eq!(target.get("a"), Some(&db_num(1)));
        assert_eq!(target.len(), 1);
    }

    #[test]
    fn test_merge_empty_target_map() {
        let mut target = HashMap::new();
        let mut from = HashMap::new();
        from.insert("a".to_string(), db_num(2));
        let target_state = HashMap::new();
        let from_state = HashMap::new();

        merge(&mut target, &from, &target_state, &from_state);

        assert_eq!(target.get("a"), Some(&db_num(2)));
        assert_eq!(target.len(), 1);
    }

    #[test]
    fn test_merge_nested_object_state_greater() {
        let mut target_inner = HashMap::new();
        target_inner.insert("x".to_string(), db_num(1));
        let mut target = HashMap::new();
        target.insert("obj".to_string(), db_obj(target_inner));
        let mut from_inner = HashMap::new();
        from_inner.insert("x".to_string(), db_num(2));
        let mut from = HashMap::new();
        from.insert("obj".to_string(), db_obj(from_inner));
        let mut target_state = HashMap::new();
        target_state.insert("obj".to_string(), 2);
        let mut from_state = HashMap::new();
        from_state.insert("obj".to_string(), 1);

        merge(&mut target, &from, &target_state, &from_state);

        let obj = match target.get("obj").unwrap().as_ref() {
            DbValue::Object(map) => map,
            _ => panic!("Expected object"),
        };
        assert_eq!(obj.get("x"), Some(&db_num(1)));
    }

    #[test]
    fn test_merge_simple_insert() {
        let mut target = HashMap::new();
        target.insert("a".to_string(), db_str("foo"));

        let mut from = HashMap::new();
        from.insert("b".to_string(), db_str("bar"));

        dumb_merge(&mut target, &from, DumbMergePriority::From);

        assert_eq!(target.get("a"), Some(&db_str("foo")));
        assert_eq!(target.get("b"), Some(&db_str("bar")));
    }

    #[test]
    fn test_merge_overwrite_from_priority() {
        let mut target = HashMap::new();
        target.insert("a".to_string(), db_str("foo"));

        let mut from = HashMap::new();
        from.insert("a".to_string(), db_str("bar"));

        dumb_merge(&mut target, &from, DumbMergePriority::From);

        assert_eq!(target.get("a"), Some(&db_str("bar")));
    }

    #[test]
    fn test_merge_overwrite_target_priority() {
        let mut target = HashMap::new();
        target.insert("a".to_string(), db_str("foo"));

        let mut from = HashMap::new();
        from.insert("a".to_string(), db_str("bar"));

        dumb_merge(&mut target, &from, DumbMergePriority::Target);

        assert_eq!(target.get("a"), Some(&db_str("foo")));
    }

    #[test]
    fn test_merge_nested_object() {
        let mut target_inner = HashMap::new();
        target_inner.insert("x".to_string(), db_num(1));
        let mut target = HashMap::new();
        target.insert("obj".to_string(), db_obj(target_inner));

        let mut from_inner = HashMap::new();
        from_inner.insert("y".to_string(), db_num(2));
        let mut from = HashMap::new();
        from.insert("obj".to_string(), db_obj(from_inner));

        dumb_merge(&mut target, &from, DumbMergePriority::From);

        let obj = match target.get("obj").unwrap().as_ref() {
            DbValue::Object(map) => map,
            _ => panic!("Expected object"),
        };
        assert_eq!(obj.get("x"), Some(&db_num(1)));
        assert_eq!(obj.get("y"), Some(&db_num(2)));
    }

    #[test]
    fn test_merge_nested_object_overwrite() {
        let mut target_inner = HashMap::new();
        target_inner.insert("x".to_string(), db_num(1));
        let mut target = HashMap::new();
        target.insert("obj".to_string(), db_obj(target_inner));

        let mut from_inner = HashMap::new();
        from_inner.insert("x".to_string(), db_num(42));
        let mut from = HashMap::new();
        from.insert("obj".to_string(), db_obj(from_inner));

        dumb_merge(&mut target, &from, DumbMergePriority::From);

        let obj = match target.get("obj").unwrap().as_ref() {
            DbValue::Object(map) => map,
            _ => panic!("Expected object"),
        };
        assert_eq!(obj.get("x"), Some(&db_num(42)));
    }

    #[test]
    fn test_merge_array_value() {
        let mut target = HashMap::new();
        target.insert("arr".to_string(), Box::new(DbValue::Array(vec![db_num(1)])));

        let mut from = HashMap::new();
        from.insert("arr".to_string(), Box::new(DbValue::Array(vec![db_num(2)])));

        dumb_merge(&mut target, &from, DumbMergePriority::From);

        assert_eq!(
            target.get("arr"),
            Some(&Box::new(DbValue::Array(vec![db_num(2)])))
        );
    }

    #[test]
    fn test_merge_boolean_value() {
        let mut target = HashMap::new();
        target.insert("flag".to_string(), db_bool(false));

        let mut from = HashMap::new();
        from.insert("flag".to_string(), db_bool(true));

        dumb_merge(&mut target, &from, DumbMergePriority::From);

        assert_eq!(target.get("flag"), Some(&db_bool(true)));
    }

    #[test]
    fn test_merge_content_priority_replaces_if_greater() {
        let mut target = HashMap::new();
        target.insert("a".to_string(), db_num(1));

        let mut from = HashMap::new();
        from.insert("a".to_string(), db_num(2));

        dumb_merge(&mut target, &from, DumbMergePriority::Content);

        assert_eq!(target.get("a"), Some(&db_num(2)));
    }

    #[test]
    fn test_merge_content_priority_does_not_replace_if_less() {
        let mut target = HashMap::new();
        target.insert("a".to_string(), db_num(5));

        let mut from = HashMap::new();
        from.insert("a".to_string(), db_num(2));

        dumb_merge(&mut target, &from, DumbMergePriority::Content);

        assert_eq!(target.get("a"), Some(&db_num(5)));
    }

    #[test]
    fn test_merge_with_empty_from() {
        let mut target = HashMap::new();
        target.insert("a".to_string(), db_str("foo"));

        let from = HashMap::new();

        dumb_merge(&mut target, &from, DumbMergePriority::From);

        assert_eq!(target.get("a"), Some(&db_str("foo")));
        assert_eq!(target.len(), 1);
    }

    #[test]
    fn test_merge_with_empty_target() {
        let mut target = HashMap::new();

        let mut from = HashMap::new();
        from.insert("a".to_string(), db_str("foo"));

        dumb_merge(&mut target, &from, DumbMergePriority::From);

        assert_eq!(target.get("a"), Some(&db_str("foo")));
        assert_eq!(target.len(), 1);
    }

    #[test]
    fn test_merge_nested_object_content_priority() {
        let mut target_inner = HashMap::new();
        target_inner.insert("x".to_string(), db_num(1));
        let mut target = HashMap::new();
        target.insert("obj".to_string(), db_obj(target_inner));

        let mut from_inner = HashMap::new();
        from_inner.insert("x".to_string(), db_num(2));
        let mut from = HashMap::new();
        from.insert("obj".to_string(), db_obj(from_inner));

        dumb_merge(&mut target, &from, DumbMergePriority::Content);

        let obj = match target.get("obj").unwrap().as_ref() {
            DbValue::Object(map) => map,
            _ => panic!("Expected object"),
        };
        assert_eq!(obj.get("x"), Some(&db_num(2)));
    }

    #[test]
    fn test_merge_array_with_different_lengths() {
        let mut target = HashMap::new();
        target.insert(
            "arr".to_string(),
            Box::new(DbValue::Array(vec![db_num(1), db_num(2)])),
        );

        let mut from = HashMap::new();
        from.insert("arr".to_string(), Box::new(DbValue::Array(vec![db_num(3)])));

        dumb_merge(&mut target, &from, DumbMergePriority::From);

        assert_eq!(
            target.get("arr"),
            Some(&Box::new(DbValue::Array(vec![db_num(3)])))
        );
    }

    #[test]
    fn test_merge_object_and_non_object() {
        let mut target = HashMap::new();
        let mut obj = HashMap::new();
        obj.insert("x".to_string(), db_num(1));
        target.insert("a".to_string(), db_obj(obj));

        let mut from = HashMap::new();
        from.insert("a".to_string(), db_num(42));

        dumb_merge(&mut target, &from, DumbMergePriority::From);

        assert_eq!(target.get("a"), Some(&db_num(42)));
    }

    #[test]
    fn test_merge_non_object_and_object() {
        let mut target = HashMap::new();
        target.insert("a".to_string(), db_num(42));

        let mut from = HashMap::new();
        let mut obj = HashMap::new();
        obj.insert("x".to_string(), db_num(1));
        from.insert("a".to_string(), db_obj(obj));

        dumb_merge(&mut target, &from, DumbMergePriority::From);

        match target.get("a").unwrap().as_ref() {
            DbValue::Object(map) => assert_eq!(map.get("x"), Some(&db_num(1))),
            _ => panic!("Expected object"),
        }
    }

    #[test]
    fn test_merge_boolean_and_number() {
        let mut target = HashMap::new();
        target.insert("a".to_string(), db_bool(true));

        let mut from = HashMap::new();
        from.insert("a".to_string(), db_num(1));

        dumb_merge(&mut target, &from, DumbMergePriority::From);

        assert_eq!(target.get("a"), Some(&db_num(1)));
    }

    #[test]
    fn test_merge_number_and_boolean() {
        let mut target = HashMap::new();
        target.insert("a".to_string(), db_num(1));

        let mut from = HashMap::new();
        from.insert("a".to_string(), db_bool(false));

        dumb_merge(&mut target, &from, DumbMergePriority::From);

        assert_eq!(target.get("a"), Some(&db_bool(false)));
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ContractContext {
    pub action: ContractAction,
    pub space: String,
    pub signed_by: Vec<u8>,
    pub contract_params: HashMap<String, DbValue>,
}
