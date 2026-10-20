import { expect, test } from "bun:test";
import { createAnalysisController } from "./controller";
import {
  timecode,
  videoLink,
  workVideoLink,
  type AnalysisVideo,
} from "./types";
import { browseWorks } from "../browseWorks";

const video: AnalysisVideo = {
  id: "video",
  name: "Original",
  path: "/cache/original.mp4",
  duration: 5,
  width: 320,
  height: 240,
  hasAudio: true,
};
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

test("work handoff does not create a project, download or invoke a model before Analyze", async () => {
  const calls: string[] = [];
  const controller = createAnalysisController(async <T>(command: string) => {
    calls.push(command);
    return undefined as T;
  });
  controller.selectWork(browseWorks[0]);
  expect(controller.getSnapshot().work?.id).toBe(browseWorks[0].id);
  expect(calls).toEqual([]);
});

test("analyzing a work prepares original media and passes a standalone model request", async () => {
  const calls: { command: string; args?: Record<string, unknown> }[] = [];
  const result = { summary: "Observed" };
  const controller = createAnalysisController(
    async <T>(command: string, args?: Record<string, unknown>) => {
      calls.push({ command, args });
      return (
        command === "prepare_video_analysis"
          ? video
          : { result, recordId: "saved" }
      ) as T;
    },
  );
  controller.selectWork(browseWorks[0]);
  await controller.analyze("video-model", "Explain music", "en");
  expect(calls.map((call) => call.command)).toEqual([
    "prepare_video_analysis",
    "analyze_reference_video",
  ]);
  expect(calls[1].args).toMatchObject({
    modelId: "video-model",
    question: "Explain music",
    language: "en",
  });
  expect(calls[1].args).not.toHaveProperty("projectId");
  expect(controller.getSnapshot().result?.summary).toBe(result.summary);
  expect(controller.getSnapshot().busy).toBeNull();
});

test("a cancelled import cannot replace a newly selected work", async () => {
  const pending = deferred<AnalysisVideo>();
  const calls: string[] = [];
  const controller = createAnalysisController(async <T>(command: string) => {
    calls.push(command);
    return (
      command === "prepare_video_analysis" ? await pending.promise : undefined
    ) as T;
  });
  const loading = controller.loadUrl("https://example.com/video.mp4");
  controller.selectWork(browseWorks[1]);
  pending.resolve(video);
  await loading;
  expect(controller.getSnapshot().work?.id).toBe(browseWorks[1].id);
  expect(controller.getSnapshot().video).toBeUndefined();
  expect(calls).toContain("cancel_video_analysis");
  expect(calls).toContain("release_video_analysis");
});

test("stopping analysis ignores late results and retains the prepared video for retry", async () => {
  const pending = deferred<unknown>();
  const controller = createAnalysisController(
    async <T>(command: string) =>
      (command === "prepare_video_analysis"
        ? video
        : command === "analyze_reference_video"
          ? await pending.promise
          : undefined) as T,
  );
  await controller.pickFile();
  const running = controller.analyze("model", "", "zh-CN");
  controller.cancel();
  pending.resolve({ summary: "Stale result" });
  await running;
  expect(controller.getSnapshot().result).toBeUndefined();
  expect(controller.getSnapshot().video).toEqual(video);
  expect(controller.getSnapshot().busy).toBeNull();
});

test("cancelled file picker and failed downloads preserve the current reference", async () => {
  const controller = createAnalysisController(async <T>() => null as T);
  controller.selectWork(browseWorks[0]);
  await controller.pickFile();
  expect(controller.getSnapshot().work).toBe(browseWorks[0]);
  await controller.loadUrl("file:///private/file.mp4");
  expect(controller.getSnapshot().error).toContain("HTTP");
  expect(controller.getSnapshot().work).toBe(browseWorks[0]);
});

test("source helpers accept only web links and derive individual Mux streams", () => {
  expect(timecode(65.5)).toBe("01:05");
  expect(videoLink(" https://example.com/watch ")).toBe(
    "https://example.com/watch",
  );
  expect(() => videoLink("https://user:secret@example.com")).toThrow();
  expect(() => videoLink("javascript:alert(1)")).toThrow();
  expect(workVideoLink(browseWorks[0])).toStartWith("https://stream.mux.com/");
});

test("browser preview metadata cannot update a replaced source and clearing revokes the video", async () => {
  const controller = createAnalysisController(async <T>() => undefined as T);
  controller.previewFile(
    new File(["video fixture"], "preview.mp4", { type: "video/mp4" }),
  );
  const preview = controller.getSnapshot().video!;
  expect(preview.path).toStartWith("blob:");
  controller.previewMetadata(preview.id, 3, 960, 540);
  expect(controller.getSnapshot().video).toMatchObject({
    duration: 3,
    width: 960,
  });
  controller.previewMetadata("old", 9, 10, 10);
  expect(controller.getSnapshot().video?.duration).toBe(3);
  controller.clear();
  expect(controller.getSnapshot().video).toBeUndefined();
  await expect(fetch(preview.path)).rejects.toThrow();
});

test("progress channels are scoped to each command and ignore late events after stop or retry", async () => {
  const listeners: ((value: import("./types").ProgressUpdate) => void)[] = [];
  const downloads = [deferred<AnalysisVideo>(), deferred<AnalysisVideo>()];
  let download = 0;
  const analysis = deferred<unknown>();
  const controller = createAnalysisController(
    async <T>(command: string) =>
      (command === "prepare_video_analysis"
        ? await downloads[download++].promise
        : command === "analyze_reference_video"
          ? await analysis.promise
          : undefined) as T,
    (receive) => {
      listeners.push(receive);
      return { channel: listeners.length };
    },
  );
  controller.selectWork(browseWorks[0]);
  const running = controller.analyze("model", "", "zh-CN");
  const startedAt = controller.getSnapshot().progress!.startedAt;
  listeners[0]({ stage: "downloading", completed: 40, total: 100 });
  expect(controller.getSnapshot().progress).toMatchObject({
    completed: 40,
    startedAt,
  });
  downloads[0].resolve(video);
  await Promise.resolve();
  await Promise.resolve();
  await Promise.resolve();
  expect(listeners).toHaveLength(2);
  listeners[1]({ stage: "analyzing" });
  listeners[0]({ stage: "checking" });
  expect(controller.getSnapshot().progress?.stage).toBe("analyzing");
  controller.cancel();
  listeners[1]({ stage: "formatting" });
  expect(controller.getSnapshot().progress).toBeUndefined();
  controller.clear();
  const retry = controller.loadUrl("https://example.com/video.mp4");
  listeners[1]({ stage: "formatting" });
  expect(controller.getSnapshot().progress?.stage).toBe("connecting");
  downloads[1].resolve(video);
  analysis.resolve({ summary: "Stale" });
  await Promise.all([retry, running]);
  expect(controller.getSnapshot().result).toBeUndefined();
  expect(controller.getSnapshot().progress).toBeUndefined();
});
