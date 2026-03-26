//! DynamoDB AttributeValue helpers.
//!
//! `attr_value_string`  — compact human-readable cell text for table display.
//! `attr_value_to_json` — converts to `serde_json::Value` for JSON export /
//!                        editor round-trips.

use aws_sdk_dynamodb::types::AttributeValue;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde_json::{json, Number, Value};

/// Return a compact, human-readable string for a single `AttributeValue`.
/// Used to populate table cells in the items view.
pub fn attr_value_string(av: &AttributeValue) -> String {
    match av {
        AttributeValue::S(s) => s.clone(),
        AttributeValue::N(n) => n.clone(),
        AttributeValue::Bool(b) => (if *b { "true" } else { "false" }).to_string(),
        AttributeValue::Null(_) => "null".to_string(),
        AttributeValue::L(list) => format!("[…{}]", list.len()),
        AttributeValue::M(map) => format!("{{…{}}}", map.len()),
        AttributeValue::Ss(ss) => format!("SS({})", ss.len()),
        AttributeValue::Ns(ns) => format!("NS({})", ns.len()),
        AttributeValue::Bs(bs) => format!("BS({})", bs.len()),
        AttributeValue::B(b) => format!("<binary {}B>", b.as_ref().len()),
        _ => "?".to_string(),
    }
}

/// Convert an `AttributeValue` into a `serde_json::Value` suitable for
/// serialisation and display.  Binary blobs are base64-encoded.
pub fn attr_value_to_json(av: &AttributeValue) -> Value {
    match av {
        AttributeValue::S(s) => Value::String(s.clone()),
        AttributeValue::N(n) => {
            // Try to parse as f64 for JSON numbers; fall back to string.
            if let Ok(f) = n.parse::<f64>() {
                if let Some(num) = Number::from_f64(f) {
                    return Value::Number(num);
                }
            }
            Value::String(n.clone())
        }
        AttributeValue::Bool(b) => Value::Bool(*b),
        AttributeValue::Null(_) => Value::Null,
        AttributeValue::L(list) => Value::Array(list.iter().map(attr_value_to_json).collect()),
        AttributeValue::M(map) => {
            let obj: serde_json::Map<String, Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), attr_value_to_json(v)))
                .collect();
            Value::Object(obj)
        }
        AttributeValue::Ss(ss) => Value::Array(ss.iter().map(|s| json!(s)).collect()),
        AttributeValue::Ns(ns) => Value::Array(ns.iter().map(|s| json!(s)).collect()),
        AttributeValue::B(b) => Value::String(B64.encode(b.as_ref())),
        AttributeValue::Bs(bs) => {
            Value::Array(bs.iter().map(|b| json!(B64.encode(b.as_ref()))).collect())
        }
        _ => Value::String("?".to_string()),
    }
}

/// Convert a raw DynamoDB item map to a pretty-printed JSON string.
pub fn item_to_json_string(
    item: &std::collections::HashMap<String, AttributeValue>,
) -> Result<String, serde_json::Error> {
    let obj: serde_json::Map<String, Value> = item
        .iter()
        .map(|(k, v)| (k.clone(), attr_value_to_json(v)))
        .collect();
    serde_json::to_string_pretty(&Value::Object(obj))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_attr() {
        let av = AttributeValue::S("hello".to_string());
        assert_eq!(attr_value_string(&av), "hello");
    }

    #[test]
    fn number_attr() {
        let av = AttributeValue::N("42".to_string());
        assert_eq!(attr_value_string(&av), "42");
    }

    #[test]
    fn bool_attr() {
        assert_eq!(attr_value_string(&AttributeValue::Bool(true)), "true");
        assert_eq!(attr_value_string(&AttributeValue::Bool(false)), "false");
    }

    #[test]
    fn null_attr() {
        assert_eq!(attr_value_string(&AttributeValue::Null(true)), "null");
    }

    #[test]
    fn list_attr() {
        let av = AttributeValue::L(vec![
            AttributeValue::S("a".to_string()),
            AttributeValue::S("b".to_string()),
        ]);
        assert_eq!(attr_value_string(&av), "[…2]");
    }

    #[test]
    fn map_attr() {
        let mut m = std::collections::HashMap::new();
        m.insert("k".to_string(), AttributeValue::S("v".to_string()));
        let av = AttributeValue::M(m);
        assert_eq!(attr_value_string(&av), "{…1}");
    }

    #[test]
    fn to_json_string() {
        let av = AttributeValue::S("world".to_string());
        assert_eq!(attr_value_to_json(&av), Value::String("world".to_string()));
    }

    #[test]
    fn to_json_number() {
        let av = AttributeValue::N("3.14".to_string());
        // Should be a Number, not a String
        assert!(matches!(attr_value_to_json(&av), Value::Number(_)));
    }
}
