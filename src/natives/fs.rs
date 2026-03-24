use crate::executor::RelType;
use std::collections::HashMap;

/// Reads a file and returns its contents as a String.
pub fn fs_read_file(path: String) -> String {
    match crate::executor::ExecutionEngine::validate_fs_path(&path) {
        Ok(safe_path) => std::fs::read_to_string(&safe_path).unwrap_or_else(|e| {
            eprintln!("[KnotenCore FS] Error reading '{}': {}", safe_path.display(), e);
            String::new()
        }),
        Err(e) => {
            eprintln!("[KnotenCore FS] Security error reading '{}': {}", path, e);
            String::new()
        }
    }
}

/// Parses a JSON string into a nested RelType structure.
/// - JSON Object → RelType::Object(HashMap)
/// - JSON Array → RelType::Array(Vec)
/// - JSON String → RelType::Str
/// - JSON Number → RelType::Int or RelType::Float
/// - JSON Bool → RelType::Bool
/// - JSON Null → RelType::Void
pub fn fs_parse_json(json_str: &str) -> Result<RelType, String> {
    match serde_json::from_str::<serde_json::Value>(json_str) {
        Ok(value) => Ok(json_value_to_reltype(&value)),
        Err(e) => Err(format!("JSON parse error: {}", e)),
    }
}

pub fn json_value_to_reltype(value: &serde_json::Value) -> RelType {
    match value {
        serde_json::Value::Null => RelType::Void,
        serde_json::Value::Bool(b) => RelType::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                RelType::Int(i)
            } else if let Some(f) = n.as_f64() {
                RelType::Float(f)
            } else {
                RelType::Int(0)
            }
        }
        serde_json::Value::String(s) => RelType::Str(s.clone()),
        serde_json::Value::Array(arr) => {
            RelType::Array(arr.iter().map(json_value_to_reltype).collect())
        }
        serde_json::Value::Object(obj) => {
            let mut map = HashMap::new();
            for (k, v) in obj {
                map.insert(k.clone(), json_value_to_reltype(v));
            }
            RelType::Object(map)
        }
    }
}

pub fn reltype_to_json_value(rel: &RelType) -> serde_json::Value {
    match rel {
        RelType::Int(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
        RelType::Float(f) => serde_json::Number::from_f64(*f).map(serde_json::Value::Number).unwrap_or(serde_json::Value::Null),
        RelType::Bool(b) => serde_json::Value::Bool(*b),
        RelType::Str(s) => serde_json::Value::String(s.clone()),
        RelType::Array(arr) => serde_json::Value::Array(arr.iter().map(reltype_to_json_value).collect()),
        RelType::Object(obj) => {
            let mut map = serde_json::Map::new();
            for (k, v) in obj {
                map.insert(k.clone(), reltype_to_json_value(v));
            }
            serde_json::Value::Object(map)
        }
        RelType::Dict(dict) => {
            if let Ok(map_guard) = dict.lock() {
                let mut map = serde_json::Map::new();
                for (k, v) in map_guard.iter() {
                    map.insert(k.clone(), reltype_to_json_value(v));
                }
                serde_json::Value::Object(map)
            } else {
                serde_json::Value::Null
            }
        }
        RelType::Void => serde_json::Value::Null,
        // Fallback for native execution handles and functions which cannot serialize cleanly
        _ => serde_json::Value::String(format!("[Unserializable: {}]", rel))
    }
}
