use crate::error::ScriptError;
use crate::types::ScriptLogEntry;
use rquickjs::context::EvalOptions;
use rquickjs::convert::Coerced;
use rquickjs::promise::PromiseState;
use rquickjs::{Ctx, Persistent, Promise, Value};
use std::time::Duration;

use super::ScriptEngine;

/// 타이머 대기 중 강제 종료 요청을 확인하는 최대 간격
const TIMER_POLL_SLICE: Duration = Duration::from_millis(50);

/// V8 classic script와 같이 sloppy 모드 전역 스크립트로 평가한다.
/// (QuickJS의 기본값은 strict 모드라 기존 사용자 스크립트와 동작이 달라질 수 있다)
fn script_options(name: &str) -> EvalOptions {
    let mut opts = EvalOptions::default();
    opts.global = true;
    opts.strict = false;
    opts.filename = Some(name.to_string());
    opts
}

/// JS 값을 문자열로 변환한다 (`String(value)`와 동일).
fn value_to_string(value: Value<'_>) -> Result<String, ScriptError> {
    value
        .get::<Coerced<String>>()
        .map(|c| c.0)
        .map_err(|_| ScriptError::ValueConversion)
}

/// rquickjs 오류를 사람이 읽을 수 있는 메시지로 변환한다.
/// JS 예외라면 `이름: 메시지` 형태로, 스택이 있으면 함께 포함한다.
pub(super) fn js_error_message(ctx: &Ctx<'_>, err: rquickjs::Error) -> String {
    if !err.is_exception() {
        return err.to_string();
    }
    let thrown = ctx.catch();
    if let Some(exc) = thrown.as_exception() {
        let name: Option<String> = exc.get("name").ok();
        let message = exc.message().unwrap_or_default();
        let head = match name {
            Some(n) if !n.is_empty() => format!("{}: {}", n, message),
            _ => message,
        };
        return match exc.stack() {
            Some(stack) if !stack.trim().is_empty() => format!("{}\n{}", head, stack.trim_end()),
            _ => head,
        };
    }
    value_to_string(thrown).unwrap_or_else(|_| "알 수 없는 예외".to_string())
}

/// 평가 결과: 즉시 값이거나, 이벤트 루프를 돌려 기다려야 하는 Promise
enum Evaluated {
    Ready(String),
    Pending(Persistent<Promise<'static>>),
}

impl ScriptEngine {
    /// 스크립트를 전역 스코프에서 실행하고, 생성된 마이크로태스크를 모두 처리한다.
    pub(crate) fn execute_script(&mut self, name: &str, code: &str) -> Result<(), ScriptError> {
        self.context.with(|ctx| {
            ctx.eval_with_options::<(), _>(code, script_options(name))
                .map_err(|e| ScriptError::Execution(js_error_message(&ctx, e)))
        })?;
        self.run_pending_jobs()
    }

    /// 대기 중인 Promise 작업(마이크로태스크)을 큐가 빌 때까지 실행한다.
    fn run_pending_jobs(&mut self) -> Result<(), ScriptError> {
        loop {
            match self.runtime.execute_pending_job() {
                Ok(true) => continue,
                Ok(false) => return Ok(()),
                Err(job_err) => {
                    // 작업 중 예외는 강제 종료(인터럽트)나 메모리 초과 같은 치명적 상황이다.
                    let message = job_err.0.with(|ctx| {
                        let thrown = ctx.catch();
                        value_to_string(thrown).unwrap_or_else(|_| "알 수 없는 예외".to_string())
                    });
                    return Err(ScriptError::Execution(message));
                }
            }
        }
    }

    /// 강제 종료가 요청되었으면 오류를 반환한다.
    fn check_interrupted(&self) -> Result<(), ScriptError> {
        if self.interrupt.is_set() {
            return Err(ScriptError::Execution(
                "스크립트 실행이 강제 종료되었습니다".to_string(),
            ));
        }
        Ok(())
    }

    /// JS 코드를 실행하고 Promise면 resolve 후 문자열 결과 반환
    ///
    /// Promise가 해결될 때까지 마이크로태스크와 타이머(setTimeout/setInterval)를
    /// 처리하는 최소한의 이벤트 루프를 돌린다.
    pub(crate) async fn eval_string_resolving(
        &mut self,
        code: &str,
    ) -> Result<String, ScriptError> {
        let evaluated = self.context.with(|ctx| -> Result<Evaluated, ScriptError> {
            let value: Value = ctx
                .eval_with_options(code, script_options("<invoke>"))
                .map_err(|e| ScriptError::Execution(js_error_message(&ctx, e)))?;
            match value.as_promise() {
                Some(promise) => Ok(Evaluated::Pending(Persistent::save(&ctx, promise.clone()))),
                None => value_to_string(value).map(Evaluated::Ready),
            }
        })?;

        let promise = match evaluated {
            Evaluated::Ready(s) => {
                self.run_pending_jobs()?;
                return Ok(s);
            }
            Evaluated::Pending(p) => p,
        };

        loop {
            self.check_interrupted()?;
            self.run_pending_jobs()?;

            if let Some(result) = self.settled_result(&promise)? {
                return result;
            }

            match self.next_timer_delay()? {
                Some(delay_ms) => {
                    if delay_ms > 0.0 {
                        let wait = Duration::from_secs_f64(delay_ms / 1000.0).min(TIMER_POLL_SLICE);
                        tokio::time::sleep(wait).await;
                    }
                    self.run_due_timers()?;
                }
                None => {
                    if !self.runtime.is_job_pending() {
                        return Err(ScriptError::Execution(
                            "Promise가 해결되지 않았지만 대기 중인 작업이나 타이머가 없습니다"
                                .to_string(),
                        ));
                    }
                }
            }
        }
    }

    /// Promise가 완료되었으면 결과(문자열 또는 거부 사유)를 반환한다.
    fn settled_result(
        &mut self,
        promise: &Persistent<Promise<'static>>,
    ) -> Result<Option<Result<String, ScriptError>>, ScriptError> {
        self.context.with(|ctx| {
            let promise = promise
                .clone()
                .restore(&ctx)
                .map_err(|e| ScriptError::Execution(e.to_string()))?;
            Ok(match promise.state() {
                PromiseState::Pending => None,
                PromiseState::Resolved | PromiseState::Rejected => {
                    match promise.result::<Value>() {
                        Some(Ok(value)) => Some(value_to_string(value)),
                        Some(Err(e)) => {
                            Some(Err(ScriptError::Execution(js_error_message(&ctx, e))))
                        }
                        None => None,
                    }
                }
            })
        })
    }

    /// 가장 가까운 타이머까지 남은 시간(ms). 등록된 타이머가 없으면 `None`.
    fn next_timer_delay(&mut self) -> Result<Option<f64>, ScriptError> {
        let delay: f64 = self.context.with(|ctx| {
            ctx.eval_with_options::<f64, _>(
                "globalThis.__cheolsu_internal.nextTimerDelay()",
                script_options("<timers>"),
            )
            .map_err(|e| ScriptError::Execution(js_error_message(&ctx, e)))
        })?;
        Ok((delay >= 0.0).then_some(delay))
    }

    /// 만료된 타이머 콜백을 실행한다.
    fn run_due_timers(&mut self) -> Result<(), ScriptError> {
        self.execute_script("<timers>", "globalThis.__cheolsu_internal.runDueTimers()")
    }

    /// JS 코드를 실행하고 문자열 결과 반환 (동기 - 내부 유틸용)
    fn eval_string(&mut self, code: &str) -> Result<String, ScriptError> {
        self.context.with(|ctx| {
            let value: Value = ctx
                .eval_with_options(code, script_options("<eval>"))
                .map_err(|e| ScriptError::Execution(js_error_message(&ctx, e)))?;
            value_to_string(value)
        })
    }

    pub(crate) fn eval_bool(&mut self, code: &str) -> Result<bool, ScriptError> {
        self.context.with(|ctx| {
            ctx.eval_with_options::<Coerced<bool>, _>(code, script_options("<eval>"))
                .map(|c| c.0)
                .map_err(|e| ScriptError::Execution(js_error_message(&ctx, e)))
        })
    }

    /// 로그 버퍼를 드레인하여 반환
    pub fn drain_logs(&mut self) -> Vec<ScriptLogEntry> {
        match self.eval_string("globalThis.__cheolsu_internal.drainLogs()") {
            Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }
}
