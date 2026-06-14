use crate::error::ScriptError;
use crate::transpiler::transpile_ts;
use tracing::info;

use super::ScriptEngine;

impl ScriptEngine {
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

        // 디렉터리 화이트리스트는 두지 않는다. 스크립트는 운영자가 직접 지정한 자기 자신의
        // 파일이며, deno_core 샌드박스(fs/net op 미노출)가 실제 격리 계층이다. 홈 디렉터리
        // 제한은 보안 이득이 미미한 반면 /tmp·외부 볼륨·홈 밖 프로젝트의 정상 스크립트 로드를
        // 막는 회귀를 유발하므로, path traversal 방지(canonicalize)와 확장자 검증만 유지한다.
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
            "[Script] 로드 완료: {} (onRequest={}, onResponse={}, onWsMessage={}, onSSEMessage={})",
            path,
            self.has_on_request,
            self.has_on_response,
            self.has_on_ws_message,
            self.has_on_sse_message
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
            "[Script] 코드 로드 완료 (onRequest={}, onResponse={}, onWsMessage={}, onSSEMessage={})",
            self.has_on_request, self.has_on_response, self.has_on_ws_message, self.has_on_sse_message
        );

        Ok(())
    }

    /// TypeScript 코드 직접 로드 (트랜스파일 후 실행)
    pub fn load_ts_code(&mut self, ts_code: &str) -> Result<(), ScriptError> {
        let js_code = transpile_ts(ts_code, "script.ts")?;
        self.load_code(&js_code)
    }

    pub(crate) fn check_hooks(&mut self) -> Result<(), ScriptError> {
        self.has_on_request = self.eval_bool("globalThis.__cheolsu_internal.hasOnRequest()")?;
        self.has_on_response = self.eval_bool("globalThis.__cheolsu_internal.hasOnResponse()")?;
        self.has_on_ws_message =
            self.eval_bool("globalThis.__cheolsu_internal.hasOnWebSocketMessage()")?;
        self.has_on_sse_message =
            self.eval_bool("globalThis.__cheolsu_internal.hasOnSSEMessage()")?;
        Ok(())
    }
}
