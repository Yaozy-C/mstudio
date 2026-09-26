import { ErrorNotice } from "../errors/ErrorNotice";
import { useAgents } from "./catalog";
import { tools } from "./tools";
export function ToolLibrary({ openAgents }: { openAgents: () => void }) {
  const hub = useAgents();
  return (
    <section>
      <div className="hub-intro">
        <div className="eyebrow">TOOLS · 执行工具</div>
        <h2>给每个 Agent 配置需要的工具。</h2>
        <p>
          工具负责执行操作，Skills 提供方法和规则。在 Agent
          详情中独立配置工具权限。
        </p>
      </div>
      <div className="hub-section-head">
        <h3>内置工具</h3>
        <button onClick={openAgents}>配置 Agent 工具</button>
      </div>
      {tools.map((tool) => {
        const assigned = hub.agents.filter((a) => a.toolIds.includes(tool.id));
        return (
          <article className="hub-feature" key={tool.id}>
            <div className="hub-section-head">
              <div>
                <h3>{tool.name}</h3>
                <p>{tool.description}</p>
              </div>
              <span className="model-badge">{tool.kind}</span>
            </div>
            <p className="model-hint">
              已配置给：{assigned.map((a) => a.name).join("、") || "暂无 Agent"}
            </p>
          </article>
        );
      })}
      <p className="model-hint">
        媒体生成使用独立模型目录中的模型，提交前仍需确认。本地导出是 Studio
        的基础功能，当前没有开放给 Agent 的导出工具。
      </p>
      {hub.error && (
        <ErrorNotice error={hub.error} fallback="OPERATION_FAILED" />
      )}
    </section>
  );
}
