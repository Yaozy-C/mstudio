//! Known host diagnostics in model-facing results; leave user/provider evidence intact.
pub(super) fn error(raw: &str) -> String {
    let exact = match raw {
        "项目不存在" => "Project not found",
        "Agent 不存在，请重新选择" => "Agent not found; select another role",
        "Agent 已停用" => "Agent disabled",
        "已停止回答；已输出内容和已完成操作保留" => {
            "Response stopped; streamed content and saved operations retained"
        }
        "已停止回答；已完成操作保留" => "Response stopped; saved operations retained",
        "已停止回答" => "Response stopped",
        "本轮已达到 20 分钟时限；已完成操作保留" => {
            "Turn reached the 20-minute limit; saved operations retained"
        }
        "本轮达到执行步数上限；已完成操作保留，剩余任务需继续处理" => {
            "Turn reached the step limit; saved operations retained and remaining work is incomplete"
        }
        "该模型报告上下文窗口已满；当前输入无法继续压缩，请减少本轮附件或调整模型窗口设置" => {
            "Provider reported context overflow; no further compaction possible. Reduce attachments or adjust model context settings"
        }
        "模型返回重复的工具调用标识，本次操作未执行" => {
            "Duplicate tool call IDs; these operations were not executed"
        }
        "模型拒绝了本次请求；未执行本次工具调用" => {
            "Model refused the request; these tool calls were not executed"
        }
        "模型输出被截断；已输出内容保留，未执行本次不完整的工具调用" => {
            "Model output truncated; text retained, incomplete tool calls not executed"
        }
        "模型回答为空" => "Empty model response",
        "模型回答过长；已输出的内容保留" => {
            "Model response too long; streamed content retained"
        }
        "压缩范围无效" => "Invalid compaction range",
        "压缩旧对话超时；已完成操作保留" => {
            "Compaction timed out; saved operations retained"
        }
        "模型协议插件未安装" => "Model protocol adapter not installed",
        "无法创建模型客户端" => "Cannot create model client",
        "无法创建 HTTP 客户端" => "Cannot create HTTP client",
        "本次模型请求超时，请检查模型服务或稍后继续；已完成的操作保留。" => {
            "Model request timed out; check the service or continue later. Saved operations retained."
        }
        "Gemini 工具调用签名缺失或无效（HTTP 400）；请在模型设置中使用 Gemini 原生协议。已完成的操作保留。" => {
            "Missing/invalid Gemini tool-call signature (HTTP 400); use the native Gemini protocol. Saved operations retained."
        }
        _ => "",
    };
    if !exact.is_empty() {
        return exact.into();
    }
    for (prefix, english) in [
        (
            "连续三次修改遇到相同错误，已停止重复尝试；已完成操作保留。请核实冲突原因后继续：",
            "Three edits returned the same error; stopped retries, retaining saved work. Resolve the conflict before continuing: ",
        ),
        (
            "模型服务拒绝认证，请检查 API Key 和权限",
            "Model authentication failed; check credentials and permissions",
        ),
        (
            "模型或接口不存在，请核对模型 ID 和服务地址",
            "Model/endpoint not found; verify model ID and endpoint",
        ),
        (
            "模型服务限流或额度不足，请检查额度后重试",
            "Model rate limit or quota exhausted; check quota before retrying",
        ),
        ("模型服务拒绝了请求", "Model service rejected the request"),
        (
            "模型服务暂时异常，请稍后重试",
            "Model service temporarily unavailable; retry later",
        ),
        ("模型服务请求失败", "Model request failed"),
        (
            "无法连接模型服务，请检查网络和服务地址",
            "Cannot connect to model service; check network and endpoint",
        ),
        (
            "模型响应无法解析，请检查接口协议是否与服务一致",
            "Cannot parse model response; verify protocol compatibility",
        ),
        (
            "无法构造模型请求，请检查接口支持的资料类型与工具格式",
            "Cannot construct model request; check supported media and tool formats",
        ),
        (
            "读取对话上下文失败，请重试",
            "Cannot read conversation context; retry",
        ),
        ("模型执行失败，请重试", "Model execution failed; retry"),
    ] {
        if let Some(detail) = raw.strip_prefix(prefix) {
            let detail = detail
                .strip_suffix("。已完成的操作保留。")
                .unwrap_or(detail);
            return format!("{english}{detail}. Saved operations retained.");
        }
    }
    raw.into()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn translates_host_errors_without_altering_evidence() {
        assert_eq!(error("外部供应商原话"), "外部供应商原话");
        let result = error("模型服务拒绝了请求（HTTP 400）：供应商原话。已完成的操作保留。");
        assert!(result.starts_with("Model service rejected"));
        assert!(result.contains("HTTP 400"));
        assert!(result.contains("供应商原话"));
    }
}
