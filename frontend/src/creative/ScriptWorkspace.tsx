import type { WorkContext } from "../assistant/workContext";
import { ScriptTiming } from "./ScriptTiming";
import { ScriptNextStep } from "./ScriptNextStep";
import { ScriptAIStart } from "./ScriptAIStart";
import { creativeTask, requestCreativeTask } from "./aiTasks";
import { useEffect, useState } from "react";
import type { Project } from "../model";
import { InlineText } from "./InlineText";
import { SAVE_DESCRIPTION } from "../workspace/projectAutosave";
import { ScriptParagraphEditor } from "./ScriptParagraphEditor";
import { createScript, newParagraph } from "./script";
import { shotsOf } from "./document";
import "../styles/script.css";

export function ScriptWorkspace({
  project,
  selected,
  onSelect,
  onChange,
  onReference,
  saved,
  navigate,
  onContext,
}: {
  onContext?: (context: WorkContext) => void;
  project: Project;
  selected: string | null;
  onSelect: (id: string | null) => void;
  onChange: (fn: (p: Project) => Project) => void;
  onReference: (id: string) => void;
  saved: string;
  navigate: (view: "script" | "storyboard") => void;
}) {
  const plans = project.nodes.filter((n) => n.kind === "plan");
  const selectedNode = project.nodes.find((n) => n.id === selected);
  const linkedPlan =
    selectedNode?.kind === "plan"
      ? selectedNode.id
      : selectedNode?.shot?.planId;
  const [paragraphId, setParagraphId] = useState<string | undefined>();
  const [manual, setManual] = useState(false);
  const plan = plans.find((n) => n.id === linkedPlan) || plans[0];
  useEffect(() => {
    const id = selectedNode?.shot?.scriptId;
    if (id)
      document
        .getElementById(`script-${id}`)
        ?.scrollIntoView({ block: "center" });
  }, [selectedNode?.shot?.scriptId, plan?.id]);
  useEffect(() => {
    setParagraphId(undefined);
  }, [plan?.id]);
  const focusedParagraph = plan?.plan?.script?.some((s) => s.id === paragraphId)
    ? paragraphId
    : undefined;
  useEffect(() => {
    onContext?.({
      view: "script",
      planId: plan?.id,
      paragraphId: focusedParagraph,
    });
  }, [plan?.id, focusedParagraph, onContext]);
  const script = plan?.plan?.script ?? [];
  const shots = plan ? shotsOf(project, plan.id) : [];
  return (
    <section className="script-workspace">
      <header className="script-heading">
        <div>
          <small>01 / SCRIPT</small>
          <h1>从想法开始，让 AI 起草。</h1>
          <p>先写清声画内容，再设计整片镜头与关键画格。</p>
        </div>
      </header>
      {plan ? (
        <>
          <nav className="script-tools" aria-label="脚本工具">
            <button type="button" onClick={() => onReference(plan.id)}>
              引用到对话
            </button>
            <span role="status" title={SAVE_DESCRIPTION}>
              {saved}
            </span>
          </nav>
          <div
            className="script-paper"
            onFocusCapture={(e) => {
              if (!(e.target as HTMLElement).closest(".script-paragraph"))
                setParagraphId(undefined);
            }}
          >
            {!manual &&
            !script?.some((s) => s.action.trim() || s.dialogue.trim()) ? (
              <ScriptAIStart
                key={plan.id}
                planId={plan.id}
                manual={() => setManual(true)}
              />
            ) : (
              <>
                <ScriptNextStep
                  hasScript={
                    !!script?.some((s) => s.action.trim() || s.dialogue.trim())
                  }
                  count={shots.length}
                  next={() => {
                    onSelect(plan.id);
                    navigate("storyboard");
                    requestCreativeTask(creativeTask("split", plan.id));
                  }}
                />
                <input
                  className="script-title"
                  aria-label="脚本标题"
                  value={plan.title}
                  onChange={(e) =>
                    onChange((p) => ({
                      ...p,
                      nodes: p.nodes.map((n) =>
                        n.id === plan.id ? { ...n, title: e.target.value } : n,
                      ),
                    }))
                  }
                />
                <InlineText
                  context="脚本"
                  label="创意概述"
                  value={plan.text}
                  limit={6000}
                  placeholder="想表达什么，为什么值得看？"
                  commit={(text) =>
                    onChange((p) => ({
                      ...p,
                      nodes: p.nodes.map((n) =>
                        n.id === plan.id ? { ...n, text } : n,
                      ),
                    }))
                  }
                />
                {plan.plan?.story && (
                  <p className="script-origin">故事结构：{plan.plan.story}</p>
                )}
                {!!script?.length && (
                  <ScriptTiming
                    script={script}
                    planId={plan.id}
                    change={onChange}
                  />
                )}
                <>
                  {script.map((s, i) => (
                    <ScriptParagraphEditor
                      key={s.id}
                      paragraph={s}
                      select={() => setParagraphId(s.id)}
                      index={i}
                      planId={plan.id}
                      shots={shots.filter((n) => n.shot?.scriptId === s.id)}
                      change={onChange}
                      openShots={(id) => {
                        onSelect(id ?? null);
                        navigate("storyboard");
                      }}
                    />
                  ))}
                  <button
                    className="script-add"
                    onClick={() =>
                      onChange((p) => ({
                        ...p,
                        nodes: p.nodes.map((n) =>
                          n.id !== plan.id
                            ? n
                            : {
                                ...n,
                                plan: {
                                  ...n.plan!,
                                  script: [
                                    ...(n.plan?.script ?? []),
                                    newParagraph(),
                                  ],
                                },
                              },
                        ),
                      }))
                    }
                  >
                    ＋ 添加段落
                  </button>
                </>
              </>
            )}
          </div>
        </>
      ) : (
        <ScriptAIStart
          manual={() =>
            onChange((p) => {
              const next = createScript(p);
              setManual(true);
              return next;
            })
          }
        />
      )}
    </section>
  );
}
