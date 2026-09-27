//! DSH tool-calls.ts contract: bounded rolling pool, exclusive barriers, ordered commits.
use super::{
    Host,
    session::{Session, result_message},
};
use futures::{StreamExt, stream::FuturesUnordered};
use rig_core::{completion::ToolDefinition, message::ToolCall};
use serde_json::{Value, json};

pub async fn execute(
    host: &impl Host,
    session: &mut Session,
    calls: &[ToolCall],
) -> Result<(), String> {
    let definitions = host.definitions();
    let mut start = 0;
    while start < calls.len() {
        let count = if host.parallel_safe(&calls[start]) {
            calls[start..]
                .iter()
                .take_while(|c| host.parallel_safe(c))
                .count()
        } else {
            1
        };
        group(host, session, &definitions, &calls[start..start + count]).await?;
        start += count;
    }
    Ok(())
}
async fn group(
    host: &impl Host,
    session: &mut Session,
    definitions: &[ToolDefinition],
    calls: &[ToolCall],
) -> Result<(), String> {
    let mut in_flight = FuturesUnordered::new();
    let mut slots = vec![None; calls.len()];
    let (mut next, mut committed) = (0, 0);
    let mut failure = None;
    while committed < calls.len() {
        while failure.is_none() && next < calls.len() && in_flight.len() < 4 {
            let index = next;
            let call = &calls[index];
            if let Err(error) = host.record("tool/call", json!({"callId":call.id.as_str(),"name":call.function.name,"arguments":call.function.arguments})) {
                failure = Some(error); break;
            }
            let invalid = validate(definitions, call);
            in_flight.push(async move {
                let value = if host.token().is_cancelled() {
                    json!({"error":"调用尚未执行，任务已停止", "code":"ABORTED_BEFORE_DISPATCH"})
                } else if let Some(error) = invalid {
                    error
                } else {
                    host.execute(call).await
                };
                (index, value)
            });
            next += 1;
        }
        if let Some((index, value)) = in_flight.next().await {
            slots[index] = Some(value);
        } else if failure.is_some() {
            break;
        }
        while let Some(value) = slots.get_mut(committed).and_then(Option::take) {
            let call = &calls[committed];
            let canonical = value;
            let value = super::tool_output::project(call, &canonical, host.result_turn());
            let message = result_message(
                call,
                if canonical.get("__offloadedImage").is_some() {
                    &canonical
                } else {
                    &value
                },
            );
            // One durable result owns both the model history and the visible tool row.
            let result = host.record("tool/result", json!({"callId":call.id.as_str(),"name":call.function.name,"result":value,"value":canonical,"isError":value.get("error").is_some(),"message":message}));
            if let Err(error) = result {
                failure.get_or_insert(error);
            } else {
                session
                    .edit_progress
                    .observe(&call.function.name, &canonical);
                session.messages.push(message);
            }
            committed += 1;
        }
        if failure.is_some() {
            // Do not abandon started writes on a recording failure.
            while in_flight.next().await.is_some() {}
            break;
        }
    }
    failure.map_or(Ok(()), Err)
}
fn validate(definitions: &[ToolDefinition], call: &ToolCall) -> Option<Value> {
    let Some(definition) = definitions.iter().find(|d| d.name == call.function.name) else {
        return Some(json!({"error":"当前 Agent 未提供此工具", "code":"UNKNOWN_TOOL"}));
    };
    if call.function.arguments.to_string().len() > 24_000 {
        return Some(json!({"error":"工具参数超过 24 KB，请拆分操作", "code":"INVALID_ARGS"}));
    }
    super::schema::validate(&definition.parameters, &call.function.arguments)
        .err()
        .map(|error| json!({"error":error,"code":"INVALID_ARGS"}))
}

pub fn skip(
    host: &impl Host,
    session: &mut Session,
    calls: &[ToolCall],
    code: &str,
    reason: &str,
) -> Result<(), String> {
    for call in calls {
        host.record("tool/call",json!({"callId":call.id.as_str(),"name":call.function.name,"arguments":call.function.arguments}))?;
        let value = json!({"error":reason,"code":code});
        let message = result_message(call, &value);
        host.record("tool/result",json!({"callId":call.id.as_str(),"name":call.function.name,"result":value,"isError":true,"message":message}))?;
        session.messages.push(message);
    }
    Ok(())
}
