import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { Theme } from "@radix-ui/themes";
import { ComposerGeneration } from "./ComposerGeneration";
import type { ProductionController } from "./useProduction";
import { fixture, model } from "./fixtures.test-helper";

function render(
  options: {
    kind?: "image" | "video";
    selected?: boolean;
    enabled?: boolean;
    running?: boolean;
  } = {},
) {
  const configured = {
    ...model("google/gemini", options.kind ?? "video"),
    enabled: options.enabled ?? true,
  };
  const canvas = {
    composerMode: "video",
    modelPreferences: {},
    media: { models: [configured] },
    task: {
      key: "draft",
      kind: "video",
      mode: "single",
      modelId: options.selected === false ? "" : configured.id,
      inputs: [],
      parameters: {},
    },
  } as unknown as ProductionController;
  return renderToStaticMarkup(
    <Theme>
      <ComposerGeneration
        canvas={canvas}
        project={fixture()}
        running={options.running ?? false}
      />
    </Theme>,
  );
}

function parameters(html: string) {
  return html.match(/<button\b[^>]*aria-label="视频生成参数"[^>]*>/)?.[0] ?? "";
}

test("generation settings require an enabled model of the current media kind", () => {
  for (const options of [
    { selected: false },
    { kind: "image" as const },
    { enabled: false },
    { running: true },
  ]) {
    expect(parameters(render(options))).toContain('disabled=""');
  }
  expect(parameters(render())).not.toContain('disabled=""');
});

test("selected media model uses its provider icon in the composer", () => {
  expect(render()).toContain('class="provider-mark"');
  expect(render()).toContain("google.svg");
  expect(render({ selected: false })).not.toContain('class="provider-mark"');
});
