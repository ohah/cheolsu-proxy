use crate::transpiler::transpile_ts;
use crate::types::{
    RequestAction, ResponseAction, ScriptRequest, ScriptResponse, ScriptWsMessage, WsAction,
};
use deno_core::JsRuntime;
use tracing::info;

const RUNTIME_JS: &str = include_str!("runtime.js");

/// deno_core 기반 TypeScript/JavaScript 스크립팅 엔진
pub struct ScriptEngine {
    runtime: JsRuntime,
    has_on_request: bool,
    has_on_response: bool,
    has_on_ws_message: bool,
}

impl ScriptEngine {
    /// 새 스크립트 엔진 생성 (runtime.js 로드)
    pub fn new() -> Result<Self, String> {
        let mut runtime = JsRuntime::new(Default::default());

        runtime
            .execute_script("<runtime>".to_string(), RUNTIME_JS.to_string())
            .map_err(|e| format!("런타임 초기화 실패: {}", e))?;

        Ok(Self {
            runtime,
            has_on_request: false,
            has_on_response: false,
            has_on_ws_message: false,
        })
    }

    /// 사용자 스크립트 파일 로드 (JS/TS 모두 지원)
    pub fn load_script(&mut self, path: &str) -> Result<(), String> {
        let script_path = std::path::Path::new(path);
        if !script_path.exists() {
            return Err(format!("스크립트 파일을 찾을 수 없습니다: {}", path));
        }

        let source = std::fs::read_to_string(script_path)
            .map_err(|e| format!("스크립트 파일 읽기 실패: {}", e))?;

        let code = if path.ends_with(".ts") || path.ends_with(".tsx") {
            transpile_ts(&source, path)?
        } else {
            source
        };

        self.runtime
            .execute_script(path.to_string(), code)
            .map_err(|e| format!("스크립트 실행 실패: {}", e))?;

        self.check_hooks()?;

        info!(
            "[Script] 로드 완료: {} (onRequest={}, onResponse={}, onWsMessage={})",
            path, self.has_on_request, self.has_on_response, self.has_on_ws_message
        );

        Ok(())
    }

    /// 스크립트 코드 직접 로드 (JS)
    pub fn load_code(&mut self, code: &str) -> Result<(), String> {
        self.runtime
            .execute_script("<script>".to_string(), code.to_string())
            .map_err(|e| format!("스크립트 실행 실패: {}", e))?;

        self.check_hooks()?;

        info!(
            "[Script] 코드 로드 완료 (onRequest={}, onResponse={}, onWsMessage={})",
            self.has_on_request, self.has_on_response, self.has_on_ws_message
        );

        Ok(())
    }

    /// TypeScript 코드 직접 로드 (트랜스파일 후 실행)
    pub fn load_ts_code(&mut self, ts_code: &str) -> Result<(), String> {
        let js_code = transpile_ts(ts_code, "script.ts")?;
        self.load_code(&js_code)
    }

    fn check_hooks(&mut self) -> Result<(), String> {
        self.has_on_request = self.eval_bool("globalThis.__cheolsu_internal.hasOnRequest()")?;
        self.has_on_response = self.eval_bool("globalThis.__cheolsu_internal.hasOnResponse()")?;
        self.has_on_ws_message =
            self.eval_bool("globalThis.__cheolsu_internal.hasOnWebSocketMessage()")?;
        Ok(())
    }

    fn eval_bool(&mut self, code: &str) -> Result<bool, String> {
        let global = self
            .runtime
            .execute_script("<eval>".to_string(), code.to_string())
            .map_err(|e| format!("eval 실패: {}", e))?;

        let context = self.runtime.main_context();
        let isolate = self.runtime.v8_isolate();
        let mut scope_raw = v8::HandleScope::new(isolate);
        let mut scope = unsafe { std::pin::Pin::new_unchecked(&mut scope_raw) }.init();
        let context_local = v8::Local::new(&scope, context);
        let scope = &mut v8::ContextScope::new(&mut scope, context_local);

        let local = v8::Local::new(scope, global);
        Ok(local.boolean_value(scope))
    }

    /// onRequest 훅 호출
    pub fn invoke_on_request(&mut self, request: &ScriptRequest) -> Result<RequestAction, String> {
        if !self.has_on_request {
            return Ok(RequestAction::Forward);
        }

        let request_json =
            serde_json::to_string(request).map_err(|e| format!("직렬화 실패: {}", e))?;

        let escaped = escape_template_literal(&request_json);
        let code = format!(
            "globalThis.__cheolsu_internal.invokeOnRequest(`{}`)",
            escaped
        );

        let result = self.eval_string(&code)?;

        serde_json::from_str(&result)
            .map_err(|e| format!("onRequest 반환값 파싱 실패: {} (raw: {})", e, result))
    }

    /// onResponse 훅 호출
    pub fn invoke_on_response(
        &mut self,
        request: &ScriptRequest,
        response: &ScriptResponse,
    ) -> Result<ResponseAction, String> {
        if !self.has_on_response {
            return Ok(ResponseAction::Forward);
        }

        let request_json =
            serde_json::to_string(request).map_err(|e| format!("직렬화 실패: {}", e))?;
        let response_json =
            serde_json::to_string(response).map_err(|e| format!("직렬화 실패: {}", e))?;

        let req_escaped = escape_template_literal(&request_json);
        let res_escaped = escape_template_literal(&response_json);
        let code = format!(
            "globalThis.__cheolsu_internal.invokeOnResponse(`{}`, `{}`)",
            req_escaped, res_escaped
        );

        let result = self.eval_string(&code)?;

        serde_json::from_str(&result)
            .map_err(|e| format!("onResponse 반환값 파싱 실패: {} (raw: {})", e, result))
    }

    /// onWebSocketMessage 훅 호출
    pub fn invoke_on_ws_message(&mut self, message: &ScriptWsMessage) -> Result<WsAction, String> {
        if !self.has_on_ws_message {
            return Ok(WsAction::Forward);
        }

        let message_json =
            serde_json::to_string(message).map_err(|e| format!("직렬화 실패: {}", e))?;

        let escaped = escape_template_literal(&message_json);
        let code = format!(
            "globalThis.__cheolsu_internal.invokeOnWebSocketMessage(`{}`)",
            escaped
        );

        let result = self.eval_string(&code)?;

        serde_json::from_str(&result).map_err(|e| {
            format!(
                "onWebSocketMessage 반환값 파싱 실패: {} (raw: {})",
                e, result
            )
        })
    }

    /// JS 코드를 실행하고 문자열 결과 반환 (동기)
    fn eval_string(&mut self, code: &str) -> Result<String, String> {
        let global = self
            .runtime
            .execute_script("<invoke>".to_string(), code.to_string())
            .map_err(|e| format!("JS 실행 실패: {}", e))?;

        let context = self.runtime.main_context();
        let isolate = self.runtime.v8_isolate();
        let mut scope_raw = v8::HandleScope::new(isolate);
        let mut scope = unsafe { std::pin::Pin::new_unchecked(&mut scope_raw) }.init();
        let context_local = v8::Local::new(&scope, context);
        let scope = &mut v8::ContextScope::new(&mut scope, context_local);

        let local = v8::Local::new(scope, global);
        let result = local
            .to_string(scope)
            .ok_or_else(|| "JS 결과를 문자열로 변환 실패".to_string())?;

        Ok(result.to_rust_string_lossy(scope))
    }

    pub fn has_on_request(&self) -> bool {
        self.has_on_request
    }

    pub fn has_on_response(&self) -> bool {
        self.has_on_response
    }

    pub fn has_on_ws_message(&self) -> bool {
        self.has_on_ws_message
    }
}

/// JS 템플릿 리터럴에서 안전하게 사용할 수 있도록 이스케이프
fn escape_template_literal(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace("${", "\\${")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_no_hooks_returns_forward() {
        let mut engine = ScriptEngine::new().unwrap();
        let req = ScriptRequest {
            method: "GET".to_string(),
            url: "https://example.com".to_string(),
            headers: HashMap::new(),
            body: None,
        };
        let result = engine.invoke_on_request(&req).unwrap();
        assert!(matches!(result, RequestAction::Forward));
    }

    #[test]
    fn test_on_request_forward() {
        let mut engine = ScriptEngine::new().unwrap();
        engine
            .load_code(r#"cheolsu.onRequest((req) => ({ action: "forward" }))"#)
            .unwrap();
        assert!(engine.has_on_request());

        let req = ScriptRequest {
            method: "GET".to_string(),
            url: "https://example.com".to_string(),
            headers: HashMap::new(),
            body: None,
        };
        let result = engine.invoke_on_request(&req).unwrap();
        assert!(matches!(result, RequestAction::Forward));
    }

    #[test]
    fn test_on_request_respond() {
        let mut engine = ScriptEngine::new().unwrap();
        engine
            .load_code(
                r#"
                cheolsu.onRequest((req) => {
                    if (req.url.includes("blocked.com")) {
                        return {
                            action: "respond",
                            response: { status: 403, headers: {}, body: "Blocked" }
                        };
                    }
                    return { action: "forward" };
                });
            "#,
            )
            .unwrap();

        let req = ScriptRequest {
            method: "GET".to_string(),
            url: "https://blocked.com/page".to_string(),
            headers: HashMap::new(),
            body: None,
        };
        let result = engine.invoke_on_request(&req).unwrap();
        match result {
            RequestAction::Respond { response } => {
                assert_eq!(response.status, 403);
                assert_eq!(response.body.unwrap(), "Blocked");
            }
            _ => panic!("Expected Respond"),
        }
    }

    #[test]
    fn test_on_request_modify() {
        let mut engine = ScriptEngine::new().unwrap();
        engine
            .load_code(
                r#"
                cheolsu.onRequest((req) => {
                    req.headers["X-Custom"] = "injected";
                    return { action: "modify", request: req };
                });
            "#,
            )
            .unwrap();

        let req = ScriptRequest {
            method: "GET".to_string(),
            url: "https://api.example.com/data".to_string(),
            headers: HashMap::new(),
            body: None,
        };
        let result = engine.invoke_on_request(&req).unwrap();
        match result {
            RequestAction::ModifyRequest { request } => {
                assert_eq!(request.headers.get("X-Custom").unwrap(), "injected");
            }
            _ => panic!("Expected ModifyRequest"),
        }
    }

    #[test]
    fn test_on_response_modify() {
        let mut engine = ScriptEngine::new().unwrap();
        engine
            .load_code(
                r#"
                cheolsu.onResponse((req, res) => {
                    res.headers["X-Proxy"] = "cheolsu";
                    return { action: "modify", response: res };
                });
            "#,
            )
            .unwrap();
        assert!(engine.has_on_response());

        let req = ScriptRequest {
            method: "GET".to_string(),
            url: "https://example.com".to_string(),
            headers: HashMap::new(),
            body: None,
        };
        let res = ScriptResponse {
            status: 200,
            headers: HashMap::new(),
            body: Some("OK".to_string()),
        };
        let result = engine.invoke_on_response(&req, &res).unwrap();
        match result {
            ResponseAction::ModifyResponse { response } => {
                assert_eq!(response.headers.get("X-Proxy").unwrap(), "cheolsu");
            }
            _ => panic!("Expected ModifyResponse"),
        }
    }

    #[test]
    fn test_typescript_script() {
        let mut engine = ScriptEngine::new().unwrap();
        engine
            .load_ts_code(
                r#"
                interface RequestContext {
                    method: string;
                    url: string;
                    headers: Record<string, string>;
                    body?: string;
                }
                cheolsu.onRequest((req: RequestContext) => {
                    return { action: "forward" };
                });
            "#,
            )
            .unwrap();
        assert!(engine.has_on_request());
    }

    #[test]
    fn test_error_in_hook_returns_forward() {
        let mut engine = ScriptEngine::new().unwrap();
        engine
            .load_code(
                r#"
                cheolsu.onRequest((req) => {
                    throw new Error("intentional error");
                });
            "#,
            )
            .unwrap();

        let req = ScriptRequest {
            method: "GET".to_string(),
            url: "https://example.com".to_string(),
            headers: HashMap::new(),
            body: None,
        };
        let result = engine.invoke_on_request(&req).unwrap();
        assert!(matches!(result, RequestAction::Forward));
    }

    #[test]
    fn test_escape_template_literal() {
        let escaped = escape_template_literal("hello `world` ${var}");
        assert_eq!(escaped, "hello \\`world\\` \\${var}");

        let escaped = escape_template_literal(r#"back\slash"#);
        assert_eq!(escaped, r#"back\\slash"#);
    }
}
