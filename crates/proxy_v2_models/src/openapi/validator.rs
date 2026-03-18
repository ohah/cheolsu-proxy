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
                .is_none_or(|(_, _, best_static)| static_count > *best_static)
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

/// `$ref` 참조를 해석하여 실제 스키마를 반환합니다.
/// 예: "#/components/schemas/User" → components.schemas.User
fn resolve_ref<'a>(
    ref_path: &str,
    components: Option<&'a super::types::Components>,
) -> Option<&'a SchemaObject> {
    let prefix = "#/components/schemas/";
    if !ref_path.starts_with(prefix) {
        return None;
    }
    let schema_name = &ref_path[prefix.len()..];
    components
        .and_then(|c| c.schemas.as_ref())
        .and_then(|schemas| schemas.get(schema_name))
}

/// 스키마를 해석합니다. `$ref`가 있으면 참조를 따라가고, 없으면 원본을 반환합니다.
fn resolve_schema<'a>(
    schema: &'a SchemaObject,
    components: Option<&'a super::types::Components>,
) -> &'a SchemaObject {
    if let Some(ref_path) = &schema.ref_path {
        resolve_ref(ref_path, components).unwrap_or(schema)
    } else {
        schema
    }
}

/// OpenAPI 스펙에 대해 요청/응답을 검증합니다.
///
/// `request_body_json`: 요청 body JSON (POST/PUT/PATCH 등)
/// `response_body_json`: 응답 body JSON
pub fn validate_transaction(
    spec: &OpenApiSpec,
    method: &str,
    request_path: &str,
    status: u16,
    request_body_json: Option<&serde_json::Value>,
    response_body_json: Option<&serde_json::Value>,
) -> (Vec<ContractViolation>, Option<String>, Option<String>) {
    let mut violations = Vec::new();
    let components = spec.components.as_ref();

    // 1. Path 매칭
    let (matched_path, path_item) = match match_path_template(request_path, &spec.paths) {
        Some((p, item)) => (Some(p), item),
        None => {
            violations.push(violation(
                ViolationType::PathNotFound,
                "$",
                format!(
                    "Path '{}' is not defined in the spec '{}'",
                    request_path, spec.info.title
                ),
                None,
                Some(request_path.to_string()),
                Severity::Warning,
            ));
            return (violations, None, None);
        }
    };

    let matched_operation = Some(method.to_uppercase());

    // 2. Method 매칭
    let operation = match get_operation_for_method(path_item, method) {
        Some(op) => op,
        None => {
            violations.push(violation(
                ViolationType::MethodNotAllowed,
                "$",
                format!(
                    "Method '{}' is not defined for path '{}'",
                    method.to_uppercase(),
                    matched_path.as_deref().unwrap_or(request_path)
                ),
                None,
                Some(method.to_uppercase()),
                Severity::Error,
            ));
            return (violations, matched_path, matched_operation);
        }
    };

    // 3. Request body 검증
    if let Some(request_body_spec) = &operation.request_body {
        if let Some(req_body) = request_body_json {
            // 요청 body가 있으면 스키마 검증
            let schema = request_body_spec
                .content
                .get("application/json")
                .or_else(|| request_body_spec.content.get("application/*"))
                .or_else(|| request_body_spec.content.get("*/*"))
                .and_then(|mt| mt.schema.as_ref());

            if let Some(schema) = schema {
                let resolved = resolve_schema(schema, components);
                validate_against_schema(
                    req_body,
                    resolved,
                    "$.request.body",
                    &mut violations,
                    components,
                );
            }
        } else if request_body_spec.required == Some(true) {
            // 필수 요청 body가 없음
            violations.push(violation(
                ViolationType::MissingField,
                "$.request.body",
                "Request body is required but not provided",
                Some("request body".to_string()),
                None,
                Severity::Error,
            ));
        }
    }

    // 4. Status code 확인
    let status_str = status.to_string();
    let has_status = operation.responses.contains_key(&status_str)
        || operation.responses.contains_key("default");

    if !has_status {
        let defined_statuses: Vec<String> = operation.responses.keys().cloned().collect();
        violations.push(violation(
            ViolationType::StatusCodeMismatch,
            "$.status",
            format!(
                "Status code {} is not defined in the spec. Defined: [{}]",
                status,
                defined_statuses.join(", ")
            ),
            Some(defined_statuses.join(", ")),
            Some(status_str.clone()),
            Severity::Error,
        ));
    }

    // 5. Response body 스키마 검증
    let response_spec = operation
        .responses
        .get(&status_str)
        .or_else(|| operation.responses.get("default"));

    if let (Some(response_spec), Some(body)) = (response_spec, response_body_json) {
        if let Some(content) = &response_spec.content {
            let schema = content
                .get("application/json")
                .or_else(|| content.get("application/*"))
                .or_else(|| content.get("*/*"))
                .and_then(|mt| mt.schema.as_ref());

            if let Some(schema) = schema {
                let resolved = resolve_schema(schema, components);
                validate_against_schema(body, resolved, "$.body", &mut violations, components);
            }
        }
    }

    (violations, matched_path, matched_operation)
}

/// 스키마 검증의 최대 재귀 깊이 (순환 $ref 보호)
const MAX_VALIDATION_DEPTH: usize = 32;

/// JSON 값을 스키마에 대해 재귀적으로 검증합니다 ($ref 해석 포함).
pub fn validate_against_schema(
    value: &serde_json::Value,
    schema: &SchemaObject,
    json_path: &str,
    violations: &mut Vec<ContractViolation>,
    components: Option<&super::types::Components>,
) {
    validate_against_schema_inner(value, schema, json_path, violations, components, 0);
}

fn validate_against_schema_inner(
    value: &serde_json::Value,
    schema: &SchemaObject,
    json_path: &str,
    violations: &mut Vec<ContractViolation>,
    components: Option<&super::types::Components>,
    depth: usize,
) {
    if depth >= MAX_VALIDATION_DEPTH {
        return;
    }

    // $ref 해석
    let schema = resolve_schema(schema, components);

    // null 값 처리
    if value.is_null() {
        if schema.nullable == Some(true) {
            return;
        }
        if let Some(expected_type) = &schema.schema_type {
            violations.push(type_mismatch(json_path, expected_type, "null"));
        }
        return;
    }

    let Some(expected_type) = &schema.schema_type else {
        return;
    };

    let actual_type = json_value_type(value);

    match expected_type.as_str() {
        "object" => {
            validate_object(
                value,
                schema,
                json_path,
                actual_type,
                violations,
                components,
                depth,
            );
        }
        "array" => {
            validate_array(
                value,
                schema,
                json_path,
                actual_type,
                violations,
                components,
                depth,
            );
        }
        "string" if !value.is_string() => {
            violations.push(type_mismatch(json_path, "string", actual_type));
        }
        "integer" if !value.is_i64() && !value.is_u64() => {
            violations.push(type_mismatch(json_path, "integer", actual_type));
        }
        "number" if !value.is_number() => {
            violations.push(type_mismatch(json_path, "number", actual_type));
        }
        "boolean" if !value.is_boolean() => {
            violations.push(type_mismatch(json_path, "boolean", actual_type));
        }
        _ => {}
    }
}

/// object 타입 스키마 검증 (프로퍼티, required, 추가 필드 검사)
fn validate_object(
    value: &serde_json::Value,
    schema: &SchemaObject,
    json_path: &str,
    actual_type: &str,
    violations: &mut Vec<ContractViolation>,
    components: Option<&super::types::Components>,
    depth: usize,
) {
    if !value.is_object() {
        violations.push(type_mismatch(json_path, "object", actual_type));
        return;
    }

    let obj = value.as_object().unwrap();

    if let Some(properties) = &schema.properties {
        for (prop_name, prop_schema) in properties {
            if let Some(prop_value) = obj.get(prop_name) {
                let child_path = format!("{}.{}", json_path, prop_name);
                validate_against_schema_inner(
                    prop_value,
                    prop_schema,
                    &child_path,
                    violations,
                    components,
                    depth + 1,
                );
            }
        }

        // required 필드 누락 검사
        if let Some(required_fields) = &schema.required {
            for field in required_fields {
                if !obj.contains_key(field) {
                    violations.push(violation(
                        ViolationType::MissingField,
                        format!("{}.{}", json_path, field),
                        format!("Required field '{}' is missing", field),
                        Some(field.clone()),
                        None,
                        Severity::Error,
                    ));
                }
            }
        }

        // 스펙에 정의되지 않은 추가 필드 검사 (Warning)
        for key in obj.keys() {
            if !properties.contains_key(key) {
                violations.push(violation(
                    ViolationType::ExtraField,
                    format!("{}.{}", json_path, key),
                    format!("Field '{}' is not defined in the spec", key),
                    None,
                    Some(key.clone()),
                    Severity::Warning,
                ));
            }
        }
    }
}

/// array 타입 스키마 검증 (첫 번째 요소 검사)
fn validate_array(
    value: &serde_json::Value,
    schema: &SchemaObject,
    json_path: &str,
    actual_type: &str,
    violations: &mut Vec<ContractViolation>,
    components: Option<&super::types::Components>,
    depth: usize,
) {
    if !value.is_array() {
        violations.push(type_mismatch(json_path, "array", actual_type));
        return;
    }

    if let Some(items_schema) = &schema.items {
        if let Some(first) = value.as_array().and_then(|arr| arr.first()) {
            let child_path = format!("{}[0]", json_path);
            validate_against_schema_inner(
                first,
                items_schema,
                &child_path,
                violations,
                components,
                depth + 1,
            );
        }
    }
}

/// ContractViolation을 간편하게 생성하는 헬퍼 함수
fn violation(
    violation_type: ViolationType,
    path: impl Into<String>,
    message: impl Into<String>,
    expected: Option<String>,
    actual: Option<String>,
    severity: Severity,
) -> ContractViolation {
    ContractViolation {
        violation_type,
        path: path.into(),
        message: message.into(),
        expected,
        actual,
        severity,
    }
}

/// 타입 불일치 위반을 생성하는 헬퍼 함수
fn type_mismatch(json_path: &str, expected_type: &str, actual_type: &str) -> ContractViolation {
    violation(
        ViolationType::TypeMismatch,
        json_path,
        format!("Expected type '{}', got '{}'", expected_type, actual_type),
        Some(expected_type.to_string()),
        Some(actual_type.to_string()),
        Severity::Error,
    )
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

    /// 간편한 SchemaObject 생성 헬퍼
    fn schema(t: &str) -> SchemaObject {
        SchemaObject {
            schema_type: Some(t.to_string()),
            properties: None,
            items: None,
            nullable: None,
            example: None,
            required: None,
            ref_path: None,
        }
    }

    fn schema_ref(ref_path: &str) -> SchemaObject {
        SchemaObject {
            schema_type: None,
            properties: None,
            items: None,
            nullable: None,
            example: None,
            required: None,
            ref_path: Some(ref_path.to_string()),
        }
    }

    fn make_spec() -> OpenApiSpec {
        let mut paths = BTreeMap::new();

        // GET /users/{id} — 200 응답에 required: [id, name]
        let mut user_responses = BTreeMap::new();
        user_responses.insert(
            "200".to_string(),
            ResponseSpec {
                description: "Success".to_string(),
                content: Some(BTreeMap::from([(
                    "application/json".to_string(),
                    MediaType {
                        schema: Some(SchemaObject {
                            schema_type: Some("object".to_string()),
                            properties: Some(BTreeMap::from([
                                ("id".to_string(), schema("integer")),
                                ("name".to_string(), schema("string")),
                            ])),
                            items: None,
                            nullable: None,
                            example: None,
                            required: Some(vec!["id".to_string(), "name".to_string()]),
                            ref_path: None,
                        }),
                    },
                )])),
            },
        );
        user_responses.insert(
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
            responses: user_responses,
        });
        paths.insert("/users/{id}".to_string(), user_path);

        // GET /items — 배열 응답
        let mut items_responses = BTreeMap::new();
        items_responses.insert(
            "200".to_string(),
            ResponseSpec {
                description: "List".to_string(),
                content: Some(BTreeMap::from([(
                    "application/json".to_string(),
                    MediaType {
                        schema: Some(SchemaObject {
                            schema_type: Some("array".to_string()),
                            properties: None,
                            items: Some(Box::new(SchemaObject {
                                schema_type: Some("object".to_string()),
                                properties: Some(BTreeMap::from([(
                                    "title".to_string(),
                                    schema("string"),
                                )])),
                                items: None,
                                nullable: None,
                                example: None,
                                required: None,
                                ref_path: None,
                            })),
                            nullable: None,
                            example: None,
                            required: None,
                            ref_path: None,
                        }),
                    },
                )])),
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

        // POST /users — 요청 body 필수 + $ref 응답
        let mut create_user_responses = BTreeMap::new();
        create_user_responses.insert(
            "201".to_string(),
            ResponseSpec {
                description: "Created".to_string(),
                content: Some(BTreeMap::from([(
                    "application/json".to_string(),
                    MediaType {
                        schema: Some(schema_ref("#/components/schemas/User")),
                    },
                )])),
            },
        );

        let mut users_path = paths
            .remove("/users/{id}")
            .map(|mut p| {
                // POST도 같은 PathItem에 추가
                // 실제로는 /users 경로에 POST를 추가해야 하지만 테스트 편의상 분리
                p
            })
            .unwrap_or_default();

        // /users (POST)
        let mut users_collection = PathItem::default();
        users_collection.post = Some(Operation {
            summary: Some("Create user".to_string()),
            parameters: vec![],
            request_body: Some(RequestBody {
                content: BTreeMap::from([(
                    "application/json".to_string(),
                    MediaType {
                        schema: Some(SchemaObject {
                            schema_type: Some("object".to_string()),
                            properties: Some(BTreeMap::from([(
                                "name".to_string(),
                                schema("string"),
                            )])),
                            items: None,
                            nullable: None,
                            example: None,
                            required: Some(vec!["name".to_string()]),
                            ref_path: None,
                        }),
                    },
                )]),
                required: Some(true),
            }),
            responses: create_user_responses,
        });

        paths.insert("/users/{id}".to_string(), users_path);
        paths.insert("/users".to_string(), users_collection);

        // components.schemas.User ($ref 대상)
        let user_schema = SchemaObject {
            schema_type: Some("object".to_string()),
            properties: Some(BTreeMap::from([
                ("id".to_string(), schema("integer")),
                ("name".to_string(), schema("string")),
            ])),
            items: None,
            nullable: None,
            example: None,
            required: Some(vec!["id".to_string(), "name".to_string()]),
            ref_path: None,
        };

        OpenApiSpec {
            openapi: "3.0.0".to_string(),
            info: OpenApiInfo {
                title: "Test API".to_string(),
                version: "1.0.0".to_string(),
                description: None,
            },
            paths,
            servers: None,
            components: Some(Components {
                schemas: Some(BTreeMap::from([("User".to_string(), user_schema)])),
            }),
        }
    }

    // --- Path 매칭 테스트 ---

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

    // --- 기본 검증 테스트 ---

    #[test]
    fn test_path_not_found() {
        let spec = make_spec();
        let (v, matched, _) = validate_transaction(&spec, "GET", "/nonexistent", 200, None, None);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].violation_type, ViolationType::PathNotFound);
        assert!(matched.is_none());
    }

    #[test]
    fn test_method_not_allowed() {
        let spec = make_spec();
        let (v, _, _) = validate_transaction(&spec, "DELETE", "/items", 200, None, None);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].violation_type, ViolationType::MethodNotAllowed);
    }

    #[test]
    fn test_status_code_mismatch() {
        let spec = make_spec();
        let (v, _, _) = validate_transaction(&spec, "GET", "/users/1", 500, None, None);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].violation_type, ViolationType::StatusCodeMismatch);
    }

    #[test]
    fn test_valid_response() {
        let spec = make_spec();
        let body = serde_json::json!({"id": 1, "name": "Alice"});
        let (v, path, op) = validate_transaction(&spec, "GET", "/users/42", 200, None, Some(&body));
        assert!(v.is_empty(), "Expected no violations, got: {:?}", v);
        assert_eq!(path.as_deref(), Some("/users/{id}"));
        assert_eq!(op.as_deref(), Some("GET"));
    }

    #[test]
    fn test_type_mismatch() {
        let spec = make_spec();
        let body = serde_json::json!({"id": "not-a-number", "name": "Alice"});
        let (v, _, _) = validate_transaction(&spec, "GET", "/users/1", 200, None, Some(&body));
        assert!(v
            .iter()
            .any(|x| x.violation_type == ViolationType::TypeMismatch));
    }

    #[test]
    fn test_extra_field() {
        let spec = make_spec();
        let body = serde_json::json!({"id": 1, "name": "Alice", "extra": true});
        let (v, _, _) = validate_transaction(&spec, "GET", "/users/1", 200, None, Some(&body));
        assert!(v
            .iter()
            .any(|x| x.violation_type == ViolationType::ExtraField));
    }

    #[test]
    fn test_array_body_valid() {
        let spec = make_spec();
        let body = serde_json::json!([{"title": "Item 1"}]);
        let (v, _, _) = validate_transaction(&spec, "GET", "/items", 200, None, Some(&body));
        assert!(v.is_empty());
    }

    #[test]
    fn test_array_item_type_mismatch() {
        let spec = make_spec();
        let body = serde_json::json!([{"title": 123}]);
        let (v, _, _) = validate_transaction(&spec, "GET", "/items", 200, None, Some(&body));
        assert!(v
            .iter()
            .any(|x| x.violation_type == ViolationType::TypeMismatch));
    }

    // --- nullable 테스트 ---

    #[test]
    fn test_null_with_nullable() {
        let mut s = schema("string");
        s.nullable = Some(true);
        let mut violations = Vec::new();
        validate_against_schema(&serde_json::Value::Null, &s, "$", &mut violations, None);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_null_without_nullable() {
        let s = schema("string");
        let mut violations = Vec::new();
        validate_against_schema(&serde_json::Value::Null, &s, "$", &mut violations, None);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].violation_type, ViolationType::TypeMismatch);
    }

    // --- required 필드 테스트 (#7) ---

    #[test]
    fn test_required_field_missing() {
        let spec = make_spec();
        let body = serde_json::json!({"id": 1}); // name 누락
        let (v, _, _) = validate_transaction(&spec, "GET", "/users/1", 200, None, Some(&body));
        assert!(
            v.iter().any(|x| x.violation_type == ViolationType::MissingField
                && x.path.contains("name")),
            "Expected MissingField for 'name', got: {:?}",
            v
        );
    }

    #[test]
    fn test_required_fields_present() {
        let spec = make_spec();
        let body = serde_json::json!({"id": 1, "name": "Alice"});
        let (v, _, _) = validate_transaction(&spec, "GET", "/users/1", 200, None, Some(&body));
        assert!(
            !v.iter()
                .any(|x| x.violation_type == ViolationType::MissingField),
            "Unexpected MissingField: {:?}",
            v
        );
    }

    // --- $ref 해석 테스트 (#8) ---

    #[test]
    fn test_ref_resolution_valid() {
        let spec = make_spec();
        let req_body = serde_json::json!({"name": "Bob"});
        let res_body = serde_json::json!({"id": 1, "name": "Bob"});
        let (v, _, _) = validate_transaction(
            &spec,
            "POST",
            "/users",
            201,
            Some(&req_body),
            Some(&res_body),
        );
        assert!(v.is_empty(), "Expected no violations, got: {:?}", v);
    }

    #[test]
    fn test_ref_resolution_type_mismatch() {
        let spec = make_spec();
        let req_body = serde_json::json!({"name": "Bob"});
        let res_body = serde_json::json!({"id": "not-int", "name": "Bob"});
        let (v, _, _) = validate_transaction(
            &spec,
            "POST",
            "/users",
            201,
            Some(&req_body),
            Some(&res_body),
        );
        assert!(v
            .iter()
            .any(|x| x.violation_type == ViolationType::TypeMismatch));
    }

    #[test]
    fn test_ref_resolution_missing_required() {
        let spec = make_spec();
        let req_body = serde_json::json!({"name": "Bob"});
        let res_body = serde_json::json!({"name": "Bob"}); // id 누락 (required by $ref → User)
        let (v, _, _) = validate_transaction(
            &spec,
            "POST",
            "/users",
            201,
            Some(&req_body),
            Some(&res_body),
        );
        assert!(
            v.iter()
                .any(|x| x.violation_type == ViolationType::MissingField && x.path.contains("id")),
            "Expected MissingField for 'id', got: {:?}",
            v
        );
    }

    // --- 요청 body 검증 테스트 ---

    #[test]
    fn test_request_body_required_missing() {
        let spec = make_spec();
        // POST /users는 request body가 required: true
        let (v, _, _) = validate_transaction(&spec, "POST", "/users", 201, None, None);
        assert!(
            v.iter()
                .any(|x| x.violation_type == ViolationType::MissingField
                    && x.path.contains("request.body")),
            "Expected MissingField for request body, got: {:?}",
            v
        );
    }

    #[test]
    fn test_request_body_valid() {
        let spec = make_spec();
        let req_body = serde_json::json!({"name": "Alice"});
        let (v, _, _) = validate_transaction(&spec, "POST", "/users", 201, Some(&req_body), None);
        assert!(
            !v.iter().any(|x| x.path.contains("request.body")
                && x.violation_type == ViolationType::TypeMismatch),
            "Unexpected request body violation: {:?}",
            v
        );
    }

    #[test]
    fn test_request_body_type_mismatch() {
        let spec = make_spec();
        let req_body = serde_json::json!({"name": 123}); // name은 string이어야 함
        let (v, _, _) = validate_transaction(&spec, "POST", "/users", 201, Some(&req_body), None);
        assert!(
            v.iter()
                .any(|x| x.violation_type == ViolationType::TypeMismatch
                    && x.path.contains("request.body")),
            "Expected TypeMismatch for request body, got: {:?}",
            v
        );
    }

    #[test]
    fn test_request_body_required_field_missing() {
        let spec = make_spec();
        let req_body = serde_json::json!({}); // name 누락 (required)
        let (v, _, _) = validate_transaction(&spec, "POST", "/users", 201, Some(&req_body), None);
        assert!(
            v.iter().any(|x| x.violation_type == ViolationType::MissingField
                && x.path.contains("name")),
            "Expected MissingField for 'name' in request body, got: {:?}",
            v
        );
    }

    // --- 추가 타입 검증 테스트 ---

    #[test]
    fn test_boolean_type_valid() {
        let s = schema("boolean");
        let mut v = Vec::new();
        validate_against_schema(&serde_json::json!(true), &s, "$", &mut v, None);
        assert!(v.is_empty());
    }

    #[test]
    fn test_boolean_type_mismatch() {
        let s = schema("boolean");
        let mut v = Vec::new();
        validate_against_schema(&serde_json::json!("true"), &s, "$", &mut v, None);
        assert_eq!(v[0].violation_type, ViolationType::TypeMismatch);
    }

    #[test]
    fn test_number_type_valid() {
        let s = schema("number");
        let mut v = Vec::new();
        validate_against_schema(&serde_json::json!(3.14), &s, "$", &mut v, None);
        assert!(v.is_empty());
    }

    #[test]
    fn test_integer_accepts_integer_not_float() {
        let s = schema("integer");
        let mut v = Vec::new();
        validate_against_schema(&serde_json::json!(42), &s, "$", &mut v, None);
        assert!(v.is_empty());
    }

    #[test]
    fn test_resolve_ref_not_found() {
        let result = resolve_ref("#/components/schemas/NonExistent", None);
        assert!(result.is_none());
    }

    // --- violation() 헬퍼 테스트 ---

    #[test]
    fn test_violation_creates_correct_fields() {
        let v = violation(
            ViolationType::MissingField,
            "$.body.name",
            "Field 'name' is missing",
            Some("name".to_string()),
            None,
            Severity::Error,
        );
        assert_eq!(v.violation_type, ViolationType::MissingField);
        assert_eq!(v.path, "$.body.name");
        assert_eq!(v.message, "Field 'name' is missing");
        assert_eq!(v.expected, Some("name".to_string()));
        assert_eq!(v.actual, None);
        assert_eq!(v.severity, Severity::Error);
    }

    #[test]
    fn test_violation_with_warning_severity() {
        let v = violation(
            ViolationType::ExtraField,
            "$.body.extra",
            "Extra field",
            None,
            Some("extra".to_string()),
            Severity::Warning,
        );
        assert_eq!(v.severity, Severity::Warning);
        assert_eq!(v.actual, Some("extra".to_string()));
        assert_eq!(v.expected, None);
    }

    #[test]
    fn test_violation_accepts_string_and_str_for_path() {
        let v1 = violation(
            ViolationType::PathNotFound,
            "$",
            "not found",
            None,
            None,
            Severity::Warning,
        );
        let v2 = violation(
            ViolationType::PathNotFound,
            String::from("$.nested"),
            String::from("not found"),
            None,
            None,
            Severity::Warning,
        );
        assert_eq!(v1.path, "$");
        assert_eq!(v2.path, "$.nested");
    }

    // --- type_mismatch() 헬퍼 테스트 ---

    #[test]
    fn test_type_mismatch_creates_correct_violation() {
        let v = type_mismatch("$.body.id", "integer", "string");
        assert_eq!(v.violation_type, ViolationType::TypeMismatch);
        assert_eq!(v.path, "$.body.id");
        assert_eq!(v.message, "Expected type 'integer', got 'string'");
        assert_eq!(v.expected, Some("integer".to_string()));
        assert_eq!(v.actual, Some("string".to_string()));
        assert_eq!(v.severity, Severity::Error);
    }

    #[test]
    fn test_type_mismatch_always_error_severity() {
        let v = type_mismatch("$", "object", "array");
        assert_eq!(v.severity, Severity::Error);
    }

    // --- validate_object() 직접 테스트 ---

    #[test]
    fn test_validate_object_with_valid_object() {
        let s = SchemaObject {
            schema_type: Some("object".to_string()),
            properties: Some(BTreeMap::from([
                ("id".to_string(), schema("integer")),
                ("name".to_string(), schema("string")),
            ])),
            items: None,
            nullable: None,
            example: None,
            required: Some(vec!["id".to_string(), "name".to_string()]),
            ref_path: None,
        };
        let value = serde_json::json!({"id": 1, "name": "Alice"});
        let mut violations = Vec::new();
        validate_object(&value, &s, "$.body", "object", &mut violations, None, 0);
        assert!(
            violations.is_empty(),
            "Expected no violations, got: {:?}",
            violations
        );
    }

    #[test]
    fn test_validate_object_missing_required_fields() {
        let s = SchemaObject {
            schema_type: Some("object".to_string()),
            properties: Some(BTreeMap::from([
                ("id".to_string(), schema("integer")),
                ("name".to_string(), schema("string")),
            ])),
            items: None,
            nullable: None,
            example: None,
            required: Some(vec!["id".to_string(), "name".to_string()]),
            ref_path: None,
        };
        let value = serde_json::json!({}); // 모든 필수 필드 누락
        let mut violations = Vec::new();
        validate_object(&value, &s, "$.body", "object", &mut violations, None, 0);
        assert_eq!(violations.len(), 2);
        assert!(violations
            .iter()
            .all(|v| v.violation_type == ViolationType::MissingField));
        assert!(violations.iter().any(|v| v.path.contains("id")));
        assert!(violations.iter().any(|v| v.path.contains("name")));
    }

    #[test]
    fn test_validate_object_extra_fields() {
        let s = SchemaObject {
            schema_type: Some("object".to_string()),
            properties: Some(BTreeMap::from([("id".to_string(), schema("integer"))])),
            items: None,
            nullable: None,
            example: None,
            required: None,
            ref_path: None,
        };
        let value = serde_json::json!({"id": 1, "extra1": "a", "extra2": "b"});
        let mut violations = Vec::new();
        validate_object(&value, &s, "$.body", "object", &mut violations, None, 0);
        let extra_violations: Vec<_> = violations
            .iter()
            .filter(|v| v.violation_type == ViolationType::ExtraField)
            .collect();
        assert_eq!(extra_violations.len(), 2);
        assert!(extra_violations.iter().any(|v| v.path.contains("extra1")));
        assert!(extra_violations.iter().any(|v| v.path.contains("extra2")));
    }

    #[test]
    fn test_validate_object_type_mismatch_not_object() {
        let s = SchemaObject {
            schema_type: Some("object".to_string()),
            properties: Some(BTreeMap::new()),
            items: None,
            nullable: None,
            example: None,
            required: None,
            ref_path: None,
        };
        let value = serde_json::json!("not an object");
        let mut violations = Vec::new();
        validate_object(&value, &s, "$.body", "string", &mut violations, None, 0);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].violation_type, ViolationType::TypeMismatch);
        assert_eq!(violations[0].expected, Some("object".to_string()));
        assert_eq!(violations[0].actual, Some("string".to_string()));
    }

    // --- validate_array() 직접 테스트 ---

    #[test]
    fn test_validate_array_with_valid_array() {
        let s = SchemaObject {
            schema_type: Some("array".to_string()),
            properties: None,
            items: Some(Box::new(schema("string"))),
            nullable: None,
            example: None,
            required: None,
            ref_path: None,
        };
        let value = serde_json::json!(["hello", "world"]);
        let mut violations = Vec::new();
        validate_array(&value, &s, "$.body", "array", &mut violations, None, 0);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_validate_array_empty_array_no_violations() {
        let s = SchemaObject {
            schema_type: Some("array".to_string()),
            properties: None,
            items: Some(Box::new(schema("string"))),
            nullable: None,
            example: None,
            required: None,
            ref_path: None,
        };
        let value = serde_json::json!([]);
        let mut violations = Vec::new();
        validate_array(&value, &s, "$.body", "array", &mut violations, None, 0);
        assert!(
            violations.is_empty(),
            "Empty array should have no violations"
        );
    }

    #[test]
    fn test_validate_array_wrong_item_type() {
        let s = SchemaObject {
            schema_type: Some("array".to_string()),
            properties: None,
            items: Some(Box::new(schema("integer"))),
            nullable: None,
            example: None,
            required: None,
            ref_path: None,
        };
        let value = serde_json::json!(["not-an-integer"]);
        let mut violations = Vec::new();
        validate_array(&value, &s, "$.body", "array", &mut violations, None, 0);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].violation_type, ViolationType::TypeMismatch);
        assert_eq!(violations[0].path, "$.body[0]");
    }

    #[test]
    fn test_validate_array_type_mismatch_not_array() {
        let s = SchemaObject {
            schema_type: Some("array".to_string()),
            properties: None,
            items: Some(Box::new(schema("string"))),
            nullable: None,
            example: None,
            required: None,
            ref_path: None,
        };
        let value = serde_json::json!({"not": "array"});
        let mut violations = Vec::new();
        validate_array(&value, &s, "$.body", "object", &mut violations, None, 0);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].violation_type, ViolationType::TypeMismatch);
        assert_eq!(violations[0].expected, Some("array".to_string()));
    }
}
