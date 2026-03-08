use crate::error::ScriptError;
use crate::transpiler::transpile_ts;
use crate::types::{
    RequestAction, ResponseAction, ScriptLogEntry, ScriptRequest, ScriptResponse, ScriptWsMessage,
    WsAction,
};
use deno_core::{JsRuntime, PollEventLoopOptions, RuntimeOptions};
use std::time::Duration;
use tracing::info;

const RUNTIME_JS: &str = include_str!("runtime.js");

/// 타이머 sleep op (setTimeout/setInterval 구현용)
#[deno_core::op2]
async fn op_timer_sleep(#[smi] delay_ms: u32) {
    tokio::time::sleep(Duration::from_millis(delay_ms as u64)).await;
}

deno_core::extension!(cheolsu_timer_ext, ops = [op_timer_sleep],);

/// deno_core 기반 TypeScript/JavaScript 스크립팅 엔진
pub struct ScriptEngine {
    runtime: JsRuntime,
    has_on_request: bool,
    has_on_response: bool,
    has_on_ws_message: bool,
}

impl ScriptEngine {
    /// 새 스크립트 엔진 생성 (runtime.js 로드)
    pub fn new() -> Result<Self, ScriptError> {
        let mut runtime = JsRuntime::new(RuntimeOptions {
            extensions: vec![cheolsu_timer_ext::init()],
            ..Default::default()
        });

        runtime
            .execute_script("<runtime>".to_string(), RUNTIME_JS.to_string())
            .map_err(|e| ScriptError::RuntimeInit(e.to_string()))?;

        Ok(Self {
            runtime,
            has_on_request: false,
            has_on_response: false,
            has_on_ws_message: false,
        })
    }

    /// 기존 타이머 및 비동기 작업 정리
    pub fn clear_timers(&mut self) {
        let _ = self.runtime.execute_script(
            "<clear_timers>".to_string(),
            "globalThis.__cheolsu_internal.clearAllTimers()".to_string(),
        );
    }

    /// 사용자 스크립트 파일 로드 (JS/TS 모두 지원)
    pub fn load_script(&mut self, path: &str) -> Result<(), ScriptError> {
        let script_path = std::path::Path::new(path);

        // 경로 정규화로 path traversal 공격 방지
        let canonical = script_path
            .canonicalize()
            .map_err(|e| ScriptError::PathResolution {
                path: path.to_string(),
                reason: e.to_string(),
            })?;

        // 스크립트 파일 확장자 검증
        match canonical.extension().and_then(|e| e.to_str()) {
            Some("js" | "ts" | "tsx" | "mjs") => {}
            _ => {
                return Err(ScriptError::InvalidExtension {
                    path: path.to_string(),
                });
            }
        }

        let source = std::fs::read_to_string(&canonical)?;

        let code = if path.ends_with(".ts") || path.ends_with(".tsx") {
            transpile_ts(&source, path)?
        } else {
            source
        };

        self.runtime
            .execute_script(path.to_string(), code)
            .map_err(|e| ScriptError::Execution(e.to_string()))?;

        self.check_hooks()?;

        info!(
            "[Script] 로드 완료: {} (onRequest={}, onResponse={}, onWsMessage={})",
            path, self.has_on_request, self.has_on_response, self.has_on_ws_message
        );

        Ok(())
    }

    /// 스크립트 코드 직접 로드 (JS)
    pub fn load_code(&mut self, code: &str) -> Result<(), ScriptError> {
        self.runtime
            .execute_script("<script>".to_string(), code.to_string())
            .map_err(|e| ScriptError::Execution(e.to_string()))?;

        self.check_hooks()?;

        info!(
            "[Script] 코드 로드 완료 (onRequest={}, onResponse={}, onWsMessage={})",
            self.has_on_request, self.has_on_response, self.has_on_ws_message
        );

        Ok(())
    }

    /// TypeScript 코드 직접 로드 (트랜스파일 후 실행)
    pub fn load_ts_code(&mut self, ts_code: &str) -> Result<(), ScriptError> {
        let js_code = transpile_ts(ts_code, "script.ts")?;
        self.load_code(&js_code)
    }

    fn check_hooks(&mut self) -> Result<(), ScriptError> {
        self.has_on_request = self.eval_bool("globalThis.__cheolsu_internal.hasOnRequest()")?;
        self.has_on_response = self.eval_bool("globalThis.__cheolsu_internal.hasOnResponse()")?;
        self.has_on_ws_message =
            self.eval_bool("globalThis.__cheolsu_internal.hasOnWebSocketMessage()")?;
        Ok(())
    }

    fn eval_bool(&mut self, code: &str) -> Result<bool, ScriptError> {
        let global = self
            .runtime
            .execute_script("<eval>".to_string(), code.to_string())
            .map_err(|e| ScriptError::Execution(e.to_string()))?;

        let context = self.runtime.main_context();
        let isolate = self.runtime.v8_isolate();
        let mut scope_raw = v8::HandleScope::new(isolate);
        let mut scope = unsafe { std::pin::Pin::new_unchecked(&mut scope_raw) }.init();
        let context_local = v8::Local::new(&scope, context);
        let scope = &mut v8::ContextScope::new(&mut scope, context_local);

        let local = v8::Local::new(scope, global);
        Ok(local.boolean_value(scope))
    }

    /// onRequest 훅 호출 (async 훅 지원)
    pub async fn invoke_on_request(
        &mut self,
        request: &ScriptRequest,
    ) -> Result<RequestAction, ScriptError> {
        if !self.has_on_request {
            return Ok(RequestAction::Forward);
        }

        let request_json = serde_json::to_string(request)?;

        // JSON을 이중 직렬화하여 안전한 JS 문자열 리터럴로 전달 (template literal injection 방지)
        let safe_js_str = serde_json::to_string(&request_json)?;
        let code = format!(
            "globalThis.__cheolsu_internal.invokeOnRequest({})",
            safe_js_str
        );

        let result = self.eval_string_resolving(&code).await?;

        serde_json::from_str(&result).map_err(|e| ScriptError::HookParse {
            message: e.to_string(),
            raw: result,
        })
    }

    /// onResponse 훅 호출 (async 훅 지원)
    pub async fn invoke_on_response(
        &mut self,
        request: &ScriptRequest,
        response: &ScriptResponse,
    ) -> Result<ResponseAction, ScriptError> {
        if !self.has_on_response {
            return Ok(ResponseAction::Forward);
        }

        let request_json = serde_json::to_string(request)?;
        let response_json = serde_json::to_string(response)?;

        // JSON을 이중 직렬화하여 안전한 JS 문자열 리터럴로 전달
        let safe_req = serde_json::to_string(&request_json)?;
        let safe_res = serde_json::to_string(&response_json)?;
        let code = format!(
            "globalThis.__cheolsu_internal.invokeOnResponse({}, {})",
            safe_req, safe_res
        );

        let result = self.eval_string_resolving(&code).await?;

        serde_json::from_str(&result).map_err(|e| ScriptError::HookParse {
            message: e.to_string(),
            raw: result,
        })
    }

    /// onWebSocketMessage 훅 호출 (async 훅 지원)
    pub async fn invoke_on_ws_message(
        &mut self,
        message: &ScriptWsMessage,
    ) -> Result<WsAction, ScriptError> {
        if !self.has_on_ws_message {
            return Ok(WsAction::Forward);
        }

        let message_json = serde_json::to_string(message)?;

        // JSON을 이중 직렬화하여 안전한 JS 문자열 리터럴로 전달
        let safe_js_str = serde_json::to_string(&message_json)?;
        let code = format!(
            "globalThis.__cheolsu_internal.invokeOnWebSocketMessage({})",
            safe_js_str
        );

        let result = self.eval_string_resolving(&code).await?;

        serde_json::from_str(&result).map_err(|e| ScriptError::HookParse {
            message: e.to_string(),
            raw: result,
        })
    }

    /// JS 코드를 실행하고 Promise면 resolve 후 문자열 결과 반환
    async fn eval_string_resolving(&mut self, code: &str) -> Result<String, ScriptError> {
        let global = self
            .runtime
            .execute_script("<invoke>".to_string(), code.to_string())
            .map_err(|e| ScriptError::Execution(e.to_string()))?;

        // Promise면 이벤트 루프를 돌려서 resolve
        let resolve = self.runtime.resolve(global);
        let resolved = self
            .runtime
            .with_event_loop_promise(resolve, PollEventLoopOptions::default())
            .await
            .map_err(|e| ScriptError::Execution(e.to_string()))?;

        // v8 값을 문자열로 변환
        let context = self.runtime.main_context();
        let isolate = self.runtime.v8_isolate();
        let mut scope_raw = v8::HandleScope::new(isolate);
        let mut scope = unsafe { std::pin::Pin::new_unchecked(&mut scope_raw) }.init();
        let context_local = v8::Local::new(&scope, context);
        let scope = &mut v8::ContextScope::new(&mut scope, context_local);

        let local = v8::Local::new(scope, resolved);
        let result = local.to_string(scope).ok_or(ScriptError::ValueConversion)?;

        Ok(result.to_rust_string_lossy(scope))
    }

    /// JS 코드를 실행하고 문자열 결과 반환 (동기 - 내부 유틸용)
    fn eval_string(&mut self, code: &str) -> Result<String, ScriptError> {
        let global = self
            .runtime
            .execute_script("<invoke>".to_string(), code.to_string())
            .map_err(|e| ScriptError::Execution(e.to_string()))?;

        let context = self.runtime.main_context();
        let isolate = self.runtime.v8_isolate();
        let mut scope_raw = v8::HandleScope::new(isolate);
        let mut scope = unsafe { std::pin::Pin::new_unchecked(&mut scope_raw) }.init();
        let context_local = v8::Local::new(&scope, context);
        let scope = &mut v8::ContextScope::new(&mut scope, context_local);

        let local = v8::Local::new(scope, global);
        let result = local.to_string(scope).ok_or(ScriptError::ValueConversion)?;

        Ok(result.to_rust_string_lossy(scope))
    }

    /// 로그 버퍼를 드레인하여 반환
    pub fn drain_logs(&mut self) -> Vec<ScriptLogEntry> {
        match self.eval_string("globalThis.__cheolsu_internal.drainLogs()") {
            Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn block_on<F: std::future::Future>(f: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(f)
    }

    #[test]
    fn test_no_hooks_returns_forward() {
        let mut engine = ScriptEngine::new().unwrap();
        let req = ScriptRequest {
            method: "GET".to_string(),
            url: "https://example.com".to_string(),
            headers: HashMap::new(),
            body: None,
        };
        let result = block_on(engine.invoke_on_request(&req)).unwrap();
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
        let result = block_on(engine.invoke_on_request(&req)).unwrap();
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
        let result = block_on(engine.invoke_on_request(&req)).unwrap();
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
        let result = block_on(engine.invoke_on_request(&req)).unwrap();
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
        let result = block_on(engine.invoke_on_response(&req, &res)).unwrap();
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
        let result = block_on(engine.invoke_on_request(&req)).unwrap();
        assert!(matches!(result, RequestAction::Forward));
    }

    #[test]
    fn test_special_chars_in_json() {
        let mut engine = ScriptEngine::new().unwrap();
        engine
            .load_code(r#"cheolsu.onRequest((req) => ({ action: "forward" }))"#)
            .unwrap();

        let mut headers = HashMap::new();
        headers.insert(
            "X-Test".to_string(),
            "value with `backticks` and ${template}".to_string(),
        );
        let req = ScriptRequest {
            method: "GET".to_string(),
            url: r#"https://example.com/path?q=back\slash"#.to_string(),
            headers,
            body: None,
        };
        let result = block_on(engine.invoke_on_request(&req)).unwrap();
        assert!(matches!(result, RequestAction::Forward));
    }

    #[test]
    fn test_async_on_request() {
        let mut engine = ScriptEngine::new().unwrap();
        engine
            .load_code(
                r#"
                cheolsu.onRequest(async (req) => {
                    await new Promise(resolve => setTimeout(resolve, 10));
                    req.headers["X-Async"] = "true";
                    return { action: "modify", request: req };
                });
            "#,
            )
            .unwrap();
        assert!(engine.has_on_request());

        let req = ScriptRequest {
            method: "GET".to_string(),
            url: "https://example.com".to_string(),
            headers: HashMap::new(),
            body: None,
        };
        let result = block_on(engine.invoke_on_request(&req)).unwrap();
        match result {
            RequestAction::ModifyRequest { request } => {
                assert_eq!(request.headers.get("X-Async").unwrap(), "true");
            }
            _ => panic!("Expected ModifyRequest from async hook"),
        }
    }

    #[test]
    fn test_async_on_response() {
        let mut engine = ScriptEngine::new().unwrap();
        engine
            .load_code(
                r#"
                cheolsu.onResponse(async (req, res) => {
                    await new Promise(resolve => setTimeout(resolve, 5));
                    res.headers["X-Async"] = "response";
                    return { action: "modify", response: res };
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
        let res = ScriptResponse {
            status: 200,
            headers: HashMap::new(),
            body: None,
        };
        let result = block_on(engine.invoke_on_response(&req, &res)).unwrap();
        match result {
            ResponseAction::ModifyResponse { response } => {
                assert_eq!(response.headers.get("X-Async").unwrap(), "response");
            }
            _ => panic!("Expected ModifyResponse from async hook"),
        }
    }

    #[test]
    fn test_async_error_returns_forward() {
        let mut engine = ScriptEngine::new().unwrap();
        engine
            .load_code(
                r#"
                cheolsu.onRequest(async (req) => {
                    throw new Error("async error");
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
        let result = block_on(engine.invoke_on_request(&req)).unwrap();
        assert!(matches!(result, RequestAction::Forward));
    }

    #[test]
    fn test_clear_timers_prevents_pending_callbacks() {
        block_on(async {
            let mut engine = ScriptEngine::new().unwrap();
            engine
                .load_code(
                    r#"
                    let counter = 0;
                    setInterval(() => { counter++; }, 50);
                    setTimeout(() => { counter += 100; }, 50);
                    cheolsu.onRequest((req) => {
                        return { action: "respond", response: { status: 200, headers: {}, body: String(counter) } };
                    });
                "#,
                )
                .unwrap();

            // 타이머 정리
            engine.clear_timers();

            // 타이머 정리 후에도 엔진이 정상 동작해야 함
            let req = ScriptRequest {
                method: "GET".to_string(),
                url: "https://example.com".to_string(),
                headers: HashMap::new(),
                body: None,
            };
            let result = engine.invoke_on_request(&req).await.unwrap();
            match result {
                RequestAction::Respond { response } => {
                    // counter가 0이어야 함 (타이머가 정리되어 콜백이 실행되지 않음)
                    assert_eq!(response.body.unwrap(), "0");
                }
                _ => panic!("Expected Respond"),
            }
        });
    }

    #[test]
    fn test_reload_script_after_clear_timers() {
        block_on(async {
            let mut engine = ScriptEngine::new().unwrap();

            // 첫 번째 스크립트 로드: 타이머 사용
            engine
                .load_code(
                    r#"
                    setInterval(() => { console.log("tick"); }, 100);
                    cheolsu.onRequest((req) => {
                        return { action: "respond", response: { status: 200, headers: {}, body: "v1" } };
                    });
                "#,
                )
                .unwrap();
            assert!(engine.has_on_request());

            // 타이머 정리 후 안전하게 drop (실제 리로드 시나리오)
            engine.clear_timers();
            drop(engine);

            // 새 엔진 생성 후 다른 스크립트 로드
            let mut engine2 = ScriptEngine::new().unwrap();
            engine2
                .load_code(
                    r#"
                    cheolsu.onRequest((req) => {
                        return { action: "respond", response: { status: 200, headers: {}, body: "v2" } };
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
            let result = engine2.invoke_on_request(&req).await.unwrap();
            match result {
                RequestAction::Respond { response } => {
                    assert_eq!(response.body.unwrap(), "v2");
                }
                _ => panic!("Expected Respond with v2"),
            }
        });
    }

    #[test]
    fn test_drain_logs() {
        let mut engine = ScriptEngine::new().unwrap();
        engine
            .load_code(
                r#"
                console.log("hello");
                console.error("oops");
                console.warn("careful");
            "#,
            )
            .unwrap();

        let logs = engine.drain_logs();
        assert_eq!(logs.len(), 3);
        assert_eq!(logs[0].level, "info");
        assert_eq!(logs[0].message, "hello");
        assert_eq!(logs[1].level, "error");
        assert_eq!(logs[1].message, "oops");
        assert_eq!(logs[2].level, "warn");
        assert_eq!(logs[2].message, "careful");

        // drain 후 비어있어야 함
        let logs2 = engine.drain_logs();
        assert!(logs2.is_empty());
    }

    #[test]
    fn test_on_ws_message_forward() {
        let mut engine = ScriptEngine::new().unwrap();
        engine
            .load_code(r#"cheolsu.onWebSocketMessage((msg) => ({ action: "forward" }))"#)
            .unwrap();
        assert!(engine.has_on_ws_message());
    }

    #[test]
    fn test_set_timeout_basic() {
        let mut engine = ScriptEngine::new().unwrap();
        engine
            .load_code(
                r#"
                let called = false;
                cheolsu.onRequest(async (req) => {
                    await new Promise(resolve => setTimeout(() => {
                        called = true;
                        resolve();
                    }, 10));
                    if (called) {
                        return { action: "respond", response: { status: 200, headers: {}, body: "timer works" } };
                    }
                    return { action: "forward" };
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
        let result = block_on(engine.invoke_on_request(&req)).unwrap();
        match result {
            RequestAction::Respond { response } => {
                assert_eq!(response.body.unwrap(), "timer works");
            }
            _ => panic!("Expected Respond with timer result"),
        }
    }
}
