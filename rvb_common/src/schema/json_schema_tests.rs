use super::*;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct TestStruct {
    a: i32,
    b: String,
    c: bool,
}

#[test]
fn test_serialize_schema_simple() {
    let data = TestStruct {
        a: 42,
        b: "hello".to_string(),
        c: true,
    };
    let db_value = serialize_schema(&data);
    match db_value {
        DbValue::Object(ref map) => {
            assert_eq!(map.get("a"), Some(&Box::new(DbValue::Number(42))));
            assert_eq!(
                map.get("b"),
                Some(&Box::new(DbValue::String("hello".to_string())))
            );
            assert_eq!(map.get("c"), Some(&Box::new(DbValue::Boolean(true))));
        }
        _ => panic!("Expected DbValue::Object"),
    }
}

#[test]
fn test_into_schema_roundtrip() {
    let data = TestStruct {
        a: 7,
        b: "world".to_string(),
        c: false,
    };
    let db_value = serialize_schema(&data);
    let result: TestStruct = into_schema(db_value).unwrap();
    assert_eq!(result, data);
}

#[test]
fn test_dbvalue_into_json_and_from_json() {
    let db_value = DbValue::Array(vec![
        Box::new(DbValue::Number(1)),
        Box::new(DbValue::String("foo".to_string())),
        Box::new(DbValue::Boolean(false)),
    ]);
    let json_str = db_value.clone().into_json();
    let parsed = DbValue::from_json(&json_str).unwrap();
    assert_eq!(parsed, db_value);
}

#[test]
fn test_dbvalue_from_json_invalid() {
    let invalid_json = "{ this is not valid json ";
    let result = DbValue::from_json(invalid_json);
    assert!(result.is_err());
}

#[test]
fn test_dbvalue_from_and_into_value() {
    let value = serde_json::json!({
        "foo": 123,
        "bar": [true, null, "baz"]
    });
    let db_value = DbValue::from(value.clone());
    let value2: serde_json::Value = db_value.into();
    assert_eq!(value, value2);
}

#[test]
fn test_dbvalue_none_json() {
    let db_value = DbValue::None;
    let json_str = db_value.clone().into_json();
    assert_eq!(json_str, "null");
    let parsed = DbValue::from_json(&json_str).unwrap();
    assert_eq!(parsed, DbValue::None);
}

#[test]
fn test_dbvalue_nested_object_json() {
    let mut inner = HashMap::new();
    inner.insert("x".to_string(), Box::new(DbValue::Number(5)));
    let mut outer = HashMap::new();
    outer.insert("inner".to_string(), Box::new(DbValue::Object(inner)));
    let db_value = DbValue::Object(outer);

    let json_str = db_value.clone().into_json();
    let parsed = DbValue::from_json(&json_str).unwrap();
    assert_eq!(parsed, db_value);
}
