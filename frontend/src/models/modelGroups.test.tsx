import { expect, test } from "bun:test";
import { Theme } from "@radix-ui/themes";
import { renderToStaticMarkup } from "react-dom/server";
import { groupMediaModels, modelInGroup } from "./modelGroups";
import { GenerationSettings } from "../production/GenerationSettings";
import { selectMediaModel } from "../production/frameInputs";
import { effectiveParameters } from "../production/parameters";
import { fixture } from "../production/fixtures.test-helper";
import type { MediaModel } from "./mediaRegistry";
import type { ProductionTask } from "../production/types";
import { modelLibrary } from "./catalogSpecs";
import { catalogMediaModel } from "./catalogMediaModel";
import { MediaModels } from "./MediaModels";
import { ModelLibrary } from "./ModelLibrary";

const multiModeModels: MediaModel[] = [
  "h3-image-to-video",
  "h3-reference-to-video",
  "wan-3.0-image-to-video",
  "wan-3.0-reference-to-video",
].map((id) => {
  const spec = modelLibrary.find((s) => s.id === id)!;
  return {
    ...catalogMediaModel(spec, "video"),
    id: spec.id,
    name: spec.name,
    connectionId: "account",
  };
});

test("catalog model families group their modes without merging distinct models or accounts", () => {
  const groups = groupMediaModels(multiModeModels);
  expect(groups.map((g) => g.name)).toEqual(["MiniMax H3", "Wan 3.0"]);
  expect(groups[0].variants).toHaveLength(2);
  expect(modelInGroup(groups[1], multiModeModels[1]).id).toBe(
    "wan-3.0-reference-to-video",
  );
  expect(
    groupMediaModels([
      ...multiModeModels,
      { ...multiModeModels[0], id: "account-2", connectionId: "other" },
    ]),
  ).toHaveLength(3);
});

test("switching model or mode preserves Agent parameters and shows the effective duration", () => {
  const task: ProductionTask = {
    key: "video",
    kind: "video",
    mode: "multi",
    modelId: multiModeModels[1].id,
    prompt: "Move",
    inputs: [],
    parameters: { duration: 8, resolution: "2K" },
  };
  const patch = selectMediaModel(task, multiModeModels[0]);
  expect(patch.parameters).toEqual(task.parameters);
  expect(patch.parameters).not.toBe(task.parameters);
  expect(
    effectiveParameters(multiModeModels[0], patch.parameters).duration,
  ).toBe(8);
  expect(effectiveParameters(multiModeModels[0]).duration).toBe(5);
  const html = renderToStaticMarkup(
    <Theme>
      <GenerationSettings
        task={task}
        project={fixture()}
        models={multiModeModels}
        close={() => {}}
        save={() => {}}
        onPatch={() => {}}
      />
    </Theme>,
  );
  expect(html).toContain('aria-label="视频时长"');
  expect(html).toContain('value="8"');
  expect(html).not.toContain("沿用模型设置");
  expect(html).not.toMatch(/<select(?![^>]*aria-hidden="true")/);
  expect(html).toContain("studio-select-trigger");
  expect(html).toContain("全参考");
});

test("H3 and Wan appear once with three modes, including existing workspace endpoints", () => {
  const variants = modelLibrary
    .filter((spec) => spec.family)
    .map((spec) => ({
      ...catalogMediaModel(spec, "video"),
      connectionId: "account",
      name: spec.name,
      ...(spec.plugin === "dashscope"
        ? {
            endpoint:
              "https://my-space.cn-beijing.maas.aliyuncs.com/api/v1/services/aigc/video-generation/video-synthesis",
          }
        : {}),
    }));
  const groups = groupMediaModels(variants);
  expect(groups.map((g) => g.name)).toEqual(["Wan 3.0", "MiniMax H3"]);
  expect(groups.map((g) => g.variants.length)).toEqual([3, 3]);
  const html = renderToStaticMarkup(
    <Theme>
      <MediaModels
        kind="video"
        hub={{
          models: variants,
          loading: false,
          error: "",
          refresh: () => true,
          save: async () => {},
          remove: async () => {},
        }}
      />
    </Theme>,
  );
  expect(html.match(/class="hub-model-card"/g)).toHaveLength(2);
  const library = renderToStaticMarkup(
    <Theme>
      <ModelLibrary
        kind="video"
        configure={() => {}}
        cancel={() => {}}
        custom={() => {}}
      />
    </Theme>,
  );
  expect(library.match(/<h3>MiniMax H3<\/h3>/g)).toHaveLength(1);
  expect(library.match(/<h3>Wan 3.0<\/h3>/g)).toHaveLength(1);
});
