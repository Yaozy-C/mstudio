import { afterEach, expect, test } from "bun:test";
import { renderToStaticMarkup as render } from "react-dom/server";
import { setLanguage } from "../i18n";
import {
  useAttachments,
  type AttachmentDraft,
} from "../assistant/useAttachments";
import { videoAnalysisTask, type CreativeTask } from "../creative/aiTasks";
import { CreativeHomeStart } from "./CreativeHomeStart";

afterEach(() => setLanguage("zh-CN"));

test("home offers two creation paths and an honest inspiration empty state in both languages", () => {
  for (const language of ["zh-CN", "en"] as const) {
    setLanguage(language);
    const html = render(
      <CreativeHomeStart onReference={() => {}} onCreate={() => {}} />,
    );
    expect(html.match(/<h1/g)).toHaveLength(1);
    expect(html.match(/<button/g)).toHaveLength(3);
    expect(html).not.toMatch(/<(img|video|iframe)\b/);
    expect(html).toContain(
      language === "en" ? "Explore a Reference" : "探索参考作品",
    );
    expect(html).toContain(
      language === "en" ? "Create Something New" : "创作全新作品",
    );
    expect(html).toContain(
      language === "en" ? "No featured videos yet" : "暂无精选视频",
    );
    if (language === "en") expect(html).not.toMatch(/[\u3400-\u9fff]/);
  }
});

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
