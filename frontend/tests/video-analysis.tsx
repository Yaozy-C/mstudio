// Synthetic responses only. No model calls, external uploads or project writes.
import ReactDOM from "react-dom/client";
import { Theme } from "@radix-ui/themes";
import { useState, useSyncExternalStore } from "react";
import "@radix-ui/themes/styles.css";
import "../src/styles/color-tokens.css";
import "../src/styles/base.css";
import "../src/styles/frame-theme.css";
import "../src/styles/workspace.css";
import "../src/workspace/workspace-home.css";
import { StudioSidebar } from "../src/workspace/StudioSidebar";
import "../src/styles/visual-system.css";
import { bridge } from "../src/bridge";
import { Studio } from "../src/workspace/Studio";
import {
  createAnalysisLauncher,
  type CreationHandoff,
} from "../src/workspace/videoAnalysis/creationHandoff";
import "./studio-fixture.css";
import { setLanguage } from "../src/i18n";
import { VideoAnalysis } from "../src/workspace/videoAnalysis/VideoAnalysis";
import { createAnalysisController } from "../src/workspace/videoAnalysis/controller";
import { browseWorks } from "../src/workspace/browseWorks";
import type { ProgressUpdate } from "../src/workspace/videoAnalysis/types";
import type { AnalysisRecord } from "../src/workspace/videoAnalysis/types";
import type { VideoAnalysisResult } from "../src/workspace/videoAnalysis/types";

setLanguage("zh-CN");
const fixture: VideoAnalysisResult = {
  summary: "合成样片交互测试：以下内容是测试响应，不是真实作品分析。",
  access: "仅用于验证时间跳转和视图切换。",
  segments: [
    {
      start: 0,
      end: 1,
      title: "开场测试",
      visual: "测试图案",
      speech: "",
      onScreenText: "",
      sound: "合成音轨",
      purpose: "验证第一段",
    },
    {
      start: 1,
      end: 3,
      title: "第二段测试",
      visual: "测试图案继续",
      speech: "",
      onScreenText: "",
      sound: "合成音轨继续",
      purpose: "验证跳转与高亮",
    },
  ],
  craft: {
    idea: "测试内核",
    visuals: "测试画面",
    camera: "测试镜头",
    language: "测试文字",
    sound: "测试声音",
  },
};
const fixtureRecords: AnalysisRecord[] = JSON.parse(
  localStorage.getItem("mstudio-analysis-history-fixture") ?? "[]",
);
let progressMode: "downloading" | "analyzing" | null = null;
let finishPreview: (() => void) | undefined;
const controller = createAnalysisController(
  async <T,>(command: string, args?: Record<string, unknown>) => {
    if (command === "cancel_video_analysis") finishPreview?.();
    if (
      (command === "prepare_video_analysis" &&
        progressMode === "downloading") ||
      (command === "analyze_reference_video" && progressMode === "analyzing")
    ) {
      const receive = args!.onProgress as (progress: ProgressUpdate) => void;
      receive(
        progressMode === "downloading"
          ? {
              stage: "downloading",
              completed: 18 * 1024 ** 2,
              total: 48 * 1024 ** 2,
            }
          : { stage: "analyzing" },
      );
      await new Promise<void>((resolve) => {
        finishPreview = resolve;
      });
      progressMode = null;
    }
    if (command === "prepare_video_analysis")
      return {
        id: "fixture",
        name: "合成测试视频",
        path: "/tests/video-analysis-fixture.mp4",
        duration: 3,
        width: 320,
        height: 240,
        hasAudio: true,
      } as T;
    if (command === "analyze_reference_video") {
      const record: AnalysisRecord = {
        id: crypto.randomUUID(),
        createdAt: Date.now(),
        context: { title: "合成测试视频", modelName: "合成测试模型" },
        video: {
          id: "fixture",
          name: "合成测试视频",
          path: "/tests/video-analysis-fixture.mp4",
          duration: 3,
          width: 320,
          height: 240,
          hasAudio: true,
        },
        result: fixture,
        question: String(args?.question || "测试关注点"),
        modelId: "fixture",
        language: "zh-CN",
      };
      fixtureRecords.unshift(record);
      localStorage.setItem(
        "mstudio-analysis-history-fixture",
        JSON.stringify(fixtureRecords),
      );
      return { result: fixture, recordId: record.id } as T;
    }
    if (command === "list_video_analysis_records") {
      const records = fixtureRecords.filter((record) =>
        `${record.context.title}${record.question}${record.result.summary}`.includes(
          String(args?.query ?? ""),
        ),
      );
      return {
        items: records
          .slice(Number(args?.offset ?? 0), Number(args?.offset ?? 0) + 40)
          .map((record) => ({
            id: record.id,
            title: record.context.title,
            createdAt: record.createdAt,
            duration: record.video.duration,
            modelName: record.context.modelName,
            question: record.question,
            summary: record.result.summary,
            missingVideo: false,
          })),
        total: records.length,
      } as T;
    }
    if (command === "open_video_analysis_record") {
      const record = fixtureRecords.find((record) => record.id === args?.id)!;
      return {
        record,
        video: { ...record.video, id: crypto.randomUUID() },
      } as T;
    }
    return undefined as T;
  },
  (receive) => receive,
);
const launcher = createAnalysisLauncher(bridge, localStorage);
const testProjects = new Set<string>();
function Test() {
  const [handoff, setHandoff] = useState<CreationHandoff | null>(null);
  const state = useSyncExternalStore(
    controller.subscribe,
    controller.getSnapshot,
  );
  if (handoff)
    return (
      <Theme appearance="light" accentColor="tomato" radius="small">
        <div style={{ padding: 12 }}>
          <strong>创作链路回归 · 合成数据 · 不调用模型</strong>
        </div>
        <div className="creation-handoff-fixture">
          <Studio
            initial={handoff.project}
            initialTask={handoff.task}
            onBack={() => setHandoff(null)}
          />
        </div>
      </Theme>
    );
  return (
    <Theme appearance="light" accentColor="tomato" radius="small">
      <div style={{ padding: 12 }}>
        <strong>交互回归 · 合成数据</strong>{" "}
        <button
          onClick={async () => {
            for (const id of testProjects) {
              await bridge("delete_project", { id });
              localStorage.removeItem(`mstudio-chat-draft:${id}`);
            }
            testProjects.clear();
          }}
        >
          清理测试项目
        </button>
        <button
          onClick={async () => {
            await controller.pickFile();
            await controller.analyze("fixture", "", "zh-CN");
          }}
        >
          装入测试响应
        </button>
        <button
          onClick={() => {
            controller.clear();
            controller.selectWork(browseWorks[0]);
            progressMode = "downloading";
            void controller.analyze("fixture", "", "zh-CN");
          }}
        >
          下载进度样例
        </button>
        <button
          onClick={async () => {
            controller.clear();
            progressMode = null;
            await controller.pickFile();
            progressMode = "analyzing";
            void controller.analyze("fixture", "", "zh-CN");
          }}
        >
          分析进度样例
        </button>
      </div>
      <div className="library-shell workspace-home-shell">
        <StudioSidebar
          activePage="video-analysis"
          onBrowse={() => controller.clear()}
          onProjects={() => {}}
          onVideoAnalysis={() => controller.clear()}
          onSettings={() => {}}
        />
        <VideoAnalysis
          state={state}
          controller={controller}
          onCreate={async () => {
            const next = await launcher.create(controller.getSnapshot());
            testProjects.add(next.project.id);
            setHandoff(next);
          }}
          onBrowse={() => {}}
          onSettings={() => {}}
        />
      </div>
    </Theme>
  );
}
ReactDOM.createRoot(document.getElementById("root")!).render(<Test />);
