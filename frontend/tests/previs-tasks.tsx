import { useState } from "react";
import { createRoot } from "react-dom/client";
import { Theme } from "@radix-ui/themes";
import { newProject } from "../src/model";
import { newConnection } from "../src/models/types";
import { newShot, type PrevisWorkspace } from "../src/previs/types";
import { setLanguage } from "../src/i18n";
import "@radix-ui/themes/styles.css";
import "../src/styles/color-tokens.css";
import "../src/styles/motion.css";
import "../src/styles/base.css";
import "../src/styles/dialogs.css";
import "../src/styles/frame-theme.css";
import "../src/styles/visual-system.css";

// Synthetic host: hold responses to inspect real session/dialog lifecycles without paid generation.
const project = newProject("预演后台任务验收");
project.nodes = [1, 2].map((i) => ({
  id: `shot-${i}`,
  kind: "shot",
  title: `Shot 0${i} · 打开午餐包`,
  text: "固定机位，打开午餐包，再把餐盒放到桌面。",
  x: 0,
  y: 0,
  shot: { screenplayId: "script", order: i, duration: 2.7, dialogue: "" },
}));
const workspace: PrevisWorkspace = {
  files: project.nodes.map((n) => ({ ...newShot(n, project), revision: 1 })),
  messages: [],
  exports: [],
};
const model = {
  ...newConnection(),
  id: "test-model",
  name: "Demo · Scene model",
  adapter: "codex" as const,
};
let notify = () => {};
const calls: Record<string, number> = {};
const pending = new Map<
  string,
  {
    request: Record<string, string>;
    resolve: (value: unknown) => void;
    reject: (error: unknown) => void;
  }
>();
const sessions = new Map<string, string>();
const renders = new Map<string, string>();
let serial = 0;
let frameDelay = 200;
Object.assign(window, {
  isTauri: true,
  __TAURI_INTERNALS__: {
    invoke: async (command: string, args: any) => {
      calls[command] = (calls[command] || 0) + 1;
      notify();
      if (command === "previs_workspace") return structuredClone(workspace);
      if (command === "model_catalog")
        return { profiles: [model], defaultId: model.id, selectedId: model.id };
      if (command === "save_previs_file") {
        const file = { ...args.file, revision: args.file.revision + 1 };
        workspace.files = workspace.files.map((f) =>
          f.id === file.id ? file : f,
        );
        return file;
      }
      if (command === "previs_chat")
        return new Promise((resolve, reject) => {
          pending.set(args.request.requestId, {
            request: args.request,
            resolve,
            reject,
          });
          notify();
        });
      if (command === "cancel_previs_chat") {
        pending.get(args.requestId)?.reject("预演 Chat 已停止，文件未修改");
        pending.delete(args.requestId);
        return;
      }
      if (command === "open_previs_runtime") {
        const file = workspace.files.find((f) => f.id === args.fileId)!;
        const sessionId = `session-${++serial}`;
        sessions.set(sessionId, file.id);
        return {
          sessionId,
          url: `${location.origin}/tests/previs-runtime.html?session=${sessionId}`,
          signature: sessionId,
          fileRevision: file.revision,
        };
      }
      if (command === "begin_previs_render") {
        const id = `render-${++serial}`;
        renders.set(id, sessions.get(args.sessionId)!);
        return { id, frameCount: 24 };
      }
      if (command === "append_previs_frame")
        return new Promise((r) => setTimeout(r, frameDelay));
      if (command === "finish_previs_render") {
        const file = workspace.files.find(
          (f) => f.id === renders.get(args.id),
        )!;
        workspace.exports.push({
          id: args.id,
          fileId: file.id,
          fileRevision: file.revision,
          modelRevisions: {},
          signature: "demo",
          sharedAssetId: null,
          asset: {
            id: "video",
            name: "Synthetic previs",
            path: "",
            preview: "",
            kind: "video",
            duration: 2.7,
            width: 720,
            height: 1280,
            hasAudio: false,
          },
        });
        return workspace.exports.at(-1);
      }
      if (command === "share_previs_video") {
        const video = workspace.exports.find((item) => item.id === args.id)!;
        const asset = project.assets.find(
          (item) => item.id === video.sharedAssetId,
        ) ?? { ...video.asset, id: `shared-${video.id}`, inLibrary: true };
        if (!project.assets.some((item) => item.id === asset.id))
          project.assets.push(asset);
        video.sharedAssetId = asset.id;
        return { shared: true, assetId: asset.id, asset };
      }
      if (
        command === "close_previs_runtime" ||
        command === "cancel_previs_render"
      )
        return;
      throw new Error(`Unexpected test command: ${command}`);
    },
  },
});
const { PrevisProvider } = await import("../src/previs/PrevisProvider");
const { openPrevis } = await import("../src/previs/context");
const { TaskPanel } = await import("../src/production/TaskPanel");
import type { ProductionController } from "../src/production/useProduction";
function Tasks() {
  const [taskPanelOpen, setTaskPanelOpen] = useState(false);
  return (
    <TaskPanel
      project={project}
      canvas={
        {
          runs: [],
          taskPanelOpen,
          setTaskPanelOpen,
          clearActiveTask: () => {},
        } as unknown as ProductionController
      }
      settings={() => {}}
    />
  );
}
function settle(failed = false) {
  for (const [id, task] of pending) {
    if (failed) task.reject("Synthetic network failure");
    else {
      const file = workspace.files.find((f) => f.id === task.request.fileId)!;
      file.code = "export async function createShot() {}";
      file.revision++;
      workspace.messages.push(
        { fileId: file.id, role: "user", text: task.request.prompt },
        { fileId: file.id, role: "assistant", text: "已按要求更新预演。" },
      );
      task.resolve({
        file: structuredClone(file),
        message: "已按要求更新预演。",
      });
    }
    pending.delete(id);
  }
  notify();
}
function App() {
  const [, update] = useState(0);
  notify = () => update((n) => n + 1);
  return (
    <Theme>
      <PrevisProvider
        project={project}
        flush={async () => {
          calls.fixture_save = (calls.fixture_save || 0) + 1;
        }}
        change={(fn) => {
          Object.assign(project, fn(project));
          update((n) => n + 1);
        }}
      >
        <main style={{ padding: 32 }}>
          <h1>预演后台任务验收</h1>
          <Tasks />
          <button onClick={() => openPrevis("shot-1")}>打开镜头 1</button>
          <button onClick={() => openPrevis("shot-2")}>打开镜头 2</button>
          <button onClick={() => settle()}>完成模型任务</button>
          <button onClick={() => settle(true)}>模型任务失败</button>
          <button onClick={() => setLanguage("en")}>English</button>
          <button onClick={() => setLanguage("zh-CN")}>中文</button>
          <button
            onClick={() => {
              frameDelay = 1000;
            }}
          >
            慢速渲染
          </button>
          <output style={{ display: "block", margin: "24px 0" }}>
            {JSON.stringify({
              pending: pending.size,
              revisions: workspace.files.map((f) => f.revision),
              exports: workspace.exports.length,
              references: project.nodes.map((node) => node.references),
              calls,
            })}
          </output>
          <p>关闭后点击完成模型任务；重新打开检查结果。所有数据均为合成。</p>
          <textarea
            aria-label="工作区输入"
            placeholder="生成中也可以继续输入"
            style={{ width: "100%", minHeight: 120 }}
          />
        </main>
      </PrevisProvider>
    </Theme>
  );
}
createRoot(document.getElementById("root")!).render(<App />);
