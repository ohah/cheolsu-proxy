use crate::error::ScriptError;
use crate::types::{
    RequestAction, ResponseAction, ScriptRequest, ScriptResponse, ScriptSseEvent, ScriptWsMessage,
    SseAction, WsAction,
};

use super::ScriptEngine;

impl ScriptEngine {
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

    /// onSSEMessage 훅 호출 (async 훅 지원)
    pub async fn invoke_on_sse_event(
        &mut self,
        event: &ScriptSseEvent,
    ) -> Result<SseAction, ScriptError> {
        if !self.has_on_sse_message {
            return Ok(SseAction::Forward);
        }

        let event_json = serde_json::to_string(event)?;

        let safe_js_str = serde_json::to_string(&event_json)?;
        let code = format!(
            "globalThis.__cheolsu_internal.invokeOnSSEMessage({})",
            safe_js_str
        );

        let result = self.eval_string_resolving(&code).await?;

        serde_json::from_str(&result).map_err(|e| ScriptError::HookParse {
            message: e.to_string(),
            raw: result,
        })
    }
}
