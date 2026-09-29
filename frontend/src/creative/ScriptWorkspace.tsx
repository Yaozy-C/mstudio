import { t, useLanguage } from "../i18n";
import type { WorkContext } from "../assistant/workContext";
import { ScriptTiming } from "./ScriptTiming";
import { ScriptNextStep } from "./ScriptNextStep";
import { ScriptAIStart } from "./ScriptAIStart";
import { creativeTask, requestCreativeTask } from "./aiTasks";
import { useEffect, useState } from "react";
import type { Project } from "../model";
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
  useLanguage();
  const screenplays = project.nodes.filter((n) => n.kind === "screenplay");
  const selectedNode = project.nodes.find((n) => n.id === selected);
  const linkedScreenplay =
    selectedNode?.kind === "screenplay"
      ? selectedNode.id
      : selectedNode?.shot?.screenplayId;
  const [paragraphId, setParagraphId] = useState<string | undefined>();
  const [manual, setManual] = useState(false);
  const screenplay =
    screenplays.find((n) => n.id === linkedScreenplay) || screenplays[0];
  useEffect(() => {
    const id = selectedNode?.shot?.scriptId;
    if (id)
      document
        .getElementById(`script-${id}`)
        ?.scrollIntoView({ block: "center" });
  }, [selectedNode?.shot?.scriptId, screenplay?.id]);
  useEffect(() => {
    setParagraphId(undefined);
  }, [screenplay?.id]);
  const focusedParagraph = screenplay?.screenplay?.script?.some(
    (s) => s.id === paragraphId,
  )
    ? paragraphId
    : undefined;
  useEffect(() => {
    onContext?.({
      view: "script",
      screenplayId: screenplay?.id,
      paragraphId: focusedParagraph,
    });
  }, [screenplay?.id, focusedParagraph, onContext]);
  const script = screenplay?.screenplay?.script ?? [];
  const shots = screenplay ? shotsOf(project, screenplay.id) : [];
  return (
    <section className="script-workspace">
      <header className="script-heading">
        <div>
          <h1>{t("脚本")}</h1>
        </div>
      </header>
      {screenplay ? (
        <>
          <nav className="script-tools" aria-label={t("脚本工具")}>
            <button type="button" onClick={() => onReference(screenplay.id)}>
              {t("引用到对话")}
            </button>
            <span role="status" title={t(SAVE_DESCRIPTION)}>
              {t(saved)}
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
                key={screenplay.id}
                screenplayId={screenplay.id}
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
                    onSelect(screenplay.id);
                    navigate("storyboard");
                    requestCreativeTask(creativeTask("split", screenplay.id));
                  }}
                />
                <input
                  className="script-title"
                  aria-label={t("脚本标题")}
                  value={screenplay.title}
                  onChange={(e) =>
                    onChange((p) => ({
                      ...p,
                      nodes: p.nodes.map((n) =>
                        n.id === screenplay.id
                          ? { ...n, title: e.target.value }
                          : n,
                      ),
                    }))
                  }
                />
                {!!script?.length && (
                  <ScriptTiming
                    script={script}
                    screenplayId={screenplay.id}
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
                      screenplayId={screenplay.id}
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
                          n.id !== screenplay.id
                            ? n
                            : {
                                ...n,
                                screenplay: {
                                  ...n.screenplay!,
                                  script: [
                                    ...(n.screenplay?.script ?? []),
                                    newParagraph(),
                                  ],
                                },
                              },
                        ),
                      }))
                    }
                  >
                    {t("＋ 添加段落")}
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
