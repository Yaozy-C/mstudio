import { useState } from "react";
import { createRoot } from "react-dom/client";
import { ScriptWorkspace } from "../src/creative/ScriptWorkspace";
import { createScript, newParagraph } from "../src/creative/script";
import { newProject } from "../src/model";
import { defaultAgent, type WorkContext } from "../src/assistant/workContext";
const seed = createScript(newProject("上下文隔离测试"));
seed.nodes.forEach((n, i) => {
  n.title = `测试脚本 ${i + 1}`;
  if (n.plan)
    n.plan.script = [
      {
        ...newParagraph(),
        title: `段落 ${i + 1}`,
        action: "原有动作，不调用远端模型。",
      },
    ];
});
function Test() {
  const [project, change] = useState(seed);
  const [selected, select] = useState<string | null>(null);
  const [work, setWork] = useState<WorkContext>({ view: "script" });
  return (
    <>
      <h1>脚本上下文验收 · 模拟工程</h1>
      <pre role="status">
        {JSON.stringify({
          script: project.nodes.find((n) => n.id === work.planId)?.title,
          paragraph: project.nodes
            .find((n) => n.id === work.planId)
            ?.plan?.script?.find((s) => s.id === work.paragraphId)?.title,
          agent: defaultAgent(work),
        })}
      </pre>
      <ScriptWorkspace
        project={project}
        selected={selected}
        onSelect={select}
        onChange={change}
        onReference={() => {}}
        saved="不保存到真实工程"
        navigate={() => {}}
        onContext={setWork}
      />
    </>
  );
}
createRoot(document.getElementById("root")!).render(<Test />);
