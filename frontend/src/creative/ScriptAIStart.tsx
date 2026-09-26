import { requestCreativeTask } from "./aiTasks";
export function ScriptAIStart({
  planId,
  manual,
}: {
  planId?: string;
  manual: () => void;
}) {
  return (
    <section className="script-ai-start" aria-label="AI 脚本起草">
      <small>脚本 → 镜头方案 → 分镜图</small>
      <h2>第一份脚本，从对话开始。</h2>
      <p>
        在右侧聊天里告诉创意编剧你想拍什么。用 @ 或素材右键菜单附加参考资料，
        生成的脚本会显示在这里。
      </p>
      <div className="script-ai-actions">
        <button
          className="primary"
          onClick={() =>
            requestCreativeTask({
              text: "",
              agentId: "concept",
              targetNodeId: planId,
            })
          }
        >
          让 Agent 起草脚本
        </button>
        <button onClick={manual}>手动写作</button>
      </div>
    </section>
  );
}
