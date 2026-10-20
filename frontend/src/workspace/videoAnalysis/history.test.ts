import { expect, test } from "bun:test";
import { createAnalysisController } from "./controller";
import { browseWorks } from "../browseWorks";
import type { OpenedRecord, AnalysisVideo } from "./types";
const video: AnalysisVideo = {
  id: "working-session",
  name: "Original.mp4",
  path: "/history/source.mp4",
  duration: 3,
  width: 320,
  height: 180,
  hasAudio: true,
};
const opened: OpenedRecord = {
  video,
  record: {
    id: "saved-record",
    createdAt: 10,
    context: {
      title: "Original",
      modelName: "Saved model",
      workId: "mora-mora",
      sourceUrl: "https://example.com/film",
    },
    video: { ...video, path: "source.mp4" },
    result: {
      summary: "Saved observation",
      access: "Full video",
      segments: [],
      craft: { idea: "Idea", visuals: "", camera: "", language: "", sound: "" },
    },
    question: "Saved focus",
    modelId: "saved-model",
    language: "zh-CN",
  },
};

test("opening history restores a fresh video session, saved result and focus without download or model calls", async () => {
  const calls: string[] = [];
  const controller = createAnalysisController(async <T>(command: string) => {
    calls.push(command);
    return opened as T;
  });
  controller.selectWork(browseWorks[1]);
  expect(await controller.openRecord("saved-record")).toBe(true);
  expect(calls).toEqual(["open_video_analysis_record"]);
  expect(controller.getSnapshot()).toMatchObject({
    recordId: "saved-record",
    restoredModelId: "saved-model",
    question: "Saved focus",
    video,
    result: opened.record.result,
    work: browseWorks[0],
    openingRecord: false,
  });
});
test("a missing original does not hide its saved observations or allow accidental re-analysis", async () => {
  const calls: string[] = [];
  const controller = createAnalysisController(async <T>(command: string) => {
    calls.push(command);
    return { ...opened, video: null } as T;
  });
  await controller.openRecord("saved-record");
  expect(controller.getSnapshot().video?.missing).toBe(true);
  expect(controller.getSnapshot().result?.summary).toBe("Saved observation");
  await controller.analyze("model", "", "zh-CN");
  expect(calls).toEqual(["open_video_analysis_record"]);
});
test("late record loads cannot replace a new reference and their working media is released", async () => {
  let resolve!: (value: OpenedRecord) => void;
  const pending = new Promise<OpenedRecord>((done) => {
    resolve = done;
  });
  const calls: string[] = [];
  const controller = createAnalysisController(async <T>(command: string) => {
    calls.push(command);
    return (
      command === "open_video_analysis_record" ? await pending : undefined
    ) as T;
  });
  const loading = controller.openRecord("saved-record");
  controller.selectWork(browseWorks[1]);
  resolve(opened);
  expect(await loading).toBe(false);
  expect(controller.getSnapshot().work).toBe(browseWorks[1]);
  expect(calls).toContain("release_video_analysis");
});
test("saving failures keep the completed result visible and retries never mutate the old snapshot", async () => {
  let analyses = 0;
  const controller = createAnalysisController(
    async <T>(command: string) =>
      (command === "prepare_video_analysis"
        ? video
        : command === "analyze_reference_video"
          ? ++analyses === 1
            ? { result: opened.record.result, recordId: "first" }
            : {
                result: { ...opened.record.result, summary: "New observation" },
                saveError: "Disk full",
              }
          : undefined) as T,
  );
  await controller.pickFile();
  await controller.analyze(
    "chosen-model",
    "Original focus",
    "en",
    "Chosen model",
  );
  expect(controller.getSnapshot().recordId).toBe("first");
  expect(controller.getSnapshot().restoredModelId).toBe("chosen-model");
  const previous = controller.getSnapshot().result;
  await controller.analyze("chosen-model", "New focus", "en", "Chosen model");
  expect(controller.getSnapshot().recordError).toBe("Disk full");
  expect(controller.getSnapshot().result?.summary).toBe("New observation");
  expect(previous?.summary).toBe("Saved observation");
  expect(controller.getSnapshot().recordId).toBeUndefined();
});
