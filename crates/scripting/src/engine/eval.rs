use crate::error::ScriptError;
use crate::types::ScriptLogEntry;
use deno_core::PollEventLoopOptions;

use super::ScriptEngine;

impl ScriptEngine {
    /// JS 코드를 실행하고 Promise면 resolve 후 문자열 결과 반환
    pub(crate) async fn eval_string_resolving(
        &mut self,
        code: &str,
    ) -> Result<String, ScriptError> {
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

    pub(crate) fn eval_bool(&mut self, code: &str) -> Result<bool, ScriptError> {
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

    /// 로그 버퍼를 드레인하여 반환
    pub fn drain_logs(&mut self) -> Vec<ScriptLogEntry> {
        match self.eval_string("globalThis.__cheolsu_internal.drainLogs()") {
            Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }
}
