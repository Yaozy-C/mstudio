import { useState, type ComponentProps } from "react";
import { createRoot } from "react-dom/client";
import { Theme } from "@radix-ui/themes";
import { StudioChrome } from "../src/workspace/StudioChrome";
import { StoryAssets } from "../src/workspace/StoryAssets";
import type { StudioView } from "../src/workspace/StudioStage";
import { newProject, type Asset, type Project } from "../src/model";
import { setLanguage, t, useLanguage } from "../src/i18n";
import { ListChecks, Cube } from "@phosphor-icons/react";
import "@radix-ui/themes/styles.css";
import "./studio-fixture.css";

const image = `data:image/svg+xml,${encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="640" height="480"><rect width="640" height="480" fill="#d2c8b5"/><rect x="200" y="120" width="240" height="240" rx="24" fill="#f3efe7"/><circle cx="320" cy="240" r="60" fill="#b65d43"/></svg>')}`;
function sample(): Project {
  const p = newProject("资产工作区验收");
  const base: Asset = {
    id: "hero",
    name: "角色 · 主角正面参考",
    kind: "image",
    path: image,
    preview: image,
    duration: 0,
    width: 640,
    height: 480,
    hasAudio: false,
    usage: "asset",
  };
  p.assets = [
    base,
    { ...base, id: "scene", name: "场景 · 午后的厨房", imageVersionCount: 3 },
    { ...base, id: "prop", name: "物品 · 便携餐盒 Hero Product Reference" },
    { ...base, id: "missing", name: "角色 · 文件缺失示例", missing: true },
    {
      ...base,
      id: "candidate",
      name: "场景 · 新生成的候选参考",
      generated: true,
    },
    { ...base, id: "material", name: "待添加的普通素材", usage: "material" },
    {
      ...base,
      id: "audio",
      name: "音频 · 声音参考",
      kind: "audio",
      path: "",
      preview: "",
      duration: 10,
    },
    {
      ...base,
      id: "text",
      name: "文档 · 角色设定",
      kind: "text",
      path: "",
      preview: "",
    },
  ];
  p.nodes = [
    {
      id: "shot",
      kind: "shot",
      title: "镜头",
      text: "",
      x: 0,
      y: 0,
      references: [{ assetId: "hero", purpose: "character" }],
    },
  ];
  return p;
}

function Fixture() {
  useLanguage();
  const [project, change] = useState(sample);
  const [view, setView] = useState<StudioView>("assets");
  const [feedback, setFeedback] = useState("");
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
      <div
        style={{
          height: 42,
          display: "flex",
          alignItems: "center",
          gap: 12,
          padding: "0 16px",
        }}
      >
        <button onClick={() => change(sample())}>重置示例</button>
        <button onClick={() => change(newProject("空资产"))}>空状态</button>
        <button onClick={() => setLanguage("en")}>English</button>
        <button onClick={() => setLanguage("zh-CN")}>中文</button>
        <span role="status">{feedback}</span>
      </div>
      <div
        className="studio-shell timeline-hidden"
        data-media="false"
        data-agent="false"
        data-view={view}
        style={{ height: "calc(100vh - 42px)", position: "relative" }}
      >
        <StudioChrome
          view={view}
          onView={setView}
          canInspect={false}
          tasks={
            <>
              <button className="chrome-action task-dialog-trigger">
                <ListChecks />
                {t("生成任务")} <span>2</span>
              </button>
              <button className="chrome-action task-dialog-trigger">
                <Cube />
                {t("预演视频")} <span>1</span>
              </button>
            </>
          }
          m={model}
          onBack={() => {}}
          setError={setFeedback}
          onModels={() => {}}
          onExport={() => {}}
          togglePanel={() => {}}
          panels={{
            media: false,
            preview: false,
            inspector: false,
            agent: false,
          }}
          addNote={() => {}}
          onCreation={() => {}}
        />
        <main className={`studio-stage view-${view}`}>
          {view === "assets" ? (
            <StoryAssets
              project={project}
              change={change}
              reference={(a) => setFeedback(`引用到对话：${a.name}`)}
              place={(a) => setFeedback(`放到画布：${a.name}`)}
            />
          ) : (
            <p>切换到「资产」查看。</p>
          )}
        </main>
      </div>
    </Theme>
  );
}
createRoot(document.getElementById("root")!).render(<Fixture />);
