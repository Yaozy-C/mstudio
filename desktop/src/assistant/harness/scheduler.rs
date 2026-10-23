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
    let definitions = host.definitions()?;
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
        while failure.is_none() && next < calls.len() && in_flight.len() < super::MAX_CONCURRENCY {
            let index = next;
            let call = &calls[index];
            if let Err(error) = host.record("tool/call", json!({"callId":call.id.wire().as_ref(),"name":call.function.name,"arguments":call.function.arguments})) {
                failure = Some(error); break;
            }
            let invalid = validate_with_limit(definitions, call, host.tool_argument_limit());
            in_flight.push(async move {
                let value = if host.token().is_cancelled() {
                    json!({"error":"Task stopped before dispatch; call not executed", "code":"ABORTED_BEFORE_DISPATCH"})
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
            let mut message = result_message(
                call,
                if canonical.get("__offloadedImage").is_some()
                    || canonical.get("__offloadedMedia").is_some()
                {
                    &canonical
                } else {
                    &value
                },
            );
            super::read_reuse::annotate(&session.messages, call, &canonical, &mut message);
            // One durable result owns both the model history and the visible tool row.
            let result = host.record("tool/result", json!({"callId":call.id.wire().as_ref(),"name":call.function.name,"result":value,"value":canonical,"isError":value.get("error").is_some_and(|v| !v.is_null()),"message":message}));
            if let Err(error) = result {
                failure.get_or_insert(error);
            } else {
                session.delegation_outcomes.observe(&canonical);
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
#[cfg(test)]
pub(super) fn validate(definitions: &[ToolDefinition], call: &ToolCall) -> Option<Value> {
    validate_with_limit(definitions, call, 24_000)
}
fn validate_with_limit(
    definitions: &[ToolDefinition],
    call: &ToolCall,
    limit: usize,
) -> Option<Value> {
    let Some(definition) = definitions.iter().find(|d| d.name == call.function.name) else {
        return Some(json!({"error":"Tool not available to this Agent", "code":"UNKNOWN_TOOL"}));
    };
    if serde_json::to_string(&call.function.arguments)
        .unwrap_or_default()
        .len()
        > limit
    {
        return Some(
            json!({"error":format!("Tool arguments exceed {limit} bytes; split the operations"), "code":"INVALID_ARGS"}),
        );
    }
    let issues = super::schema::issues(
        &definition.parameters,
        &serde_json::Value::Object(call.function.arguments.clone()),
    );
    (!issues.is_empty()).then(|| super::schema::rejection(issues))
}

pub fn skip(
    host: &impl Host,
    session: &mut Session,
    calls: &[ToolCall],
    code: &str,
    reason: &str,
) -> Result<(), String> {
    for call in calls {
        host.record("tool/call",json!({"callId":call.id.wire().as_ref(),"name":call.function.name,"arguments":call.function.arguments}))?;
        let value = json!({"error":reason,"code":code});
        let message = result_message(call, &value);
        host.record("tool/result",json!({"callId":call.id.wire().as_ref(),"name":call.function.name,"result":value,"value":value,"isError":true,"message":message}))?;
        session.messages.push(message);
    }
    Ok(())
}
