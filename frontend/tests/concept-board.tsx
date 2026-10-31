import { useEffect, useState, type ComponentProps } from "react";
import { createRoot } from "react-dom/client";
import { Theme } from "@radix-ui/themes";
import { StudioChrome } from "../src/workspace/StudioChrome";
import { StudioStage, type StudioView } from "../src/workspace/StudioStage";
import { PlaybackClock } from "../src/timeline/clock";
import { newProject, type Concept, type Project } from "../src/model";
import { setLanguage } from "../src/i18n";
import { initialStudioView } from "../src/workspace/initialStudioView";
import type { CreativeTask } from "../src/creative/aiTasks";
import "@radix-ui/themes/styles.css";
import "./studio-fixture.css";

// Synthetic project only. No native persistence, account or generation calls.
const initial = newProject("创意板验收");
const sample: Concept = {
  id: "sample",
  title: "《午餐的另一种打开方式》",
  oneLine: "忙碌的一天，在打开餐盒的那一刻，重新找回自己的节奏。",
  participant: "一周三趟出差的年轻职员。",
  want: "想在移动中证明自己还有生活。",
  ritual: "检票、落座、小桌板放下的一刻。",
  evidence: "小桌板放下的瞬间，自家饭盒与邻座盒饭并置。两种容器，两种声音。",
  reframe: "他不是在路上吃饭，他是把家带到了路上。",
  risk: "场景依赖公共交通，拍摄难度较高；需提前协调场地与机位，避免影响其他乘客。",
  cost: "中：车厢拍摄限制较多。",
  status: "rough",
  createdAt: 1,
  updatedAt: 1,
};
const preview = new URLSearchParams(window.location.search).has("preview");
const sampleCards = () =>
  Array.from({ length: 13 }, (_, i) => ({
    ...sample,
    id: `concept-${i}`,
    title: `${sample.title} ${i + 1}`,
    status: (["rough", "developed", "selected", "killed"] as const)[i % 4],
  }));
const clock = new PlaybackClock();
function Fixture() {
  const [project, change] = useState<Project>(() =>
    preview ? { ...initial, concepts: sampleCards() } : initial,
  );
  const [view, setView] = useState<StudioView>(() =>
    initialStudioView(initial),
  );
  const [task, setTask] = useState<CreativeTask | null>(null);
  useEffect(() => {
    const receive = (event: Event) =>
      setTask((event as CustomEvent<CreativeTask>).detail);
    window.addEventListener("studio-creative-request", receive);
    return () => window.removeEventListener("studio-creative-request", receive);
  }, []);
  const [wide, setWide] = useState(preview);
  const model = {
    project,
    saved: "已保存",
    canUndo: false,
    canRedo: false,
    conflicted: false,
    flush: async () => {},
    undo: () => {},
    redo: () => {},
  } as ComponentProps<typeof StudioChrome>["m"];
  return (
    <Theme appearance="light" accentColor="tomato" radius="small">
      {!preview && (
        <div
          style={{
            height: 42,
            display: "flex",
            gap: 12,
            alignItems: "center",
            padding: "0 16px",
          }}
        >
          <button
            onClick={() =>
              change((p) => ({
                ...p,
                concepts: sampleCards(),
              }))
            }
          >
            载入 13 张演示卡
          </button>
          <button onClick={() => change((p) => ({ ...p, concepts: [] }))}>
            清空演示卡
          </button>
          <button onClick={() => setLanguage("en")}>English</button>
          <button onClick={() => setLanguage("zh-CN")}>中文</button>
          <button onClick={() => setWide(!wide)}>切换侧栏</button>
        </div>
      )}
      <div
        className="studio-shell timeline-hidden"
        data-media={!wide}
        data-agent={!wide}
        data-view={view}
        style={{
          height: preview ? "100vh" : "calc(100vh - 42px)",
          position: "relative",
        }}
      >
        {task && !preview && (
          <aside
            aria-label="待发送任务"
            style={{
              position: "absolute",
              right: 16,
              top: 85,
              width: 380,
              maxHeight: "70vh",
              overflow: "auto",
              padding: 16,
            }}
          >
            <h3>{task.agentId}</h3>
            <p>{task.text}</p>
          </aside>
        )}
        <StudioChrome
          view={view}
          onView={setView}
          canInspect={false}
          tasks={null}
          m={model}
          onBack={() => {}}
          setError={() => {}}
          onModels={() => {}}
          onExport={() => {}}
          togglePanel={() => {}}
          panels={{
            media: !wide,
            preview: false,
            inspector: false,
            agent: !wide,
          }}
          addNote={() => {}}
          onCreation={() => {}}
        />
        <StudioStage
          view={view}
          project={project}
          onChange={change}
          onSelect={() => {}}
          selected={null}
          onReference={() => {}}
          saved="已保存"
          navigate={setView}
          clock={clock}
          onAdd={() => {}}
          onPlace={() => {}}
          onAssetReference={() => {}}
          canvas={
            {
              items: [],
              selected: [],
              choose: () => {},
            } as unknown as ComponentProps<typeof StudioStage>["canvas"]
          }
        />
      </div>
    </Theme>
  );
}
createRoot(document.getElementById("root")!).render(<Fixture />);
