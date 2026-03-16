use std::sync::Arc;

use parking_lot::RwLock;
use proxy_v2_models::openapi::{
    validate_response, ContractSpecInfo, ContractValidationResult, OpenApiSpec,
};
use proxy_v2_models::ClientRequest;
use proxy_v2_models::ClientResponse;
use tracing::{error, info};

/// 로드된 OpenAPI 스펙
pub struct ContractSpec {
    pub id: String,
    pub name: String,
    pub file_path: String,
    pub spec: OpenApiSpec,
    pub enabled: bool,
    pub loaded_at: i64,
}

impl ContractSpec {
    pub fn to_info(&self) -> ContractSpecInfo {
        ContractSpecInfo {
            id: self.id.clone(),
            name: self.name.clone(),
            file_path: self.file_path.clone(),
            enabled: self.enabled,
            path_count: self.spec.paths.len(),
            loaded_at: self.loaded_at,
        }
    }
}

/// OpenAPI 스펙 기반 Contract Testing 검증기
#[derive(Clone)]
pub struct ContractValidator {
    specs: Arc<RwLock<Vec<ContractSpec>>>,
}

impl Default for ContractValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ContractValidator {
    pub fn new() -> Self {
        Self {
            specs: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// OpenAPI 스펙 파일을 로드합니다 (JSON 또는 YAML).
    pub fn load_spec(&self, path: &str) -> Result<ContractSpecInfo, String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("파일 읽기 실패: {}", e))?;

        let spec: OpenApiSpec = if path.ends_with(".yaml") || path.ends_with(".yml") {
            serde_yaml::from_str(&content).map_err(|e| format!("YAML 파싱 실패: {}", e))?
        } else {
            serde_json::from_str(&content).map_err(|e| format!("JSON 파싱 실패: {}", e))?
        };

        let id = format!(
            "{}-{}",
            chrono::Local::now().timestamp_millis(),
            std::process::id()
        );
        let name = spec.info.title.clone();
        let loaded_at = chrono::Local::now()
            .timestamp_nanos_opt()
            .unwrap_or_default();

        let contract_spec = ContractSpec {
            id: id.clone(),
            name: name.clone(),
            file_path: path.to_string(),
            spec,
            enabled: true,
            loaded_at,
        };

        let info = contract_spec.to_info();

        self.specs.write().push(contract_spec);

        info!(
            "[ContractValidator] 스펙 로드: {} ({}개 경로)",
            name, info.path_count
        );

        Ok(info)
    }

    /// 스펙을 제거합니다.
    pub fn unload_spec(&self, id: &str) {
        self.specs.write().retain(|s| s.id != id);
        info!("[ContractValidator] 스펙 제거: {}", id);
    }

    /// 스펙 활성/비활성 토글
    pub fn toggle_spec(&self, id: &str, enabled: bool) {
        if let Some(spec) = self.specs.write().iter_mut().find(|s| s.id == id) {
            spec.enabled = enabled;
        }
    }

    /// 로드된 스펙 목록 반환
    pub fn list_specs(&self) -> Vec<ContractSpecInfo> {
        self.specs.read().iter().map(|s| s.to_info()).collect()
    }

    /// 활성 스펙이 있는지 확인
    pub fn has_active_specs(&self) -> bool {
        self.specs.read().iter().any(|s| s.enabled)
    }

    /// 요청/응답을 모든 활성 스펙에 대해 검증합니다.
    pub fn validate(
        &self,
        req: &ClientRequest,
        res: &ClientResponse,
    ) -> Vec<ContractValidationResult> {
        if !self.has_active_specs() {
            return Vec::new();
        }

        let specs = self.specs.read();
        let mut results = Vec::new();

        let method = req.method().as_str();
        let uri = req.uri().to_string();

        // URI에서 path 부분만 추출
        let request_path = if let Some(path_start) = uri.find("://") {
            // full URL: "https://example.com/path" → "/path"
            uri[path_start + 3..]
                .find('/')
                .map(|i| uri[path_start + 3 + i..].to_string())
                .unwrap_or_else(|| "/".to_string())
        } else if uri.starts_with('/') {
            uri.clone()
        } else {
            format!("/{}", uri)
        };
        // 쿼리 스트링 제거는 match_path_template 내부에서 처리

        let status = res.status().as_u16();

        for spec in specs.iter().filter(|s| s.enabled) {
            let (violations, matched_path, matched_operation) = validate_response(
                &spec.spec,
                method,
                &request_path,
                status,
                res.body_json().as_ref(),
            );

            // PathNotFound는 해당 스펙에 관련없는 경로이므로 결과에 포함하지 않음
            if violations.len() == 1
                && violations[0].violation_type
                    == proxy_v2_models::openapi::ViolationType::PathNotFound
            {
                continue;
            }

            let validated_at = chrono::Local::now()
                .timestamp_nanos_opt()
                .unwrap_or_default();

            results.push(ContractValidationResult {
                request_id: res.id().to_string(),
                spec_id: spec.id.clone(),
                violations,
                validated_at,
                matched_path,
                matched_operation,
            });
        }

        if !results.is_empty() {
            let total_violations: usize = results.iter().map(|r| r.violations.len()).sum();
            if total_violations > 0 {
                error!(
                    "[ContractValidator] {} 위반 발견: {} {}",
                    total_violations, method, request_path
                );
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_spec_json() -> String {
        serde_json::json!({
            "openapi": "3.0.0",
            "info": { "title": "Test API", "version": "1.0.0" },
            "paths": {
                "/users/{id}": {
                    "get": {
                        "responses": {
                            "200": {
                                "description": "Success",
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "type": "object",
                                            "properties": {
                                                "id": { "type": "integer" },
                                                "name": { "type": "string" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        })
        .to_string()
    }

    #[test]
    fn test_load_spec_json() {
        let tmp = tempfile::NamedTempFile::with_suffix(".json").unwrap();
        std::fs::write(tmp.path(), make_spec_json()).unwrap();

        let validator = ContractValidator::new();
        let result = validator.load_spec(tmp.path().to_str().unwrap());
        assert!(result.is_ok());

        let info = result.unwrap();
        assert_eq!(info.name, "Test API");
        assert_eq!(info.path_count, 1);
        assert!(info.enabled);
    }

    #[test]
    fn test_load_spec_yaml() {
        let yaml_content = r#"
openapi: "3.0.0"
info:
  title: "YAML API"
  version: "1.0.0"
paths:
  /hello:
    get:
      responses:
        "200":
          description: "OK"
"#;
        let mut tmp = tempfile::NamedTempFile::with_suffix(".yaml").unwrap();
        tmp.write_all(yaml_content.as_bytes()).unwrap();

        let validator = ContractValidator::new();
        let result = validator.load_spec(tmp.path().to_str().unwrap());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "YAML API");
    }

    #[test]
    fn test_unload_spec() {
        let tmp = tempfile::NamedTempFile::with_suffix(".json").unwrap();
        std::fs::write(tmp.path(), make_spec_json()).unwrap();

        let validator = ContractValidator::new();
        let info = validator.load_spec(tmp.path().to_str().unwrap()).unwrap();
        assert_eq!(validator.list_specs().len(), 1);

        validator.unload_spec(&info.id);
        assert_eq!(validator.list_specs().len(), 0);
    }

    #[test]
    fn test_toggle_spec() {
        let tmp = tempfile::NamedTempFile::with_suffix(".json").unwrap();
        std::fs::write(tmp.path(), make_spec_json()).unwrap();

        let validator = ContractValidator::new();
        let info = validator.load_spec(tmp.path().to_str().unwrap()).unwrap();
        assert!(validator.has_active_specs());

        validator.toggle_spec(&info.id, false);
        assert!(!validator.has_active_specs());

        validator.toggle_spec(&info.id, true);
        assert!(validator.has_active_specs());
    }

    #[test]
    fn test_load_invalid_file() {
        let validator = ContractValidator::new();
        let result = validator.load_spec("/nonexistent/path.json");
        assert!(result.is_err());
    }
}
