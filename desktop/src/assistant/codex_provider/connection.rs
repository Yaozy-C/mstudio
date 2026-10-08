//! A native turn pauses only while the host executes its dynamic tool.
use super::*;
use rig_core::message::{AssistantContent, Message, ToolResult, UserContent};

pub(super) struct Connection {
    pub rpc: Rpc,
    pub request: CompletionRequest,
    pub pending: Value,
    pub thread: String,
    pub turn: String,
    pub usage: super::usage::NativeUsage,
}
impl Connection {
    pub async fn start(model: &str, request: CompletionRequest) -> Result<Self, CompletionError> {
        let setup = async {
            let mut rpc = Rpc::start_text().await.map_err(failure)?;
            let account = rpc
                .call("account/read", json!({"refreshToken":false}))
                .await
                .map_err(failure)?;
            if account["account"]["type"] != "chatgpt" {
                return Err(failure("请在服务连接中登录 Codex"));
            }
            Self::open(rpc, model, request).await
        };
        tokio::time::timeout(Duration::from_secs(40), setup)
            .await
            .map_err(failure)?
    }
    pub async fn open(
        mut rpc: Rpc,
        model: &str,
        request: CompletionRequest,
    ) -> Result<Self, CompletionError> {
        let input =
            crate::assistant::codex_input::turn_input(&request.chat_history).map_err(failure)?;
        let tools: Vec<_> = request
            .tools
            .iter()
            .map(|t| json!({"name":t.name,"description":t.description,"inputSchema":t.parameters}))
            .collect();
        let started = rpc.call("thread/start", json!({
            "model":model,"ephemeral":true,"cwd":std::env::temp_dir(),
            "approvalPolicy":"untrusted","sandbox":"read-only","dynamicTools":tools,
            "developerInstructions":format!("You are the conversation model for Mstudio. Continue the provided serialized conversation history. Use only the supplied dynamic tools; Mstudio executes them. Never use built-in tools, shell, filesystem, web, MCP or plugins. Answer the user's latest request. {}",request.preamble.as_deref().unwrap_or("")),
            "config":{"features":{"shell_tool":false,"apply_patch_freeform":false},"web_search":"disabled"}
        })).await.map_err(failure)?;
        let thread = started["thread"]["id"]
            .as_str()
            .ok_or_else(|| failure("Codex 未返回会话"))?
            .to_owned();
        let started = rpc
            .call(
                "turn/start",
                json!({"threadId":thread,"input":input,"outputSchema":request.output_schema}),
            )
            .await
            .map_err(failure)?;
        let turn = started["turn"]["id"]
            .as_str()
            .ok_or_else(|| failure("Codex 未返回回合"))?
            .to_owned();
        Ok(Self {
            rpc,
            request,
            pending: Value::Null,
            thread,
            turn,
            usage: Default::default(),
        })
    }
    pub fn accepts(&self, params: &Value) -> bool {
        params["threadId"]
            .as_str()
            .is_none_or(|id| id == self.thread)
            && params["turnId"]
                .as_str()
                .or(params["turn"]["id"].as_str())
                .is_none_or(|id| id == self.turn)
    }
    pub fn continuation<'a>(&self, next: &'a CompletionRequest) -> Option<&'a ToolResult> {
        if envelope(&self.request) != envelope(next)
            || !next.chat_history.starts_with(&self.request.chat_history)
        {
            return None;
        }
        let suffix = &next.chat_history[self.request.chat_history.len()..];
        let [
            Message::Assistant { content, .. },
            Message::User { content: results },
        ] = suffix
        else {
            return None;
        };
        let calls: Vec<_> = content
            .iter()
            .filter_map(|c| match c {
                AssistantContent::ToolCall(c) => Some(c),
                _ => None,
            })
            .collect();
        let [call] = calls.as_slice() else {
            return None;
        };
        let [UserContent::ToolResult(result)] = results.as_slice() else {
            return None;
        };
        (call.id.as_str() == self.pending["params"]["callId"].as_str()?
            && result.call == call.id
            && result.name == call.function.name)
            .then_some(result)
    }
}
fn envelope(request: &CompletionRequest) -> Value {
    json!({"tools":request.tools,"preamble":request.preamble,"output":request.output_schema,
        "toolChoice":request.tool_choice,"model":request.model})
}
