use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::types::{OpenApiSpec, PathItem, SchemaObject};

/// 검증 위반 유형
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ViolationType {
    /// 스펙에 정의되지 않은 상태 코드
    StatusCodeMismatch,
    /// 필수 필드 누락
    MissingField,
    /// 타입 불일치
    TypeMismatch,
    /// 스펙에 정의되지 않은 추가 필드
    ExtraField,
    /// 스펙에 해당 경로가 없음
    PathNotFound,
    /// 스펙에 해당 메서드가 없음
    MethodNotAllowed,
}

/// 위반 심각도
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Severity {
    Error,
    Warning,
}

/// 개별 검증 위반 항목
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContractViolation {
    pub violation_type: ViolationType,
    /// JSON path (e.g. "$.body.data.name")
    pub path: String,
    /// 사람이 읽을 수 있는 설명
    pub message: String,
    /// 스펙에서 기대하는 값
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    /// 실제 값
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,
    pub severity: Severity,
}

/// 하나의 스펙에 대한 검증 결과
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContractValidationResult {
    pub request_id: String,
    pub spec_id: String,
    pub violations: Vec<ContractViolation>,
    pub validated_at: i64,
    /// 매칭된 OpenAPI path 패턴 (e.g. "/users/{id}")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_path: Option<String>,
    /// 매칭된 HTTP 메서드
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_operation: Option<String>,
}

/// UI 전송용 스펙 정보 (경량)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContractSpecInfo {
    pub id: String,
    pub name: String,
    pub file_path: String,
    pub enabled: bool,
    pub path_count: usize,
    pub loaded_at: i64,
}

/// OpenAPI 스펙의 path 템플릿과 요청 경로를 매칭합니다.
///
/// 예: 요청 경로 "/users/123"은 스펙의 "/users/{id}"와 매칭됩니다.
pub fn match_path_template<'a>(
    request_path: &str,
    spec_paths: &'a BTreeMap<String, PathItem>,
) -> Option<(String, &'a PathItem)> {
    // 쿼리 스트링 제거
    let path = request_path.split('?').next().unwrap_or(request_path);

    // 정확한 매칭 우선
    if let Some(item) = spec_paths.get(path) {
        return Some((path.to_string(), item));
    }

    let request_segments: Vec<&str> = path.split('/').collect();

    let mut best_match: Option<(String, &PathItem, usize)> = None;

    for (template, item) in spec_paths {
        let template_segments: Vec<&str> = template.split('/').collect();

        if template_segments.len() != request_segments.len() {
            continue;
        }

        let mut matches = true;
        let mut static_count = 0;

        for (t_seg, r_seg) in template_segments.iter().zip(request_segments.iter()) {
            if t_seg.starts_with('{') && t_seg.ends_with('}') {
                // 파라미터 세그먼트 — 항상 매칭
                continue;
            } else if t_seg == r_seg {
                static_count += 1;
            } else {
                matches = false;
                break;
            }
        }

        if matches {
            // static 세그먼트가 더 많은 것이 더 구체적인 매칭
            if best_match
                .as_ref()
                .map_or(true, |(_, _, best_static)| static_count > *best_static)
            {
                best_match = Some((template.clone(), item, static_count));
            }
        }
    }

    best_match.map(|(template, item, _)| (template, item))
}

/// PathItem에서 주어진 HTTP 메서드에 해당하는 Operation을 찾습니다.
fn get_operation_for_method<'a>(
    path_item: &'a PathItem,
    method: &str,
) -> Option<&'a super::types::Operation> {
    match method.to_uppercase().as_str() {
        "GET" => path_item.get.as_ref(),
        "POST" => path_item.post.as_ref(),
        "PUT" => path_item.put.as_ref(),
        "DELETE" => path_item.delete.as_ref(),
        "PATCH" => path_item.patch.as_ref(),
        "HEAD" => path_item.head.as_ref(),
        "OPTIONS" => path_item.options.as_ref(),
        _ => None,
    }
}

/// OpenAPI 스펙에 대해 응답을 검증합니다.
pub fn validate_response(
    spec: &OpenApiSpec,
    method: &str,
    request_path: &str,
    status: u16,
    body_json: Option<&serde_json::Value>,
) -> (Vec<ContractViolation>, Option<String>, Option<String>) {
    let mut violations = Vec::new();

    // 1. Path 매칭
    let (matched_path, path_item) = match match_path_template(request_path, &spec.paths) {
        Some((p, item)) => (Some(p), item),
        None => {
            violations.push(ContractViolation {
                violation_type: ViolationType::PathNotFound,
                path: "$".to_string(),
                message: format!(
                    "Path '{}' is not defined in the spec '{}'",
                    request_path, spec.info.title
                ),
                expected: None,
                actual: Some(request_path.to_string()),
                severity: Severity::Warning,
            });
            return (violations, None, None);
        }
    };

    let matched_operation = Some(method.to_uppercase());

    // 2. Method 매칭
    let operation = match get_operation_for_method(path_item, method) {
        Some(op) => op,
        None => {
            violations.push(ContractViolation {
                violation_type: ViolationType::MethodNotAllowed,
                path: "$".to_string(),
                message: format!(
                    "Method '{}' is not defined for path '{}'",
                    method.to_uppercase(),
                    matched_path.as_deref().unwrap_or(request_path)
                ),
                expected: None,
                actual: Some(method.to_uppercase()),
                severity: Severity::Error,
            });
            return (violations, matched_path, matched_operation);
        }
    };

    // 3. Status code 확인
    let status_str = status.to_string();
    let has_status = operation.responses.contains_key(&status_str)
        || operation.responses.contains_key("default");

    if !has_status {
        let defined_statuses: Vec<String> = operation.responses.keys().cloned().collect();
        violations.push(ContractViolation {
            violation_type: ViolationType::StatusCodeMismatch,
            path: "$.status".to_string(),
            message: format!(
                "Status code {} is not defined in the spec. Defined: [{}]",
                status,
                defined_statuses.join(", ")
            ),
            expected: Some(defined_statuses.join(", ")),
            actual: Some(status_str.clone()),
            severity: Severity::Error,
        });
    }

    // 4. Response body 스키마 검증
    let response_spec = operation
        .responses
        .get(&status_str)
        .or_else(|| operation.responses.get("default"));

    if let (Some(response_spec), Some(body)) = (response_spec, body_json) {
        if let Some(content) = &response_spec.content {
            // application/json 스키마 찾기
            let schema = content
                .get("application/json")
                .or_else(|| content.get("application/*"))
                .or_else(|| content.get("*/*"))
                .and_then(|mt| mt.schema.as_ref());

            if let Some(schema) = schema {
                validate_against_schema(body, schema, "$.body", &mut violations);
            }
        }
    }

    (violations, matched_path, matched_operation)
}

/// JSON 값을 스키마에 대해 재귀적으로 검증합니다.
pub fn validate_against_schema(
    value: &serde_json::Value,
    schema: &SchemaObject,
    json_path: &str,
    violations: &mut Vec<ContractViolation>,
) {
    // null 값 처리
    if value.is_null() {
        if schema.nullable == Some(true) {
            return;
        }
        if let Some(expected_type) = &schema.schema_type {
            violations.push(ContractViolation {
                violation_type: ViolationType::TypeMismatch,
                path: json_path.to_string(),
                message: format!("Expected type '{}', got null", expected_type),
                expected: Some(expected_type.clone()),
                actual: Some("null".to_string()),
                severity: Severity::Error,
            });
        }
        return;
    }

    let Some(expected_type) = &schema.schema_type else {
        return;
    };

    let actual_type = json_value_type(value);

    // 타입 검증
    match expected_type.as_str() {
        "object" => {
            if !value.is_object() {
                violations.push(ContractViolation {
                    violation_type: ViolationType::TypeMismatch,
                    path: json_path.to_string(),
                    message: format!("Expected type 'object', got '{}'", actual_type),
                    expected: Some("object".to_string()),
                    actual: Some(actual_type.to_string()),
                    severity: Severity::Error,
                });
                return;
            }

            let obj = value.as_object().unwrap();

            // properties 검증
            if let Some(properties) = &schema.properties {
                // 스펙에 정의된 필드가 응답에 있는지 확인
                for (prop_name, prop_schema) in properties {
                    if let Some(prop_value) = obj.get(prop_name) {
                        let child_path = format!("{}.{}", json_path, prop_name);
                        validate_against_schema(prop_value, prop_schema, &child_path, violations);
                    }
                    // NOTE: required 필드가 SchemaObject에 없으므로
                    // 필드 누락은 Warning으로 보고하지 않음 (스펙 정보 부족)
                }

                // 스펙에 정의되지 않은 추가 필드 검사 (Warning)
                for key in obj.keys() {
                    if !properties.contains_key(key) {
                        violations.push(ContractViolation {
                            violation_type: ViolationType::ExtraField,
                            path: format!("{}.{}", json_path, key),
                            message: format!("Field '{}' is not defined in the spec", key),
                            expected: None,
                            actual: Some(key.clone()),
                            severity: Severity::Warning,
                        });
                    }
                }
            }
        }
        "array" => {
            if !value.is_array() {
                violations.push(ContractViolation {
                    violation_type: ViolationType::TypeMismatch,
                    path: json_path.to_string(),
                    message: format!("Expected type 'array', got '{}'", actual_type),
                    expected: Some("array".to_string()),
                    actual: Some(actual_type.to_string()),
                    severity: Severity::Error,
                });
                return;
            }

            // items 스키마로 첫 번째 요소 검증 (성능상 전체 검증은 생략)
            if let Some(items_schema) = &schema.items {
                if let Some(first) = value.as_array().and_then(|arr| arr.first()) {
                    let child_path = format!("{}[0]", json_path);
                    validate_against_schema(first, items_schema, &child_path, violations);
                }
            }
        }
        "string" => {
            if !value.is_string() {
                violations.push(ContractViolation {
                    violation_type: ViolationType::TypeMismatch,
                    path: json_path.to_string(),
                    message: format!("Expected type 'string', got '{}'", actual_type),
                    expected: Some("string".to_string()),
                    actual: Some(actual_type.to_string()),
                    severity: Severity::Error,
                });
            }
        }
        "integer" => {
            if !value.is_i64() && !value.is_u64() {
                violations.push(ContractViolation {
                    violation_type: ViolationType::TypeMismatch,
                    path: json_path.to_string(),
                    message: format!("Expected type 'integer', got '{}'", actual_type),
                    expected: Some("integer".to_string()),
                    actual: Some(actual_type.to_string()),
                    severity: Severity::Error,
                });
            }
        }
        "number" => {
            if !value.is_number() {
                violations.push(ContractViolation {
                    violation_type: ViolationType::TypeMismatch,
                    path: json_path.to_string(),
                    message: format!("Expected type 'number', got '{}'", actual_type),
                    expected: Some("number".to_string()),
                    actual: Some(actual_type.to_string()),
                    severity: Severity::Error,
                });
            }
        }
        "boolean" => {
            if !value.is_boolean() {
                violations.push(ContractViolation {
                    violation_type: ViolationType::TypeMismatch,
                    path: json_path.to_string(),
                    message: format!("Expected type 'boolean', got '{}'", actual_type),
                    expected: Some("boolean".to_string()),
                    actual: Some(actual_type.to_string()),
                    severity: Severity::Error,
                });
            }
        }
        _ => {}
    }
}

/// JSON 값의 타입 문자열을 반환합니다.
fn json_value_type(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                "integer"
            } else {
                "number"
            }
        }
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openapi::types::*;

    fn make_spec() -> OpenApiSpec {
        let mut paths = BTreeMap::new();

        // GET /users/{id}
        let mut responses = BTreeMap::new();
        responses.insert(
            "200".to_string(),
            ResponseSpec {
                description: "Success".to_string(),
                content: Some({
                    let mut content = BTreeMap::new();
                    content.insert(
                        "application/json".to_string(),
                        MediaType {
                            schema: Some(SchemaObject {
                                schema_type: Some("object".to_string()),
                                properties: Some({
                                    let mut props = BTreeMap::new();
                                    props.insert(
                                        "id".to_string(),
                                        SchemaObject {
                                            schema_type: Some("integer".to_string()),
                                            properties: None,
                                            items: None,
                                            nullable: None,
                                            example: None,
                                        },
                                    );
                                    props.insert(
                                        "name".to_string(),
                                        SchemaObject {
                                            schema_type: Some("string".to_string()),
                                            properties: None,
                                            items: None,
                                            nullable: None,
                                            example: None,
                                        },
                                    );
                                    props
                                }),
                                items: None,
                                nullable: None,
                                example: None,
                            }),
                        },
                    );
                    content
                }),
            },
        );
        responses.insert(
            "404".to_string(),
            ResponseSpec {
                description: "Not Found".to_string(),
                content: None,
            },
        );

        let mut user_path = PathItem::default();
        user_path.get = Some(Operation {
            summary: Some("Get user by ID".to_string()),
            parameters: vec![],
            request_body: None,
            responses,
        });

        paths.insert("/users/{id}".to_string(), user_path);

        // GET /items
        let mut items_responses = BTreeMap::new();
        items_responses.insert(
            "200".to_string(),
            ResponseSpec {
                description: "List".to_string(),
                content: Some({
                    let mut content = BTreeMap::new();
                    content.insert(
                        "application/json".to_string(),
                        MediaType {
                            schema: Some(SchemaObject {
                                schema_type: Some("array".to_string()),
                                properties: None,
                                items: Some(Box::new(SchemaObject {
                                    schema_type: Some("object".to_string()),
                                    properties: Some({
                                        let mut props = BTreeMap::new();
                                        props.insert(
                                            "title".to_string(),
                                            SchemaObject {
                                                schema_type: Some("string".to_string()),
                                                properties: None,
                                                items: None,
                                                nullable: None,
                                                example: None,
                                            },
                                        );
                                        props
                                    }),
                                    items: None,
                                    nullable: None,
                                    example: None,
                                })),
                                nullable: None,
                                example: None,
                            }),
                        },
                    );
                    content
                }),
            },
        );

        let mut items_path = PathItem::default();
        items_path.get = Some(Operation {
            summary: None,
            parameters: vec![],
            request_body: None,
            responses: items_responses,
        });
        paths.insert("/items".to_string(), items_path);

        OpenApiSpec {
            openapi: "3.0.0".to_string(),
            info: OpenApiInfo {
                title: "Test API".to_string(),
                version: "1.0.0".to_string(),
                description: None,
            },
            paths,
            servers: None,
        }
    }

    #[test]
    fn test_match_path_exact() {
        let spec = make_spec();
        let result = match_path_template("/items", &spec.paths);
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, "/items");
    }

    #[test]
    fn test_match_path_with_param() {
        let spec = make_spec();
        let result = match_path_template("/users/123", &spec.paths);
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, "/users/{id}");
    }

    #[test]
    fn test_match_path_with_query() {
        let spec = make_spec();
        let result = match_path_template("/items?page=1", &spec.paths);
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, "/items");
    }

    #[test]
    fn test_match_path_not_found() {
        let spec = make_spec();
        let result = match_path_template("/nonexistent", &spec.paths);
        assert!(result.is_none());
    }

    #[test]
    fn test_validate_response_path_not_found() {
        let spec = make_spec();
        let (violations, matched, _) = validate_response(&spec, "GET", "/nonexistent", 200, None);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].violation_type, ViolationType::PathNotFound);
        assert!(matched.is_none());
    }

    #[test]
    fn test_validate_response_method_not_allowed() {
        let spec = make_spec();
        let (violations, _, _) = validate_response(&spec, "DELETE", "/items", 200, None);
        assert_eq!(violations.len(), 1);
        assert_eq!(
            violations[0].violation_type,
            ViolationType::MethodNotAllowed
        );
    }

    #[test]
    fn test_validate_response_status_mismatch() {
        let spec = make_spec();
        let (violations, _, _) = validate_response(&spec, "GET", "/users/1", 500, None);
        assert_eq!(violations.len(), 1);
        assert_eq!(
            violations[0].violation_type,
            ViolationType::StatusCodeMismatch
        );
    }

    #[test]
    fn test_validate_response_valid() {
        let spec = make_spec();
        let body = serde_json::json!({"id": 1, "name": "Alice"});
        let (violations, matched_path, matched_op) =
            validate_response(&spec, "GET", "/users/42", 200, Some(&body));
        assert!(
            violations.is_empty(),
            "Expected no violations, got: {:?}",
            violations
        );
        assert_eq!(matched_path.as_deref(), Some("/users/{id}"));
        assert_eq!(matched_op.as_deref(), Some("GET"));
    }

    #[test]
    fn test_validate_response_type_mismatch() {
        let spec = make_spec();
        let body = serde_json::json!({"id": "not-a-number", "name": "Alice"});
        let (violations, _, _) = validate_response(&spec, "GET", "/users/1", 200, Some(&body));
        assert!(violations
            .iter()
            .any(|v| v.violation_type == ViolationType::TypeMismatch));
    }

    #[test]
    fn test_validate_response_extra_field() {
        let spec = make_spec();
        let body = serde_json::json!({"id": 1, "name": "Alice", "extra": true});
        let (violations, _, _) = validate_response(&spec, "GET", "/users/1", 200, Some(&body));
        assert!(violations
            .iter()
            .any(|v| v.violation_type == ViolationType::ExtraField));
    }

    #[test]
    fn test_validate_array_body() {
        let spec = make_spec();
        let body = serde_json::json!([{"title": "Item 1"}]);
        let (violations, _, _) = validate_response(&spec, "GET", "/items", 200, Some(&body));
        assert!(violations.is_empty());
    }

    #[test]
    fn test_validate_array_item_type_mismatch() {
        let spec = make_spec();
        let body = serde_json::json!([{"title": 123}]);
        let (violations, _, _) = validate_response(&spec, "GET", "/items", 200, Some(&body));
        assert!(violations
            .iter()
            .any(|v| v.violation_type == ViolationType::TypeMismatch));
    }

    #[test]
    fn test_validate_null_with_nullable() {
        let schema = SchemaObject {
            schema_type: Some("string".to_string()),
            properties: None,
            items: None,
            nullable: Some(true),
            example: None,
        };
        let mut violations = Vec::new();
        validate_against_schema(&serde_json::Value::Null, &schema, "$", &mut violations);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_validate_null_without_nullable() {
        let schema = SchemaObject {
            schema_type: Some("string".to_string()),
            properties: None,
            items: None,
            nullable: None,
            example: None,
        };
        let mut violations = Vec::new();
        validate_against_schema(&serde_json::Value::Null, &schema, "$", &mut violations);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].violation_type, ViolationType::TypeMismatch);
    }
}
