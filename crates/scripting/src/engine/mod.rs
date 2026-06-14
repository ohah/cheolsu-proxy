mod eval;
mod hooks;
mod loader;

#[cfg(test)]
mod tests;

use crate::error::ScriptError;
use deno_core::{JsRuntime, RuntimeOptions};
use std::time::Duration;

/// V8 초기 힙 크기 (4MB)
const V8_INITIAL_HEAP_SIZE: usize = 4 * 1024 * 1024;
/// V8 최대 힙 크기 (64MB)
const V8_MAX_HEAP_SIZE: usize = 64 * 1024 * 1024;

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
    /// 다른 스레드에서 실행 중인 스크립트를 강제 종료(terminate)하기 위한 핸들
    isolate_handle: v8::IsolateHandle,
    has_on_request: bool,
    has_on_response: bool,
    has_on_ws_message: bool,
    has_on_sse_message: bool,
}

impl ScriptEngine {
    /// 새 스크립트 엔진 생성 (runtime.js 로드)
    pub fn new() -> Result<Self, ScriptError> {
        let create_params =
            v8::CreateParams::default().heap_limits(V8_INITIAL_HEAP_SIZE, V8_MAX_HEAP_SIZE);

        let mut runtime = JsRuntime::new(RuntimeOptions {
            extensions: vec![cheolsu_timer_ext::init()],
            create_params: Some(create_params),
            ..Default::default()
        });

        let isolate_handle = runtime.v8_isolate().thread_safe_handle();

        // 힙 한계 근접 시 스크립트 실행을 강제 종료하여 프로세스 전체 abort를 방지한다.
        // (heap_limits만 설정하면 한계 초과 시 V8이 프로세스를 abort시켜 데몬 전체가 죽는다)
        {
            let oom_handle = isolate_handle.clone();
            runtime.add_near_heap_limit_callback(move |current_limit, _initial_limit| {
                oom_handle.terminate_execution();
                // 종료가 적용되어 스택이 풀릴 때까지 즉시 OOM abort를 피하도록 한계를 일시 상향
                current_limit + 16 * 1024 * 1024
            });
        }

        runtime
            .execute_script("<runtime>".to_string(), RUNTIME_JS.to_string())
            .map_err(|e| ScriptError::RuntimeInit(e.to_string()))?;

        Ok(Self {
            runtime,
            isolate_handle,
            has_on_request: false,
            has_on_response: false,
            has_on_ws_message: false,
            has_on_sse_message: false,
        })
    }

    /// 다른 스레드에서 이 엔진의 실행을 강제 종료할 수 있는 핸들을 반환한다.
    pub fn isolate_handle(&self) -> v8::IsolateHandle {
        self.isolate_handle.clone()
    }

    /// 외부 terminate_execution으로 인해 현재 실행이 종료 상태인지 확인한다.
    pub fn is_terminating(&mut self) -> bool {
        self.runtime.v8_isolate().is_execution_terminating()
    }

    /// 종료 플래그를 해제한다(엔진을 안전하게 정리/재사용하기 위함).
    pub fn cancel_termination(&mut self) {
        self.runtime.v8_isolate().cancel_terminate_execution();
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
