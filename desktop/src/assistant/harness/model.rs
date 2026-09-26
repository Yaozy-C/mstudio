//! Provider adapters own wire protocols; the harness owns attempts and visible frames.
use super::{Host, session::Session};
use futures::StreamExt;
use rig_core::{
    completion::{CompletionError, CompletionModel, CompletionRequest, CompletionResponse},
    streaming::StreamedAssistantContent,
};
use serde_json::json;

pub async fn request(
    model: &impl CompletionModel,
    request: CompletionRequest,
    host: &impl Host,
    session: &mut Session,
    streaming: bool,
    key: &str,
    deadline: tokio::time::Instant,
) -> Result<CompletionResponse, String> {
    request
        .validate_message_content()
        .map_err(|e| e.to_string())?;
    for attempt in 0..3 {
        if host.token().is_cancelled() {
            return Err("已停止回答；已输出内容和已完成操作保留".into());
        }
        host.record("request/start", json!({"attempt":attempt + 1}))?;
        let prefix = session.text.len();
        let mut persistence_error = None;
        let call = async {
            if !streaming {
                return model.completion(request.clone()).await;
            }
            let mut stream = model.stream(request.clone()).await?;
            let mut finished = false;
            let mut pending = String::new();
            let mut last_frame = std::time::Instant::now();
            while let Some(part) = stream.next().await {
                let part = part?;
                if matches!(&part, StreamedAssistantContent::Final(_)) {
                    finished = true;
                }
                if let StreamedAssistantContent::Text(text) = part {
                    pending.push_str(&text.text);
                    if session.text.len() == prefix
                        || pending.len() >= 128
                        || last_frame.elapsed().as_millis() >= 40
                    {
                        if let Err(error) = session.publish(host, &pending) {
                            persistence_error = Some(error);
                            return Err(CompletionError::ResponseError("无法保存输出".into()));
                        }
                        pending.clear();
                        last_frame = std::time::Instant::now();
                    }
                }
            }
            if !pending.is_empty()
                && let Err(error) = session.publish(host, &pending)
            {
                persistence_error = Some(error);
                return Err(CompletionError::ResponseError("无法保存输出".into()));
            }
            if !finished {
                return Err(CompletionError::ResponseError(
                    "模型流在完成事件前中断".into(),
                ));
            }
            Ok(stream.into())
        };
        let result = tokio::select! {
            result = call => result,
            _ = host.token().cancelled() => return Err("已停止回答；已输出内容和已完成操作保留".into()),
            _ = tokio::time::sleep_until(deadline) => return Err("本轮已达到 20 分钟时限；已完成操作保留".into()),
        };
        if let Some(error) = persistence_error {
            return Err(error);
        }
        match result {
            Ok(response) => return Ok(response),
            Err(error) => {
                if session.text.len() == prefix
                    && crate::assistant::failure::context_overflow(&error)
                {
                    host.record("request/context-overflow", json!({"attempt":attempt + 1}))?;
                    return Err("CONTEXT_WINDOW_EXCEEDED".into());
                }
                // Only retry an uncommitted model request, never an executed tool or visible prefix.
                if attempt < 2 && session.text.len() == prefix && retryable(&error) {
                    host.record("request/retry", json!({"attempt":attempt + 2}))?;
                    tokio::select! {
                        _ = tokio::time::sleep(std::time::Duration::from_millis(500 * (1 << attempt))) => {},
                        _ = host.token().cancelled() => return Err("已停止回答".into()),
                        _ = tokio::time::sleep_until(deadline) => return Err("本轮已达到 20 分钟时限；已完成操作保留".into()),
                    }
                    continue;
                }
                return Err(crate::assistant::failure::describe(&error.into(), key));
            }
        }
    }
    unreachable!()
}
fn retryable(error: &CompletionError) -> bool {
    use rig_core::http_client::Error;
    match error {
        CompletionError::HttpError(Error::InvalidStatusCode(s))
        | CompletionError::HttpError(Error::InvalidStatusCodeWithMessage(s, _))
        | CompletionError::HttpError(Error::InvalidStatusCodeWithDetails { status: s, .. }) => {
            s.as_u16() == 429 || s.is_server_error()
        }
        CompletionError::HttpError(Error::Instance(e)) => e
            .downcast_ref::<rig_http::Error>()
            .is_some_and(|e| e.is_connect() || e.is_timeout()),
        _ => false,
    }
}
