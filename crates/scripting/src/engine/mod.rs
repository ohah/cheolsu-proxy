mod eval;
mod hooks;
mod loader;

#[cfg(test)]
mod tests;

use crate::error::ScriptError;
use deno_core::{JsRuntime, RuntimeOptions};
use std::time::Duration;

const RUNTIME_JS: &str = include_str!("../runtime.js");

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
    has_on_sse_message: bool,
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
            has_on_sse_message: false,
        })
    }

    /// 기존 타이머 및 비동기 작업 정리
    pub fn clear_timers(&mut self) {
        let _ = self.runtime.execute_script(
            "<clear_timers>".to_string(),
            "globalThis.__cheolsu_internal.clearAllTimers()".to_string(),
        );
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

    pub fn has_on_sse_message(&self) -> bool {
        self.has_on_sse_message
    }
}
