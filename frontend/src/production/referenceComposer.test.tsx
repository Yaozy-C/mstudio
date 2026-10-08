import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { Theme } from "@radix-ui/themes";
import { MediaComposer } from "../assistant/MediaComposer";
import type { AttachmentDraft } from "../assistant/useAttachments";
import type { ProductionController } from "./useProduction";
import { fixture, model } from "./fixtures.test-helper";

test("reference mode renders the shared video composer without a conversation model", () => {
  const canvas = {
    composerMode: "reference",
    modelPreferences: {},
    media: { models: [model("minimax/h3/reference-to-video")] },
    task: {
      key: "draft",
      kind: "video",
      mode: "mixed",
      modelId: "model",
      prompt: "保留运镜",
      inputs: [],
    },
  } as unknown as ProductionController;
  const draft = { items: [], busy: false } as unknown as AttachmentDraft;
  const html = renderToStaticMarkup(
    <Theme>
      <MediaComposer
        canvas={canvas}
        project={fixture()}
        draft={draft}
        settings={() => {}}
      />
    </Theme>,
  );
  expect(html).toContain("参考模式");
  expect(html).toContain("请添加一段参考视频");
  expect(html).toContain("视频生成参数");
  expect(html).toContain("描述要保留的动作、运镜");
  expect(html).not.toContain("正在理解素材");
});
