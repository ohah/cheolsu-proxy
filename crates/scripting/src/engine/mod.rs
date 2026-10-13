mod eval;
mod hooks;
mod loader;

#[cfg(test)]
mod tests;

use crate::error::ScriptError;
use rquickjs::{Context, Function, Runtime};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// QuickJS 런타임 최대 메모리 (64MB)
const JS_MEMORY_LIMIT: usize = 64 * 1024 * 1024;
/// QuickJS 최대 스택 크기 (1MB, 엔진 스레드 기본 스택 2MB 안쪽)
const JS_MAX_STACK_SIZE: usize = 1024 * 1024;

const RUNTIME_JS: &str = include_str!("../runtime.js");

/// 다른 스레드에서 실행 중인 스크립트를 강제 종료하기 위한 핸들.
///
/// QuickJS 인터럽트 핸들러가 실행 중 주기적으로 이 플래그를 확인하고,
/// `true`이면 잡을 수 없는(uncatchable) 예외를 던져 실행을 중단한다.
#[derive(Clone, Debug, Default)]
pub struct InterruptHandle(Arc<AtomicBool>);

impl InterruptHandle {
    /// 실행 중인 스크립트의 강제 종료를 요청한다.
    pub fn terminate_execution(&self) {
        self.0.store(true, Ordering::Release);
    }

    fn is_set(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }

    fn clear(&self) {
        self.0.store(false, Ordering::Release);
    }
}

/// QuickJS(rquickjs) 기반 TypeScript/JavaScript 스크립팅 엔진
pub struct ScriptEngine {
    // context가 runtime보다 먼저 drop되도록 선언 순서를 유지한다.
    context: Context,
    runtime: Runtime,
    /// 다른 스레드에서 실행 중인 스크립트를 강제 종료(terminate)하기 위한 핸들
    interrupt: InterruptHandle,
    has_on_request: bool,
    has_on_response: bool,
    has_on_ws_message: bool,
    has_on_sse_message: bool,
}

impl ScriptEngine {
    /// 새 스크립트 엔진 생성 (runtime.js 로드)
    pub fn new() -> Result<Self, ScriptError> {
        let runtime = Runtime::new().map_err(|e| ScriptError::RuntimeInit(e.to_string()))?;
        // 한계 초과 시 프로세스 abort 대신 JS 예외(out of memory)로 실패시킨다.
        runtime.set_memory_limit(JS_MEMORY_LIMIT);
        runtime.set_max_stack_size(JS_MAX_STACK_SIZE);

        let interrupt = InterruptHandle::default();
        {
            let flag = interrupt.clone();
            runtime.set_interrupt_handler(Some(Box::new(move || flag.is_set())));
        }

        let context =
            Context::full(&runtime).map_err(|e| ScriptError::RuntimeInit(e.to_string()))?;

        // runtime.js가 사용할 내부 네이티브 함수 등록 (runtime.js가 캡처 후 전역에서 제거)
        let started = Instant::now();
        context
            .with(|ctx| -> rquickjs::Result<()> {
                let globals = ctx.globals();
                globals.set(
                    "__cheolsu_now",
                    Function::new(ctx.clone(), move || {
                        started.elapsed().as_secs_f64() * 1000.0
                    })?,
                )?;
                globals.set(
                    "__cheolsu_print",
                    Function::new(ctx.clone(), |msg: String, is_err: bool| {
                        if is_err {
                            tracing::debug!(target: "script", "[stderr] {}", msg.trim_end());
                        } else {
                            tracing::debug!(target: "script", "{}", msg.trim_end());
                        }
                    })?,
                )?;
                Ok(())
            })
            .map_err(|e| ScriptError::RuntimeInit(e.to_string()))?;

        let mut engine = Self {
            context,
            runtime,
            interrupt,
            has_on_request: false,
            has_on_response: false,
            has_on_ws_message: false,
            has_on_sse_message: false,
        };

        engine
            .execute_script("<runtime>", RUNTIME_JS)
            .map_err(|e| ScriptError::RuntimeInit(e.to_string()))?;

        Ok(engine)
    }

    /// 다른 스레드에서 이 엔진의 실행을 강제 종료할 수 있는 핸들을 반환한다.
    pub fn interrupt_handle(&self) -> InterruptHandle {
        self.interrupt.clone()
    }

    /// 외부 강제 종료 요청으로 인해 현재 실행이 종료 상태인지 확인한다.
    pub fn is_terminating(&mut self) -> bool {
        self.interrupt.is_set()
    }

    /// 종료 플래그를 해제한다(엔진을 안전하게 정리/재사용하기 위함).
    pub fn cancel_termination(&mut self) {
        self.interrupt.clear();
    }

    /// 기존 타이머 및 비동기 작업 정리
    pub fn clear_timers(&mut self) {
        let _ = self.execute_script(
            "<clear_timers>",
            "globalThis.__cheolsu_internal.clearAllTimers()",
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
