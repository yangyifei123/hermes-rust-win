//! Tool Argument Coercion
//!
//! Converts string arguments to correct types based on JSON Schema definitions.
//! LLMs sometimes return all arguments as strings; this module coerces them
//! to the expected types (integer, number, boolean) before tool dispatch.

use serde_json::Value;

/// Coerce string arguments to their correct types based on a JSON Schema.
///
/// The schema should have a `parameters.properties` object mapping parameter
/// names to their type definitions. Only string values that have a matching
/// schema property with a known type are coerced.
///
/// Supported type coercions:
/// - `"integer"` → parse as `i64`
/// - `"number"` → parse as `f64`
/// - `"boolean"` → parse `"true"/"yes"/"1"` as true, `"false"/"no"/"0"` as false
pub fn coerce_args(schema: &Value, args: &mut Value) {
    let properties =
        schema.get("parameters").and_then(|p| p.get("properties")).and_then(|p| p.as_object());

    if properties.is_none() || !args.is_object() {
        return;
    }

    let props = properties.unwrap();
    let args_obj = args.as_object_mut().unwrap();

    for (key, value) in args_obj.iter_mut() {
        if !value.is_string() {
            continue;
        }
        let Some(prop) = props.get(key) else {
            continue;
        };
        let Some(typ) = prop.get("type").and_then(|t| t.as_str()) else {
            continue;
        };

        let s = value.as_str().unwrap();
        *value = match typ {
            "integer" => s
                .parse::<i64>()
                .map(|n| Value::Number(n.into()))
                .unwrap_or_else(|_| Value::String(s.to_string())),
            "number" => {
                if let Ok(f) = s.parse::<f64>() {
                    if let Some(n) = serde_json::Number::from_f64(f) {
                        Value::Number(n)
                    } else {
                        Value::String(s.to_string())
                    }
                } else {
                    Value::String(s.to_string())
                }
            }
            "boolean" => match s.trim().to_lowercase().as_str() {
                "true" | "yes" | "1" => Value::Bool(true),
                "false" | "no" | "0" => Value::Bool(false),
                _ => Value::String(s.to_string()),
            },
            _ => value.clone(),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_coerce_integer() {
        let schema = json!({
            "parameters": {
                "properties": {
                    "count": {"type": "integer"}
                }
            }
        });
        let mut args = json!({"count": "42"});
        coerce_args(&schema, &mut args);
        assert_eq!(args["count"], Value::Number(42i64.into()));
    }

    #[test]
    fn test_coerce_number() {
        let schema = json!({
            "parameters": {
                "properties": {
                    "ratio": {"type": "number"}
                }
            }
        });
        let mut args = json!({"ratio": "3.14"});
        coerce_args(&schema, &mut args);
        assert_eq!(args["ratio"], Value::Number(serde_json::Number::from_f64(3.14).unwrap()));
    }

    #[test]
    fn test_coerce_boolean_true() {
        let schema = json!({
            "parameters": {
                "properties": {
                    "flag": {"type": "boolean"}
                }
            }
        });

        for val in &["true", "yes", "1", "True", "YES"] {
            let mut args = json!({"flag": *val});
            coerce_args(&schema, &mut args);
            assert_eq!(args["flag"], Value::Bool(true), "Failed for input: {}", val);
        }
    }

    #[test]
    fn test_coerce_boolean_false() {
        let schema = json!({
            "parameters": {
                "properties": {
                    "flag": {"type": "boolean"}
                }
            }
        });

        for val in &["false", "no", "0", "False", "NO"] {
            let mut args = json!({"flag": *val});
            coerce_args(&schema, &mut args);
            assert_eq!(args["flag"], Value::Bool(false), "Failed for input: {}", val);
        }
    }

    #[test]
    fn test_coerce_invalid_integer_stays_string() {
        let schema = json!({
            "parameters": {
                "properties": {
                    "count": {"type": "integer"}
                }
            }
        });
        let mut args = json!({"count": "not_a_number"});
        coerce_args(&schema, &mut args);
        assert_eq!(args["count"], Value::String("not_a_number".to_string()));
    }

    #[test]
    fn test_coerce_non_string_unchanged() {
        let schema = json!({
            "parameters": {
                "properties": {
                    "count": {"type": "integer"}
                }
            }
        });
        let mut args = json!({"count": 42});
        coerce_args(&schema, &mut args);
        assert_eq!(args["count"], Value::Number(42i64.into()));
    }

    #[test]
    fn test_coerce_unknown_property_skipped() {
        let schema = json!({
            "parameters": {
                "properties": {
                    "known": {"type": "integer"}
                }
            }
        });
        let mut args = json!({"unknown": "42"});
        coerce_args(&schema, &mut args);
        assert_eq!(args["unknown"], Value::String("42".to_string()));
    }

    #[test]
    fn test_coerce_no_schema_properties() {
        let schema = json!({});
        let mut args = json!({"count": "42"});
        coerce_args(&schema, &mut args);
        assert_eq!(args["count"], Value::String("42".to_string()));
    }

    #[test]
    fn test_coerce_args_not_object() {
        let schema = json!({"parameters": {"properties": {"x": {"type": "integer"}}}});
        let mut args = json!("not an object");
        coerce_args(&schema, &mut args);
        assert_eq!(args, json!("not an object"));
    }

    #[test]
    fn test_coerce_multiple_types() {
        let schema = json!({
            "parameters": {
                "properties": {
                    "count": {"type": "integer"},
                    "ratio": {"type": "number"},
                    "enabled": {"type": "boolean"},
                    "name": {"type": "string"}
                }
            }
        });
        let mut args = json!({
            "count": "10",
            "ratio": "2.5",
            "enabled": "true",
            "name": "test"
        });
        coerce_args(&schema, &mut args);
        assert_eq!(args["count"], Value::Number(10i64.into()));
        assert_eq!(args["ratio"], Value::Number(serde_json::Number::from_f64(2.5).unwrap()));
        assert_eq!(args["enabled"], Value::Bool(true));
        assert_eq!(args["name"], Value::String("test".to_string()));
    }
}
