use super::types::SchemaObject;
use std::collections::BTreeMap;

/// JSON 값에서 JSON Schema를 추론합니다.
pub fn infer_schema(value: &serde_json::Value) -> SchemaObject {
    match value {
        serde_json::Value::Null => SchemaObject {
            schema_type: None,
            nullable: Some(true),
            properties: None,
            items: None,
            example: None,
            required: None,
            ref_path: None,
        },
        serde_json::Value::Bool(b) => SchemaObject {
            schema_type: Some("boolean".to_string()),
            example: Some(serde_json::Value::Bool(*b)),
            properties: None,
            items: None,
            nullable: None,
            required: None,
            ref_path: None,
        },
        serde_json::Value::Number(n) => {
            let type_str = if n.is_f64() && !n.is_i64() && !n.is_u64() {
                "number"
            } else {
                "integer"
            };
            SchemaObject {
                schema_type: Some(type_str.to_string()),
                example: Some(serde_json::Value::Number(n.clone())),
                properties: None,
                items: None,
                nullable: None,
                required: None,
                ref_path: None,
            }
        }
        serde_json::Value::String(s) => SchemaObject {
            schema_type: Some("string".to_string()),
            example: Some(serde_json::Value::String(truncate_example(s))),
            properties: None,
            items: None,
            nullable: None,
            required: None,
            ref_path: None,
        },
        serde_json::Value::Array(arr) => {
            let items = arr.first().map(|item| Box::new(infer_schema(item)));
            SchemaObject {
                schema_type: Some("array".to_string()),
                items,
                properties: None,
                example: None,
                nullable: None,
                required: None,
                ref_path: None,
            }
        }
        serde_json::Value::Object(obj) => {
            let properties: BTreeMap<String, SchemaObject> = obj
                .iter()
                .map(|(k, v)| (k.clone(), infer_schema(v)))
                .collect();
            SchemaObject {
                schema_type: Some("object".to_string()),
                properties: if properties.is_empty() {
                    None
                } else {
                    Some(properties)
                },
                items: None,
                example: None,
                nullable: None,
                required: None,
                ref_path: None,
            }
        }
    }
}

/// 여러 JSON 값의 스키마를 병합합니다 (optional 필드 감지).
pub fn merge_schemas(schemas: &[SchemaObject]) -> SchemaObject {
    if schemas.is_empty() {
        return SchemaObject {
            schema_type: None,
            properties: None,
            items: None,
            example: None,
            required: None,
            ref_path: None,
            nullable: None,
        };
    }
    if schemas.len() == 1 {
        return schemas[0].clone();
    }

    // 모든 스키마가 object인 경우 properties를 병합
    let all_objects = schemas
        .iter()
        .all(|s| s.schema_type.as_deref() == Some("object"));

    if all_objects {
        let mut merged_props: BTreeMap<String, SchemaObject> = BTreeMap::new();
        for schema in schemas {
            if let Some(props) = &schema.properties {
                for (k, v) in props {
                    merged_props.entry(k.clone()).or_insert_with(|| v.clone());
                }
            }
        }
        return SchemaObject {
            schema_type: Some("object".to_string()),
            properties: if merged_props.is_empty() {
                None
            } else {
                Some(merged_props)
            },
            items: None,
            example: None,
            required: None,
            ref_path: None,
            nullable: None,
        };
    }

    // 타입이 다르면 첫 번째 스키마 반환
    schemas[0].clone()
}

fn truncate_example(s: &str) -> String {
    if s.len() > 100 {
        format!("{}...", &s[..100])
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_string() {
        let schema = infer_schema(&serde_json::json!("hello"));
        assert_eq!(schema.schema_type.as_deref(), Some("string"));
    }

    #[test]
    fn test_infer_object() {
        let schema = infer_schema(&serde_json::json!({"name": "test", "age": 25}));
        assert_eq!(schema.schema_type.as_deref(), Some("object"));
        let props = schema.properties.unwrap();
        assert!(props.contains_key("name"));
        assert!(props.contains_key("age"));
    }

    #[test]
    fn test_infer_array() {
        let schema = infer_schema(&serde_json::json!([1, 2, 3]));
        assert_eq!(schema.schema_type.as_deref(), Some("array"));
        assert!(schema.items.is_some());
    }
}
