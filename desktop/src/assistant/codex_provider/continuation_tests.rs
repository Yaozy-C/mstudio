use super::*;
use rig_core::{
    completion::ToolDefinition,
    message::{AssistantContent, ImageMediaType, Message, ToolResultContent, UserContent},
};

#[tokio::test]
async fn native_turn_survives_multiple_tools_with_errors_images_and_message_phases() {
    use std::os::unix::fs::PermissionsExt;
    let path = std::env::temp_dir().join(format!("mstudio-native-turn-{}", mstudio::media::id()));
    std::fs::write(&path, r#"#!/usr/bin/env python3
import sys,json
def read(): return json.loads(sys.stdin.readline())
def send(v): print(json.dumps(v),flush=True)
r=read(); assert r['method']=='initialize'; send({'id':r['id'],'result':{}})
assert read()['method']=='initialized'
r=read(); assert r['method']=='thread/start'; send({'id':r['id'],'result':{'thread':{'id':'thread'}}})
r=read(); assert r['method']=='turn/start'; send({'id':r['id'],'result':{'turn':{'id':'turn'}}})
send({'id':700,'method':'item/tool/call','params':{'threadId':'thread','turnId':'turn','callId':'one','tool':'echo','arguments':{}}})
r=read(); assert r['id']==700 and r['result']['success'] is False
assert json.loads(r['result']['contentItems'][0]['text'])['error']=='Try a different input'
send({'method':'thread/tokenUsage/updated','params':{'threadId':'thread','turnId':'turn','tokenUsage':{'modelContextWindow':258400,'total':{'inputTokens':1000,'outputTokens':20,'totalTokens':1020},'last':{'inputTokens':1000,'outputTokens':20,'totalTokens':1020}}}})
send({'method':'thread/tokenUsage/updated','params':{'threadId':'other','turnId':'turn','tokenUsage':{'modelContextWindow':8192,'total':{'totalTokens':999999},'last':{'totalTokens':999999}}}})
send({'method':'item/started','params':{'threadId':'thread','turnId':'turn','item':{'id':'note','type':'agentMessage','phase':'commentary'}}})
send({'method':'item/agentMessage/delta','params':{'threadId':'thread','turnId':'turn','itemId':'note','delta':'Looking at the reference.'}})
send({'id':701,'method':'item/tool/call','params':{'threadId':'thread','turnId':'turn','callId':'two','tool':'image','arguments':{}}})
r=read(); assert r['id']==701 and r['result']['success'] is True
assert r['result']['contentItems'][0]['type']=='inputImage'
assert r['result']['contentItems'][0]['imageUrl']=='data:image/png;base64,AA=='
send({'method':'thread/tokenUsage/updated','params':{'threadId':'thread','turnId':'turn','tokenUsage':{'modelContextWindow':258400,'total':{'inputTokens':2100,'outputTokens':40,'totalTokens':2140},'last':{'inputTokens':1100,'outputTokens':20,'totalTokens':1120}}}})
send({'method':'item/started','params':{'threadId':'thread','turnId':'turn','item':{'id':'answer','type':'agentMessage','phase':'final_answer'}}})
send({'method':'item/agentMessage/delta','params':{'threadId':'other','turnId':'turn','delta':'UNRELATED'}})
send({'method':'item/agentMessage/delta','params':{'threadId':'thread','turnId':'turn','itemId':'answer','delta':'Saved.'}})
send({'method':'thread/tokenUsage/updated','params':{'threadId':'thread','turnId':'turn','tokenUsage':{'modelContextWindow':258400,'total':{'inputTokens':3300,'outputTokens':50,'totalTokens':3350},'last':{'inputTokens':1200,'outputTokens':10,'totalTokens':1210}}}})
send({'method':'turn/completed','params':{'threadId':'thread','turn':{'id':'turn','status':'completed'}}})
read()
"#).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let model = CodexModel::new("test".into());
    let mut request = model
        .completion_request("Inspect then save.")
        .tools(
            ["echo", "image"]
                .map(|name| ToolDefinition {
                    name: name.into(),
                    description: name.into(),
                    parameters: json!({"type":"object"}),
                })
                .to_vec(),
        )
        .build();
    let rpc = Rpc::connect(&path, "--stdio", false).await.unwrap();
    let mut connection = Connection::open(rpc, "test", request.clone())
        .await
        .unwrap();
    let pending = connection.rpc.next().await.unwrap();
    let RawStreamingChoice::ToolCall(_) = tool_frame(&pending, &["echo".into()]).unwrap() else {
        panic!()
    };
    // Use the same canonical tool pairing the harness appends after execution.
    let first = rig_core::message::ToolCall::from_wire(
        "one",
        rig_core::message::ToolFunction {
            name: "echo".into(),
            arguments: json!({}),
        },
    );
    request.chat_history.push(Message::Assistant {
        id: None,
        content: vec![AssistantContent::ToolCall(first)],
    });
    request.chat_history.push(Message::tool_result(
        "one",
        "echo",
        r#"{"error":"Try a different input"}"#,
    ));
    connection.pending = pending;
    assert!(connection.continuation(&request).is_some());
    let mut altered = request.clone();
    altered.tools.clear();
    assert!(connection.continuation(&altered).is_none());
    let mut altered = request.clone();
    altered.chat_history.push(Message::user("New task"));
    assert!(connection.continuation(&altered).is_none());
    *model.waiting.lock().unwrap() = Some(connection);
    let response = model.completion(request.clone()).await.unwrap();
    assert_eq!(response.usage.total_tokens, 1020);
    assert_eq!(response.raw["tokenUsage"]["modelContextWindow"], 258400);
    assert_eq!(response.raw["contextMessageCount"], 2);
    assert!(response.choice.iter().any(|part| matches!(part, AssistantContent::Text(t) if t.additional_params.as_ref().and_then(|p|p.get("phase"))==Some(&json!("commentary")))));
    let second_call = response
        .choice
        .iter()
        .find_map(|p| match p {
            AssistantContent::ToolCall(c) => Some(c.clone()),
            _ => None,
        })
        .unwrap();
    request.chat_history.push(Message::Assistant {
        id: None,
        content: response.choice,
    });
    request.chat_history.push(Message::User {
        content: vec![UserContent::tool_result_for(
            second_call.id,
            second_call.provider,
            "image",
            vec![ToolResultContent::Image(rig_core::message::Image {
                data: rig_core::message::DocumentSourceKind::Base64("AA==".into()),
                media_type: Some(ImageMediaType::PNG),
                ..Default::default()
            })],
        )],
    });
    let response = model.completion(request).await.unwrap();
    assert_eq!(response.usage.total_tokens, 2330);
    assert_eq!(response.raw["tokenUsage"]["last"]["totalTokens"], 1210);
    assert!(response.raw["contextMessageCount"].is_null());
    let text = response
        .choice
        .iter()
        .filter_map(|part| match part {
            AssistantContent::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect::<String>();
    assert_eq!(text, "Saved.");
    assert!(model.waiting.lock().unwrap().is_none());
    std::fs::remove_file(path).unwrap();
}
