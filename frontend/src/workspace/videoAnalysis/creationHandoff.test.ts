import { expect, test } from "bun:test";
import { analysisCreationTask, analysisProject } from "./creationBrief";
import { createAnalysisLauncher } from "./creationHandoff";
import { browseWorks } from "../browseWorks";
import type { Project } from "../../model";
import type { AnalysisInvoke, AnalysisState } from "./types";
const state: AnalysisState = {
  busy: null,
  error: "",
  question: "关注声画反差",
  recordId: "record-1",
  work: browseWorks[0],
  video: {
    id: "video",
    name: "film.mp4",
    path: "/private/cache/source.mp4",
    width: 1920,
    height: 1080,
    duration: 10,
    hasAudio: true,
  },
  result: {
    summary: "参考片概述",
    access: "无法听清部分台词",
    segments: [
      {
        start: 0.125,
        end: 1.53,
        title: "开场",
        visual: "眼神特写",
        speech: "原话",
        onScreenText: "原字幕",
        sound: "突然静音",
        purpose: "制造悬念",
      },
    ],
    craft: {
      idea: "创意内核原文",
      visuals: "美术原文",
      camera: "镜头原文",
      language: "话语原文",
      sound: "音乐原文",
    },
  },
};
function storage() {
  const values = new Map<string, string>();
  return {
    values,
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => {
      values.set(key, value);
    },
    removeItem: (key: string) => {
      values.delete(key);
    },
  };
}
function projects() {
  const documents = new Map<string, Project>();
  const calls: string[] = [];
  const invoke: AnalysisInvoke = async <T>(
    command: string,
    args?: Record<string, unknown>,
  ) => {
    calls.push(command);
    if (command === "create_project") {
      const project = args!.document as Project;
      documents.set(project.id, project);
    }
    if (command === "list_projects") return [...documents.values()] as T;
    if (command === "get_project")
      return documents.get(String(args!.projectId)) as T;
    return undefined as T;
  };
  return { documents, calls, invoke };
}
test("handoff includes the entire analysis and creative intent, with no transient file paths", () => {
  const task = analysisCreationTask(state);
  expect(task.agentId).toBe("coordinator");
  expect(task.isolated).toBe(true);
  expect(task.text).toContain("做一条自己的新视频");
  expect(task.text).toContain(state.question);
  for (const value of [
    state.result!.summary,
    state.result!.access,
    ...Object.values(state.result!.craft),
    ...Object.values(state.result!.segments[0]).filter(
      (value): value is string => typeof value === "string",
    ),
  ])
    expect(task.text).toContain(value);
  expect(task.text).toContain("00:00.125 — 00:01.530");
  expect(task.text).toContain(browseWorks[0].source);
  expect(task.text).not.toContain("/private/cache");
  expect(task.refs).toEqual([]);
  expect(task.text).not.toContain(state.recordId!);
  const project = analysisProject(state);
  expect(project.name).toBe("Mora Mora · 新创作");
  expect([project.width, project.height]).toEqual([1920, 1080]);
  expect(project.nodes).toEqual([]);
  expect(project.clips).toEqual([]);
});
test("one click creates a project and seeds its composer draft before lazy mounting, without sending or generation", async () => {
  const db = projects();
  const drafts = storage();
  const service = createAnalysisLauncher(db.invoke, drafts);
  const first = service.create(state);
  const second = service.create(state);
  expect(first).toBe(second);
  const result = await first;
  expect(db.documents.size).toBe(1);
  expect(db.calls).toEqual(["create_project"]);
  expect(drafts.getItem(`mstudio-chat-draft:${result.project.id}`)).toBe(
    result.task.text,
  );
  expect(drafts.values.size).toBe(1);
});
test("one saved analysis creates independent projects without changing prior projects or drafts", async () => {
  const db = projects();
  const drafts = storage();
  const snapshot = structuredClone(state);
  const service = createAnalysisLauncher(db.invoke, drafts);
  const first = await service.create(state);
  const key = `mstudio-chat-draft:${first.project.id}`;
  drafts.setItem(key, "我已经补充了自己的商品资料");
  first.project.name = "我的第一个独立项目";
  const second = await service.create(state);
  const third = await createAnalysisLauncher(db.invoke, drafts).create(state);
  expect(
    new Set([first.project.id, second.project.id, third.project.id]).size,
  ).toBe(3);
  expect(db.documents.size).toBe(3);
  expect(db.calls).toEqual([
    "create_project",
    "create_project",
    "create_project",
  ]);
  expect(drafts.getItem(key)).toBe("我已经补充了自己的商品资料");
  expect(first.project.name).toBe("我的第一个独立项目");
  expect(second.task.text).toBe(first.task.text);
  expect(third.task.isolated).toBe(true);
  expect(drafts.values.size).toBe(3);
  expect(state).toEqual(snapshot);
});
test("a lost creation response recovers the committed project instead of creating a duplicate", async () => {
  const db = projects();
  const drafts = storage();
  const service = createAnalysisLauncher(
    async <T>(command: string, args?: Record<string, unknown>) => {
      const result = await db.invoke<T>(command, args);
      if (command === "create_project") throw new Error("response interrupted");
      return result;
    },
    drafts,
  );
  const result = await service.create(state);
  expect(result.project.id).toBe([...db.documents.keys()][0]);
  expect(db.documents.size).toBe(1);
});
test("definite create failures clean up their draft; unavailable draft storage creates nothing", async () => {
  const db = projects();
  const drafts = storage();
  drafts.setItem("another-project-draft", "Keep");
  const service = createAnalysisLauncher(
    async <T>(command: string, args?: Record<string, unknown>) => {
      if (command === "create_project") throw new Error("Disk full");
      return db.invoke<T>(command, args);
    },
    drafts,
  );
  await expect(service.create(state)).rejects.toThrow("Disk full");
  expect(drafts.values.size).toBe(1);
  const blocked = createAnalysisLauncher(db.invoke, {
    ...drafts,
    setItem: () => {
      throw new Error("Draft storage full");
    },
  });
  await expect(blocked.create(state)).rejects.toThrow("Draft storage full");
  expect(db.documents.size).toBe(0);
});
test("different analyses remain separate and creation requires a completed, idle result", async () => {
  const db = projects();
  const drafts = storage();
  const service = createAnalysisLauncher(db.invoke, drafts);
  const first = await service.create(state);
  const second = await service.create({ ...state, recordId: "record-2" });
  expect(first.project.id).not.toBe(second.project.id);
  await expect(
    service.create({ ...state, busy: "analyzing" }),
  ).rejects.toThrow();
  await expect(
    service.create({ ...state, result: undefined }),
  ).rejects.toThrow();
  const huge = analysisProject({
    ...state,
    video: { ...state.video!, width: 7681, height: 4321 },
  });
  expect(huge.width).toBe(3840);
  expect(huge.height % 2).toBe(0);
  expect(analysisProject({ ...state, video: undefined }).height).toBe(1920);
});
