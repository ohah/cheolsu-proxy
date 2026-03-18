use serde::{Deserialize, Serialize};

/// GraphQL 오퍼레이션 타입
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GraphqlOperationType {
    Query,
    Mutation,
    Subscription,
}

impl std::fmt::Display for GraphqlOperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraphqlOperationType::Query => write!(f, "query"),
            GraphqlOperationType::Mutation => write!(f, "mutation"),
            GraphqlOperationType::Subscription => write!(f, "subscription"),
        }
    }
}

/// GraphQL 오퍼레이션 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphqlOperation {
    pub operation_type: GraphqlOperationType,
    pub operation_name: Option<String>,
    pub query: String,
    pub variables: Option<serde_json::Value>,
    pub extensions: Option<serde_json::Value>,
}

/// GraphQL 에러 위치 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphqlErrorLocation {
    pub line: u64,
    pub column: u64,
}

/// GraphQL 에러
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphqlError {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locations: Option<Vec<GraphqlErrorLocation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

/// GraphQL 응답
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphqlResponse {
    pub data: Option<serde_json::Value>,
    pub errors: Option<Vec<GraphqlError>>,
    pub extensions: Option<serde_json::Value>,
}

/// UI에 전달되는 GraphQL 오퍼레이션 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphqlOperationInfo {
    /// 고유 ID (트랜잭션 요청 ID)
    pub id: String,
    /// 오퍼레이션 타입 (query, mutation, subscription)
    pub operation_type: GraphqlOperationType,
    /// 오퍼레이션 이름 (named query인 경우)
    pub operation_name: Option<String>,
    /// GraphQL 쿼리 문자열
    pub query: String,
    /// 변수 (JSON)
    pub variables: Option<serde_json::Value>,
    /// 엔드포인트 URI
    pub endpoint: String,
    /// 요청 시간 (나노초)
    pub request_time: i64,
    /// 응답 시간 (나노초, 응답이 있는 경우)
    pub response_time: Option<i64>,
    /// 응답 데이터
    pub response_data: Option<serde_json::Value>,
    /// 응답 에러
    pub response_errors: Option<Vec<GraphqlError>>,
    /// 응답 extensions
    pub response_extensions: Option<serde_json::Value>,
    /// 배치 인덱스 (배치 쿼리인 경우)
    pub batch_index: Option<usize>,
    /// 배치 크기 (배치 쿼리인 경우)
    pub batch_size: Option<usize>,
    /// HTTP 상태 코드
    pub status_code: Option<u16>,
}

/// GraphQL 요청인지 감지
///
/// - POST 요청이고 JSON body에 "query" 필드가 있는 경우
/// - GET 요청이고 query 파라미터에 "query"가 있는 경우
/// - Content-Type에 "graphql"이 포함된 경우
pub fn is_graphql(method: &str, uri: &str, content_type: Option<&str>, body: Option<&str>) -> bool {
    // Content-Type에 graphql이 포함된 경우
    if let Some(ct) = content_type {
        if ct.contains("graphql") {
            return true;
        }
    }

    match method.to_uppercase().as_str() {
        "POST" => {
            if let Some(body_str) = body {
                let trimmed = body_str.trim();
                // JSON array (배치 쿼리) 또는 JSON object에 "query" 필드가 있는지 확인
                if trimmed.starts_with('[') {
                    // 배치 쿼리: 배열의 첫 번째 요소에 "query" 필드가 있는지 확인
                    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(trimmed) {
                        return arr.first().is_some_and(|v| v.get("query").is_some());
                    }
                } else if trimmed.starts_with('{') {
                    if let Ok(obj) = serde_json::from_str::<serde_json::Value>(trimmed) {
                        return obj.get("query").is_some();
                    }
                }
            }
            false
        }
        "GET" => {
            // GET 요청의 query 파라미터에 "query"가 있는지 확인
            if let Some(query_start) = uri.find('?') {
                let query_string = &uri[query_start + 1..];
                return query_string
                    .split('&')
                    .any(|param| param.starts_with("query=") || param == "query");
            }
            false
        }
        _ => false,
    }
}

/// 주석과 공백을 건너뛴 query 문자열 반환
fn skip_comments_and_whitespace(query: &str) -> &str {
    let mut s = query.trim();
    while s.starts_with('#') {
        // 줄 끝까지 건너뛰기
        if let Some(newline_pos) = s.find('\n') {
            s = s[newline_pos + 1..].trim();
        } else {
            // 주석만 있고 개행이 없는 경우
            return "";
        }
    }
    s
}

/// query 문자열에서 오퍼레이션 타입을 파싱
fn parse_operation_type(query: &str) -> GraphqlOperationType {
    let trimmed = skip_comments_and_whitespace(query);
    if trimmed.starts_with("mutation") {
        GraphqlOperationType::Mutation
    } else if trimmed.starts_with("subscription") {
        GraphqlOperationType::Subscription
    } else {
        // "query"로 시작하거나 축약형 (바로 { 로 시작)
        GraphqlOperationType::Query
    }
}

/// query 문자열에서 오퍼레이션 이름을 파싱
fn parse_operation_name_from_query(query: &str) -> Option<String> {
    let trimmed = skip_comments_and_whitespace(query);
    // "query OperationName" 또는 "mutation OperationName" 패턴 파싱
    let after_keyword = trimmed
        .strip_prefix("query")
        .or_else(|| trimmed.strip_prefix("mutation"))
        .or_else(|| trimmed.strip_prefix("subscription"));

    if let Some(rest) = after_keyword {
        let rest = rest.trim_start();
        if rest.starts_with('{') || rest.starts_with('(') || rest.is_empty() {
            return None;
        }
        // 이름 부분 추출 (공백, (, { 전까지)
        let name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if name.is_empty() {
            None
        } else {
            Some(name)
        }
    } else {
        None
    }
}

/// JSON body에서 GraphQL 오퍼레이션을 파싱
pub fn parse_graphql_request(body: &str) -> Vec<GraphqlOperation> {
    let trimmed = body.trim();

    if trimmed.starts_with('[') {
        // 배치 쿼리
        if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(trimmed) {
            return arr.iter().filter_map(parse_single_operation).collect();
        }
    } else if trimmed.starts_with('{') {
        if let Ok(obj) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if let Some(op) = parse_single_operation(&obj) {
                return vec![op];
            }
        }
    }

    Vec::new()
}

/// 단일 GraphQL 오퍼레이션을 파싱
fn parse_single_operation(value: &serde_json::Value) -> Option<GraphqlOperation> {
    let query = value.get("query")?.as_str()?.to_string();
    let operation_name = value
        .get("operationName")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| parse_operation_name_from_query(&query));
    let variables = value.get("variables").cloned();
    let extensions = value.get("extensions").cloned();
    let operation_type = parse_operation_type(&query);

    Some(GraphqlOperation {
        operation_type,
        operation_name,
        query,
        variables,
        extensions,
    })
}

/// GET 요청의 query 파라미터에서 GraphQL 오퍼레이션을 파싱
pub fn parse_graphql_request_from_query_params(uri: &str) -> Vec<GraphqlOperation> {
    let query_start = match uri.find('?') {
        Some(pos) => pos + 1,
        None => return Vec::new(),
    };
    let query_string = &uri[query_start..];

    let mut query = None;
    let mut operation_name = None;
    let mut variables = None;
    let mut extensions = None;

    for param in query_string.split('&') {
        if let Some((key, value)) = param.split_once('=') {
            let decoded_value = urlencoding_decode(value).unwrap_or_else(|| value.to_string());
            match key {
                "query" => query = Some(decoded_value),
                "operationName" => operation_name = Some(decoded_value),
                "variables" => {
                    variables = serde_json::from_str(&decoded_value).ok();
                }
                "extensions" => {
                    extensions = serde_json::from_str(&decoded_value).ok();
                }
                _ => {}
            }
        }
    }

    if let Some(query_str) = query {
        let op_name = operation_name.or_else(|| parse_operation_name_from_query(&query_str));
        let operation_type = parse_operation_type(&query_str);
        vec![GraphqlOperation {
            operation_type,
            operation_name: op_name,
            query: query_str,
            variables,
            extensions,
        }]
    } else {
        Vec::new()
    }
}

/// 간단한 URL 디코딩 (percent-encoding)
fn urlencoding_decode(input: &str) -> Option<String> {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                } else {
                    result.push('%');
                    result.push_str(&hex);
                }
            } else {
                result.push('%');
                result.push_str(&hex);
            }
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    Some(result)
}

/// 응답 body에서 GraphQL 응답을 파싱
pub fn parse_graphql_response(body: &str) -> Vec<GraphqlResponse> {
    let trimmed = body.trim();

    if trimmed.starts_with('[') {
        // 배치 응답
        if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(trimmed) {
            return arr.iter().filter_map(parse_single_response).collect();
        }
    } else if trimmed.starts_with('{') {
        if let Ok(obj) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if let Some(resp) = parse_single_response(&obj) {
                return vec![resp];
            }
        }
    }

    Vec::new()
}

/// 단일 GraphQL 응답을 파싱
fn parse_single_response(value: &serde_json::Value) -> Option<GraphqlResponse> {
    // GraphQL 응답에는 최소한 "data" 또는 "errors"가 있어야 함
    if value.get("data").is_none() && value.get("errors").is_none() {
        return None;
    }

    let data = value.get("data").cloned();
    let errors: Option<Vec<GraphqlError>> = value
        .get("errors")
        .and_then(|v| serde_json::from_value(v.clone()).ok());
    let extensions = value.get("extensions").cloned();

    Some(GraphqlResponse {
        data,
        errors,
        extensions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- is_graphql 테스트 ---

    #[test]
    fn test_is_graphql_post_with_query_field() {
        let body = r#"{"query":"{ user { name } }","variables":{}}"#;
        assert!(is_graphql("POST", "/graphql", None, Some(body)));
    }

    #[test]
    fn test_is_graphql_post_without_query_field() {
        let body = r#"{"data":"hello"}"#;
        assert!(!is_graphql("POST", "/api", None, Some(body)));
    }

    #[test]
    fn test_is_graphql_get_with_query_param() {
        assert!(is_graphql(
            "GET",
            "/graphql?query=%7B+user+%7B+name+%7D+%7D",
            None,
            None
        ));
    }

    #[test]
    fn test_is_graphql_get_without_query_param() {
        assert!(!is_graphql("GET", "/api?foo=bar", None, None));
    }

    #[test]
    fn test_is_graphql_content_type_graphql() {
        assert!(is_graphql(
            "POST",
            "/graphql",
            Some("application/graphql+json"),
            None
        ));
    }

    #[test]
    fn test_is_graphql_batch_query() {
        let body = r#"[{"query":"{ user { name } }"},{"query":"mutation { createUser { id } }"}]"#;
        assert!(is_graphql("POST", "/graphql", None, Some(body)));
    }

    #[test]
    fn test_is_graphql_non_json_body() {
        assert!(!is_graphql("POST", "/graphql", None, Some("not json")));
    }

    // --- parse_graphql_request 테스트 ---

    #[test]
    fn test_parse_single_query() {
        let body = r#"{"query":"query GetUser($id: ID!) { user(id: $id) { name } }","variables":{"id":"1"},"operationName":"GetUser"}"#;
        let ops = parse_graphql_request(body);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].operation_type, GraphqlOperationType::Query);
        assert_eq!(ops[0].operation_name, Some("GetUser".to_string()));
        assert!(ops[0].variables.is_some());
    }

    #[test]
    fn test_parse_mutation() {
        let body = r#"{"query":"mutation CreateUser { createUser { id } }"}"#;
        let ops = parse_graphql_request(body);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].operation_type, GraphqlOperationType::Mutation);
        assert_eq!(ops[0].operation_name, Some("CreateUser".to_string()));
    }

    #[test]
    fn test_parse_subscription() {
        let body = r#"{"query":"subscription OnMessage { messageAdded { text } }"}"#;
        let ops = parse_graphql_request(body);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].operation_type, GraphqlOperationType::Subscription);
        assert_eq!(ops[0].operation_name, Some("OnMessage".to_string()));
    }

    #[test]
    fn test_parse_anonymous_query() {
        let body = r#"{"query":"{ user { name } }"}"#;
        let ops = parse_graphql_request(body);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].operation_type, GraphqlOperationType::Query);
        assert_eq!(ops[0].operation_name, None);
    }

    #[test]
    fn test_parse_batch_query() {
        let body = r#"[{"query":"{ user { name } }"},{"query":"mutation { deleteUser(id: 1) }"}]"#;
        let ops = parse_graphql_request(body);
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0].operation_type, GraphqlOperationType::Query);
        assert_eq!(ops[1].operation_type, GraphqlOperationType::Mutation);
    }

    #[test]
    fn test_parse_empty_body() {
        let ops = parse_graphql_request("");
        assert!(ops.is_empty());
    }

    #[test]
    fn test_parse_invalid_json() {
        let ops = parse_graphql_request("not json");
        assert!(ops.is_empty());
    }

    // --- parse_graphql_response 테스트 ---

    #[test]
    fn test_parse_response_with_data() {
        let body = r#"{"data":{"user":{"name":"Alice"}}}"#;
        let responses = parse_graphql_response(body);
        assert_eq!(responses.len(), 1);
        assert!(responses[0].data.is_some());
        assert!(responses[0].errors.is_none());
    }

    #[test]
    fn test_parse_response_with_errors() {
        let body = r#"{"data":null,"errors":[{"message":"Not found","locations":[{"line":1,"column":3}]}]}"#;
        let responses = parse_graphql_response(body);
        assert_eq!(responses.len(), 1);
        let errors = responses[0].errors.as_ref().unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "Not found");
        assert!(errors[0].locations.is_some());
    }

    #[test]
    fn test_parse_response_with_extensions() {
        let body = r#"{"data":{"user":null},"extensions":{"complexity":42}}"#;
        let responses = parse_graphql_response(body);
        assert_eq!(responses.len(), 1);
        assert!(responses[0].extensions.is_some());
    }

    #[test]
    fn test_parse_batch_response() {
        let body =
            r#"[{"data":{"user":{"name":"Alice"}}},{"data":null,"errors":[{"message":"Error"}]}]"#;
        let responses = parse_graphql_response(body);
        assert_eq!(responses.len(), 2);
        assert!(responses[0].data.is_some());
        assert!(responses[1].errors.is_some());
    }

    #[test]
    fn test_parse_non_graphql_response() {
        let body = r#"{"result":"ok"}"#;
        let responses = parse_graphql_response(body);
        assert!(responses.is_empty());
    }

    // --- parse_operation_type 테스트 ---

    #[test]
    fn test_operation_type_query_explicit() {
        assert_eq!(
            parse_operation_type("query { user { name } }"),
            GraphqlOperationType::Query
        );
    }

    #[test]
    fn test_operation_type_query_shorthand() {
        assert_eq!(
            parse_operation_type("{ user { name } }"),
            GraphqlOperationType::Query
        );
    }

    #[test]
    fn test_operation_type_mutation() {
        assert_eq!(
            parse_operation_type("mutation CreateUser { createUser { id } }"),
            GraphqlOperationType::Mutation
        );
    }

    #[test]
    fn test_operation_type_subscription() {
        assert_eq!(
            parse_operation_type("subscription OnMessage { messageAdded { text } }"),
            GraphqlOperationType::Subscription
        );
    }

    // --- parse_operation_name_from_query 테스트 ---

    #[test]
    fn test_operation_name_named_query() {
        assert_eq!(
            parse_operation_name_from_query("query GetUser { user { name } }"),
            Some("GetUser".to_string())
        );
    }

    #[test]
    fn test_operation_name_named_mutation() {
        assert_eq!(
            parse_operation_name_from_query("mutation CreateUser($input: CreateUserInput!) { createUser(input: $input) { id } }"),
            Some("CreateUser".to_string())
        );
    }

    #[test]
    fn test_operation_name_anonymous() {
        assert_eq!(parse_operation_name_from_query("{ user { name } }"), None);
    }

    #[test]
    fn test_operation_name_anonymous_query() {
        assert_eq!(
            parse_operation_name_from_query("query { user { name } }"),
            None
        );
    }

    // --- parse_graphql_request_from_query_params 테스트 ---

    #[test]
    fn test_parse_get_query_params() {
        let uri = "/graphql?query=%7B+user+%7B+name+%7D+%7D&operationName=GetUser";
        let ops = parse_graphql_request_from_query_params(uri);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].operation_name, Some("GetUser".to_string()));
    }

    #[test]
    fn test_parse_get_no_query() {
        let ops = parse_graphql_request_from_query_params("/graphql?foo=bar");
        assert!(ops.is_empty());
    }

    // --- GraphqlOperationInfo serde 테스트 ---

    #[test]
    fn test_operation_info_serialization() {
        let info = GraphqlOperationInfo {
            id: "test-id".to_string(),
            operation_type: GraphqlOperationType::Query,
            operation_name: Some("GetUser".to_string()),
            query: "query GetUser { user { name } }".to_string(),
            variables: None,
            endpoint: "https://api.example.com/graphql".to_string(),
            request_time: 1234567890,
            response_time: Some(1234567900),
            response_data: Some(serde_json::json!({"user": {"name": "Alice"}})),
            response_errors: None,
            response_extensions: None,
            batch_index: None,
            batch_size: None,
            status_code: Some(200),
        };
        let json = serde_json::to_string(&info).unwrap();
        let deserialized: GraphqlOperationInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "test-id");
        assert_eq!(deserialized.operation_type, GraphqlOperationType::Query);
        assert_eq!(deserialized.operation_name, Some("GetUser".to_string()));
    }

    // --- urlencoding_decode 테스트 ---

    #[test]
    fn test_url_decode_basic() {
        assert_eq!(
            urlencoding_decode("%7B+user+%7D"),
            Some("{ user }".to_string())
        );
    }

    #[test]
    fn test_url_decode_plain() {
        assert_eq!(urlencoding_decode("hello"), Some("hello".to_string()));
    }

    // --- GraphqlOperationType Display 테스트 ---

    #[test]
    fn test_operation_type_display() {
        assert_eq!(format!("{}", GraphqlOperationType::Query), "query");
        assert_eq!(format!("{}", GraphqlOperationType::Mutation), "mutation");
        assert_eq!(
            format!("{}", GraphqlOperationType::Subscription),
            "subscription"
        );
    }

    // --- 엣지 케이스 테스트 ---

    #[test]
    fn test_empty_query_string() {
        let body = r#"{"query":""}"#;
        let ops = parse_graphql_request(body);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].query, "");
        assert_eq!(ops[0].operation_type, GraphqlOperationType::Query);
    }

    #[test]
    fn test_invalid_json_body() {
        let ops = parse_graphql_request("{invalid json}");
        assert!(ops.is_empty());
    }

    #[test]
    fn test_query_field_is_number() {
        let body = r#"{"query":42}"#;
        let ops = parse_graphql_request(body);
        assert!(ops.is_empty());
    }

    #[test]
    fn test_query_field_is_null() {
        let body = r#"{"query":null}"#;
        let ops = parse_graphql_request(body);
        assert!(ops.is_empty());
    }

    #[test]
    fn test_variables_null() {
        let body = r#"{"query":"{ user { name } }","variables":null}"#;
        let ops = parse_graphql_request(body);
        assert_eq!(ops.len(), 1);
        // variables 필드가 존재하되 null 값
        assert!(ops[0].variables.is_some());
        assert!(ops[0].variables.as_ref().unwrap().is_null());
    }

    #[test]
    fn test_variables_absent() {
        let body = r#"{"query":"{ user { name } }"}"#;
        let ops = parse_graphql_request(body);
        assert_eq!(ops.len(), 1);
        // variables 필드가 없는 경우
        assert!(ops[0].variables.is_none());
    }

    #[test]
    fn test_very_long_query_string() {
        let long_field = "a".repeat(10_000);
        let body = format!(
            r#"{{"query":"query {} {{ user {{ name }} }}"}}"#,
            long_field
        );
        let ops = parse_graphql_request(&body);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].operation_type, GraphqlOperationType::Query);
        assert_eq!(ops[0].operation_name, Some(long_field));
    }

    #[test]
    fn test_operation_name_with_special_chars() {
        // 특수문자는 이름으로 인식하지 않으므로 underscore와 영숫자만 이름으로 추출
        let body = r#"{"query":"query Get_User123 { user { name } }"}"#;
        let ops = parse_graphql_request(body);
        assert_eq!(ops[0].operation_name, Some("Get_User123".to_string()));
    }

    #[test]
    fn test_operation_name_starts_with_special_char() {
        // 이름이 특수문자로 시작하면 이름을 추출할 수 없음
        let body = r#"{"query":"query @directive { user { name } }"}"#;
        let ops = parse_graphql_request(body);
        assert_eq!(ops[0].operation_name, None);
    }

    #[test]
    fn test_batch_query_empty_array() {
        let body = "[]";
        let ops = parse_graphql_request(body);
        assert!(ops.is_empty());
    }

    #[test]
    fn test_batch_query_partially_valid() {
        let body = r#"[{"query":"{ user { name } }"},{"not_query":"value"},{"query":"mutation { deleteUser }"}]"#;
        let ops = parse_graphql_request(body);
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0].operation_type, GraphqlOperationType::Query);
        assert_eq!(ops[1].operation_type, GraphqlOperationType::Mutation);
    }

    #[test]
    fn test_nested_fragments_operation_type() {
        let body = r#"{"query":"mutation CreateUser { createUser { ...UserFields } } fragment UserFields on User { id name email }"}"#;
        let ops = parse_graphql_request(body);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].operation_type, GraphqlOperationType::Mutation);
        assert_eq!(ops[0].operation_name, Some("CreateUser".to_string()));
    }

    #[test]
    fn test_is_graphql_content_type_with_charset() {
        assert!(is_graphql(
            "POST",
            "/graphql",
            Some("application/graphql+json; charset=utf-8"),
            None
        ));
    }

    #[test]
    fn test_persisted_query_no_query_field() {
        // persisted query: extensions에 persistedQuery hash만 있고 query가 없는 경우
        let body = r#"{"extensions":{"persistedQuery":{"version":1,"sha256Hash":"abc123"}}}"#;
        let ops = parse_graphql_request(body);
        // query 필드가 없으므로 파싱되지 않음
        assert!(ops.is_empty());
    }

    #[test]
    fn test_persisted_query_with_empty_query() {
        // persisted query에 빈 query가 포함된 경우
        let body =
            r#"{"query":"","extensions":{"persistedQuery":{"version":1,"sha256Hash":"abc123"}}}"#;
        let ops = parse_graphql_request(body);
        assert_eq!(ops.len(), 1);
        assert!(ops[0].extensions.is_some());
    }

    #[test]
    fn test_multiline_query() {
        let body = r#"{"query":"query GetUser {\n  user {\n    name\n    email\n  }\n}"}"#;
        let ops = parse_graphql_request(body);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].operation_type, GraphqlOperationType::Query);
        assert_eq!(ops[0].operation_name, Some("GetUser".to_string()));
    }

    #[test]
    fn test_query_with_comments() {
        let body =
            r##"{"query":"# This is a comment\nmutation CreateUser { createUser { id } }"}"##;
        let ops = parse_graphql_request(body);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].operation_type, GraphqlOperationType::Mutation);
        assert_eq!(ops[0].operation_name, Some("CreateUser".to_string()));
    }

    #[test]
    fn test_query_with_multiple_comments() {
        let body =
            r##"{"query":"# comment 1\n# comment 2\nsubscription OnEvent { event { data } }"}"##;
        let ops = parse_graphql_request(body);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].operation_type, GraphqlOperationType::Subscription);
        assert_eq!(ops[0].operation_name, Some("OnEvent".to_string()));
    }

    #[test]
    fn test_query_with_inline_comment_only() {
        let body = r##"{"query":"# just a comment"}"##;
        let ops = parse_graphql_request(body);
        assert_eq!(ops.len(), 1);
        // 주석만 있는 경우 기본적으로 Query로 파싱됨
        assert_eq!(ops[0].operation_type, GraphqlOperationType::Query);
    }

    #[test]
    fn test_is_graphql_post_no_body() {
        assert!(!is_graphql("POST", "/graphql", None, None));
    }

    #[test]
    fn test_is_graphql_put_method() {
        let body = r#"{"query":"{ user { name } }"}"#;
        assert!(!is_graphql("PUT", "/graphql", None, Some(body)));
    }

    #[test]
    fn test_is_graphql_batch_empty_array() {
        assert!(!is_graphql("POST", "/graphql", None, Some("[]")));
    }

    #[test]
    fn test_is_graphql_batch_first_item_no_query() {
        let body = r#"[{"data":"test"},{"query":"{ user { name } }"}]"#;
        assert!(!is_graphql("POST", "/graphql", None, Some(body)));
    }

    #[test]
    fn test_parse_get_with_variables() {
        let uri = "/graphql?query=%7B+user+%7D&variables=%7B%22id%22%3A1%7D";
        let ops = parse_graphql_request_from_query_params(uri);
        assert_eq!(ops.len(), 1);
        assert!(ops[0].variables.is_some());
    }

    #[test]
    fn test_parse_get_with_extensions() {
        let uri = "/graphql?query=%7B+user+%7D&extensions=%7B%22key%22%3A%22val%22%7D";
        let ops = parse_graphql_request_from_query_params(uri);
        assert_eq!(ops.len(), 1);
        assert!(ops[0].extensions.is_some());
    }

    #[test]
    fn test_parse_get_no_query_string() {
        let ops = parse_graphql_request_from_query_params("/graphql");
        assert!(ops.is_empty());
    }

    #[test]
    fn test_response_only_errors() {
        let body = r#"{"errors":[{"message":"Unauthorized"}]}"#;
        let responses = parse_graphql_response(body);
        assert_eq!(responses.len(), 1);
        assert!(responses[0].data.is_none());
        assert!(responses[0].errors.is_some());
    }

    #[test]
    fn test_response_empty_body() {
        let responses = parse_graphql_response("");
        assert!(responses.is_empty());
    }

    #[test]
    fn test_response_invalid_json() {
        let responses = parse_graphql_response("not json");
        assert!(responses.is_empty());
    }

    #[test]
    fn test_url_decode_incomplete_percent() {
        // 끝에 불완전한 % 인코딩
        assert_eq!(urlencoding_decode("%7"), Some("%7".to_string()));
    }

    #[test]
    fn test_url_decode_invalid_hex() {
        assert_eq!(urlencoding_decode("%ZZ"), Some("%ZZ".to_string()));
    }

    #[test]
    fn test_skip_comments_and_whitespace() {
        assert_eq!(
            skip_comments_and_whitespace("# comment\nmutation { }"),
            "mutation { }"
        );
        assert_eq!(skip_comments_and_whitespace("query { }"), "query { }");
        assert_eq!(skip_comments_and_whitespace("# only comment"), "");
        assert_eq!(
            skip_comments_and_whitespace("# c1\n# c2\n{ user }"),
            "{ user }"
        );
    }
}
