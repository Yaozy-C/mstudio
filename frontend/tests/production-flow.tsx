import { canvasAsset } from "../src/workspace/canvasAsset";
import { removeNodes } from "../src/canvas/removeNodes";
import { useEffect, useState, type ComponentProps } from "react";
import { createRoot } from "react-dom/client";
import { Theme, Dialog } from "@radix-ui/themes";
import { ListChecks } from "@phosphor-icons/react";
import { StudioChrome } from "../src/workspace/StudioChrome";
import { StudioStage, type StudioView } from "../src/workspace/StudioStage";
import { draftItems } from "../src/production/items";
import type { ProductionController } from "../src/production/useProduction";
import { runsOf } from "../src/production/requestTask";
import { newProject, type Asset } from "../src/model";
import { PlaybackClock } from "../src/timeline/clock";
import { setLanguage, t, useLanguage } from "../src/i18n";
import type { CreativeTask } from "../src/creative/aiTasks";
import type { ProductionTask } from "../src/production/types";
import { GenerationRun } from "../src/production/GenerationRun";
import "@radix-ui/themes/styles.css";
import "./studio-fixture.css";

const picture = (color: string) =>
  `data:image/svg+xml,${encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" width="640" height="400"><rect width="640" height="400" fill="#d8cfbd"/><rect x="140" y="80" width="360" height="240" rx="30" fill="${color}"/><circle cx="320" cy="200" r="58" fill="#eee9de"/></svg>`)}`;
function sample() {
  const p = newProject("把家带到路上");
  const asset = (id: string, name: string, color = "#b56449"): Asset => ({
    id,
    name,
    kind: "image",
    path: picture(color),
    preview: picture(color),
    width: 640,
    height: 400,
    duration: 0,
    hasAudio: false,
  });
  p.assets = [
    asset("source", "产品实拍 · 创意与脚本资料"),
    { ...asset("hero", "小满 · 主角身份", "#776b60"), usage: "asset" },
    { ...asset("box", "餐盒 · 商品外观"), usage: "asset" },
    {
      ...asset("candidate", "厨房 · 候选场景", "#a39f7b"),
      usage: "asset",
      generated: true,
    },
    { ...asset("frame1", "01 · 打开餐盒分镜图"), generated: true },
    { ...asset("frame2", "02 · 车窗边的午餐", "#8c998b"), generated: true },
    {
      ...asset("video1", "01 · 视频片段 A"),
      path: "",
      kind: "video",
      duration: 6,
      generated: true,
    },
    {
      ...asset("video2", "02 · 视频片段 A", "#8c998b"),
      path: "",
      kind: "video",
      duration: 6,
      generated: true,
    },
  ];
  p.nodes = [
    {
      id: "script",
      kind: "screenplay",
      title: "把家带到路上",
      text: "",
      x: 0,
      y: 0,
      screenplay: {
        script: [
          {
            id: "p1",
            title: "出发前",
            action: "小满打开餐盒，把早餐装好。",
            dialogue: "",
            sound: "餐盒轻合的声音。",
            duration: 5,
          },
          {
            id: "p2",
            title: "在路上",
            action: "车窗边，小满打开餐盒。",
            dialogue: "走得再远，也能好好吃饭。",
            sound: "轻微环境声。",
            duration: 5,
          },
        ],
      },
    },
    ...[1, 2].map((i) => ({
      id: `shot${i}`,
      kind: "shot" as const,
      title: i === 1 ? "打开餐盒" : "车窗边的午餐",
      text:
        i === 1
          ? "【画面与动作】中近景。小满坐在厨房桌前，双手打开餐盒，盒盖放在右侧。\n【摄影】固定机位，晨光从左侧窗户进入，保留餐盒结构和人物身份。"
          : "【画面与动作】车窗边的午餐。小满打开餐盒，热气轻轻升起。\n【摄影】侧面中景，窗外掠过柔和的光影。",
      x: 0,
      y: 0,
      references: [
        { assetId: "hero", purpose: "保持人物身份" },
        { assetId: "box", purpose: "保持餐盒外观" },
      ],
      shot: {
        screenplayId: "script",
        scriptId: `p${i}`,
        order: i,
        duration: 5,
        dialogue: i === 2 ? "走得再远，也能好好吃饭。" : "",
        frames: [{ assetId: `frame${i}`, title: "分镜图 · 版本 1" }],
        takes: [{ assetId: `video${i}`, trimIn: 0, trimOut: 5 }],
      },
    })),
  ];
  p.production = {
    drafts: {
      task: {
        key: "task",
        title: "打开餐盒 · 视频片段 B",
        kind: "video",
        targetNodeId: "shot1",
        mode: "multi",
        inputs: [
          {
            key: "hero",
            assetId: "hero",
            purpose: "人物身份",
            role: "reference",
          },
        ],
        modelId: "",
        prompt: "保持人物和餐盒外观，固定机位拍摄打开餐盒的动作。",
        turnId: "fixture",
        createdAt: 1,
        status: "AWAITING_CONFIRMATION",
      },
    },
  };
  return p;
}
const clock = new PlaybackClock();
function Fixture() {
  useLanguage();
  const [project, change] = useState(sample);
  const [view, setView] = useState<StudioView>("storyboard");
  const [selected, select] = useState<string | null>("shot1");
  const [focus, setFocus] = useState<{
    id?: string;
    itemKey?: string;
    tick: number;
  }>({ tick: 0 });
  const selectShot = (id: string | null) => {
    select(id);
    if (id) setFocus((f) => ({ id, tick: f.tick + 1 }));
  };
  const [selectedItems, setSelectedItems] = useState<string[]>([]);
  const [agent, setAgent] = useState(false);
  const [task, setTask] = useState<CreativeTask | null>(null);
  const [generation, setGeneration] = useState<ProductionTask | null>(null);
  useEffect(() => {
    const receive = (e: Event) => {
      setTask((e as CustomEvent<CreativeTask>).detail);
      setAgent(true);
    };
    window.addEventListener("studio-creative-request", receive);
    const showAssets = () => setView("assets");
    window.addEventListener("studio-show-assets", showAssets);
    return () => {
      window.removeEventListener("studio-creative-request", receive);
      window.removeEventListener("studio-show-assets", showAssets);
    };
  }, []);
  const canvas = {
    items: draftItems(project),
    selected: selectedItems,
    runs: runsOf(project),
    focus,
    referenced: new Set(),
    composerMode: "agent",
    setComposerMode: () => {},
    media: { models: [] },
    modelPreferences: {},
    choose: setSelectedItems,
    move: (key: string, x: number, y: number) =>
      change((p) => ({
        ...p,
        production: {
          ...p.production,
          positions: { ...p.production?.positions, [key]: { x, y } },
        },
      })),
    select: (key: string) => setSelectedItems([key]),
    focusItem: (itemKey: string) =>
      setFocus((f) => ({ ...f, itemKey, tick: f.tick + 1 })),
    reference: (keys: string[]) => {
      setTask({ text: `引用草稿：${keys.join(", ")}` });
      setAgent(true);
    },
    remove: (item: { nodeId: string }) =>
      change((p) => removeNodes(p, [item.nodeId])),
    showTask: setGeneration,
    setTaskPanelOpen: () => {},
    configure: () => {},
    attach: () => {},
  } as unknown as ProductionController;
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
        <button
          onClick={() => {
            change(sample());
            select("shot1");
          }}
        >
          重置示例
        </button>
        <button onClick={() => change(newProject("空项目"))}>空状态</button>
        <button onClick={() => setLanguage("en")}>English</button>
        <button onClick={() => setLanguage("zh-CN")}>中文</button>
        <button onClick={() => setAgent(!agent)}>切换助手侧栏</button>
      </div>
      <div
        className="studio-shell timeline-hidden"
        data-agent={agent}
        data-view={view}
        style={{ height: "calc(100vh - 42px)", position: "relative" }}
      >
        <StudioChrome
          view={view}
          onView={setView}
          canInspect={false}
          m={model}
          tasks={
            <button
              className="chrome-action"
              onClick={() => setGeneration(runsOf(project)[0] ?? null)}
            >
              <ListChecks />
              {t("生成任务")}
            </button>
          }
          onBack={() => {}}
          setError={() => {}}
          onModels={() => {}}
          onExport={() => {}}
          togglePanel={() => setAgent(!agent)}
          panels={{ preview: false, inspector: false, agent }}
          addNote={() => {}}
          onCreation={() => {}}
        />
        <StudioStage
          view={view}
          project={project}
          onChange={change}
          selected={selected}
          onSelect={selectShot}
          onReference={() => setAgent(true)}
          onAssetReference={(a) => {
            setTask({ text: `引用：${a.name}` });
            setAgent(true);
          }}
          saved="已保存"
          navigate={setView}
          clock={clock}
          canvas={canvas}
          onAdd={() => {}}
          onPlace={(asset) => {
            change((p) => ({
              ...p,
              nodes: [...p.nodes, canvasAsset(asset, p.nodes.length)],
            }));
            setView("drafts");
          }}
        />
        {agent && (
          <aside className="dock-panel dock-agent">
            <header className="dock-heading">
              <h2>Agent 工作台</h2>
            </header>
            <div
              style={{
                overflow: "auto",
                padding: 20,
                whiteSpace: "pre-wrap",
                fontSize: 13,
                lineHeight: 1.8,
              }}
            >
              <p>界面验收：下面是待发送草稿，不会请求模型。</p>
              <strong>{task?.agentId}</strong>
              <p>{task?.text ?? "选择镜头后，与对应 Agent 协作。"}</p>
            </div>
          </aside>
        )}
      </div>
      <Dialog.Root
        open={!!generation}
        onOpenChange={(v) => {
          if (!v) setGeneration(null);
        }}
      >
        <Dialog.Content aria-describedby={undefined}>
          <Dialog.Title>{t("生成任务")}</Dialog.Title>
          {generation && (
            <GenerationRun
              task={generation}
              project={project}
              canvas={canvas}
              settings={() => {}}
              follow={() => {}}
            />
          )}
        </Dialog.Content>
      </Dialog.Root>
    </Theme>
  );
}
createRoot(document.getElementById("root")!).render(<Fixture />);
