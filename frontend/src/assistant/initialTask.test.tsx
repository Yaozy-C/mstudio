import { afterEach, expect, test } from "bun:test";
import { renderToStaticMarkup as render } from "react-dom/server";
import { setLanguage } from "../i18n";
import { useAttachments, type AttachmentDraft } from "./useAttachments";
import { videoAnalysisTask, type CreativeTask } from "../creative/aiTasks";

afterEach(() => setLanguage("zh-CN"));

function initialDraft(task?: CreativeTask) {
  let draft: AttachmentDraft | undefined;
  function Probe() {
    draft = useAttachments("new-project", () => {}, task);
    return null;
  }
  render(<Probe />);
  return draft!;
}

test("reference handoff is available before the lazy composer mounts, with no automatic submission", () => {
  const task = videoAnalysisTask();
  const draft = initialDraft(task);
  expect(draft.suggestion).toBe(task);
  expect(draft.suggestion?.agentId).toBe("video-analyst");
  expect(draft.omitWork).toBe(true);
  expect(draft.items).toEqual([]);
  expect(draft.focus).toBe(1);
  expect(draft.busy).toBe(false);
});

test("normal creation and reopening start without a reference task or stale attachments", () => {
  const draft = initialDraft();
  expect(draft.suggestion).toBeNull();
  expect(draft.omitWork).toBe(false);
  expect(draft.items).toEqual([]);
  expect(draft.targetNodeId).toBeNull();
  expect(draft.focus).toBe(0);
});
