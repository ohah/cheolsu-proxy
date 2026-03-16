use super::path_inference::infer_path_template;
use super::schema_inference::{infer_schema, merge_schemas};
use super::types::*;
use crate::RequestInfo;
use std::collections::{BTreeMap, HashMap};

/// 캡처된 트래픽에서 OpenAPI 3.0 스펙을 생성합니다.
pub fn build_openapi_spec(transactions: &[RequestInfo]) -> OpenApiSpec {
    let mut path_operations: HashMap<(String, String), Vec<&RequestInfo>> = HashMap::new();
    let mut servers: std::collections::HashSet<String> = std::collections::HashSet::new();

    // 1. 트랜잭션을 (경로, 메서드)로 그룹핑
    for info in transactions {
        let Some(req) = info.0.as_ref() else {
            continue;
        };

        let uri = req.uri().to_string();
        let method = req.method().to_string().to_lowercase();

        // 서버 URL 추출
        if let Ok(parsed) = uri.parse::<http::Uri>() {
            if let (Some(scheme), Some(authority)) = (parsed.scheme_str(), parsed.authority()) {
                servers.insert(format!("{}://{}", scheme, authority));
            }
        }

        // 경로 추출 (쿼리 제거)
        let path = uri
            .parse::<http::Uri>()
            .ok()
            .and_then(|u| u.path_and_query().map(|pq| pq.path().to_string()))
            .unwrap_or_else(|| uri.split('?').next().unwrap_or(&uri).to_string());

        path_operations
            .entry((path, method))
            .or_default()
            .push(info);
    }

    // 2. 경로 파라미터 추론
    let all_paths: Vec<String> = path_operations.keys().map(|(p, _)| p.clone()).collect();
    let templates = infer_path_template(&all_paths);

    // 3. PathItem 구성
    let mut paths: BTreeMap<String, PathItem> = BTreeMap::new();

    for ((path, method), infos) in &path_operations {
        // 템플릿화된 경로 찾기
        let template_path = templates
            .iter()
            .find(|(_, originals)| originals.contains(path))
            .map(|(t, _)| t.clone())
            .unwrap_or_else(|| path.clone());

        let operation = build_operation(infos);

        let path_item = paths.entry(template_path).or_default();
        match method.as_str() {
            "get" => path_item.get = Some(operation),
            "post" => path_item.post = Some(operation),
            "put" => path_item.put = Some(operation),
            "delete" => path_item.delete = Some(operation),
            "patch" => path_item.patch = Some(operation),
            "head" => path_item.head = Some(operation),
            "options" => path_item.options = Some(operation),
            _ => {}
        }
    }

    let servers_vec: Vec<OpenApiServer> = servers
        .into_iter()
        .map(|url| OpenApiServer {
            url,
            description: None,
        })
        .collect();

    OpenApiSpec {
        openapi: "3.0.3".to_string(),
        info: OpenApiInfo {
            title: "Auto-generated API Spec".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Generated from captured HTTP traffic by Cheolsu Proxy".to_string()),
        },
        paths,
        servers: if servers_vec.is_empty() {
            None
        } else {
            Some(servers_vec)
        },
    }
}

fn build_operation(infos: &[&RequestInfo]) -> Operation {
    let mut response_schemas: BTreeMap<String, Vec<SchemaObject>> = BTreeMap::new();
    let mut request_schemas: Vec<SchemaObject> = Vec::new();
    let mut has_request_body = false;

    for info in infos {
        // 요청 바디 스키마 수집
        if let Some(req) = info.0.as_ref() {
            if let Some(body_json) = req.body_json() {
                has_request_body = true;
                request_schemas.push(infer_schema(body_json));
            }
        }

        // 응답 스키마 수집 (상태 코드별 그룹핑)
        if let Some(res) = info.1.as_ref() {
            let status = res.status().as_u16().to_string();
            if let Some(body_json) = res.body_json() {
                response_schemas
                    .entry(status)
                    .or_default()
                    .push(infer_schema(body_json));
            } else {
                response_schemas.entry(status).or_default();
            }
        }
    }

    // 요청 바디
    let request_body = if has_request_body && !request_schemas.is_empty() {
        let schema = merge_schemas(&request_schemas);
        let mut content = BTreeMap::new();
        content.insert(
            "application/json".to_string(),
            MediaType {
                schema: Some(schema),
            },
        );
        Some(RequestBody {
            content,
            required: Some(true),
        })
    } else {
        None
    };

    // 응답
    let mut responses: BTreeMap<String, ResponseSpec> = BTreeMap::new();
    for (status, schemas) in &response_schemas {
        let content = if !schemas.is_empty() {
            let schema = merge_schemas(schemas);
            let mut content = BTreeMap::new();
            content.insert(
                "application/json".to_string(),
                MediaType {
                    schema: Some(schema),
                },
            );
            Some(content)
        } else {
            None
        };
        responses.insert(
            status.clone(),
            ResponseSpec {
                description: status_description(status),
                content,
            },
        );
    }

    if responses.is_empty() {
        responses.insert(
            "200".to_string(),
            ResponseSpec {
                description: "OK".to_string(),
                content: None,
            },
        );
    }

    Operation {
        summary: None,
        parameters: Vec::new(),
        request_body,
        responses,
    }
}

fn status_description(status: &str) -> String {
    match status {
        "200" => "OK".to_string(),
        "201" => "Created".to_string(),
        "204" => "No Content".to_string(),
        "400" => "Bad Request".to_string(),
        "401" => "Unauthorized".to_string(),
        "403" => "Forbidden".to_string(),
        "404" => "Not Found".to_string(),
        "500" => "Internal Server Error".to_string(),
        _ => format!("Response {}", status),
    }
}

/// OpenAPI 스펙을 JSON 문자열로 변환합니다.
pub fn build_openapi_json(transactions: &[RequestInfo]) -> Result<String, serde_json::Error> {
    let spec = build_openapi_spec(transactions);
    serde_json::to_string_pretty(&spec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ProxiedRequest, ProxiedResponse};
    use bytes::Bytes;
    use http::{HeaderMap, HeaderValue, Method, StatusCode, Uri, Version};

    fn make_request_info(method: &str, uri: &str, status: u16, body_json: &str) -> RequestInfo {
        let mut req_headers = HeaderMap::new();
        req_headers.insert("content-type", HeaderValue::from_static("application/json"));

        let req = ProxiedRequest::new(
            method.parse::<Method>().unwrap(),
            uri.parse::<Uri>().unwrap(),
            Version::HTTP_11,
            req_headers,
            Bytes::from(body_json.to_string()),
            1000,
        );

        let mut res_headers = HeaderMap::new();
        res_headers.insert("content-type", HeaderValue::from_static("application/json"));

        let res = ProxiedResponse::new(
            StatusCode::from_u16(status).unwrap(),
            Version::HTTP_11,
            res_headers,
            Bytes::from(body_json.to_string()),
            2000,
        );

        let client_req = req.for_client(None);
        let client_res = res.for_client(client_req.id(), None);

        RequestInfo(Some(client_req), Some(client_res), None)
    }

    #[test]
    fn test_empty_transactions() {
        let spec = build_openapi_spec(&[]);
        assert_eq!(spec.openapi, "3.0.3");
        assert!(spec.paths.is_empty());
    }

    #[test]
    fn test_build_openapi_json() {
        let result = build_openapi_json(&[]);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains("3.0.3"));
    }

    #[test]
    fn test_build_with_transactions() {
        let info = make_request_info(
            "GET",
            "http://api.example.com/users",
            200,
            r#"[{"id":1,"name":"Alice"}]"#,
        );
        let spec = build_openapi_spec(&[info]);

        assert!(!spec.paths.is_empty());
        assert!(spec.servers.is_some());
    }

    #[test]
    fn test_status_description_known() {
        assert_eq!(status_description("200"), "OK");
        assert_eq!(status_description("404"), "Not Found");
    }

    #[test]
    fn test_status_description_unknown() {
        assert_eq!(status_description("418"), "Response 418");
    }
}
