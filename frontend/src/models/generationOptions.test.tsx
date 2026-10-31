import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { Theme } from "@radix-ui/themes";
import { MediaModelForm } from "./MediaModelForm";
import { modelLibrary } from "./catalogSpecs";
import { catalogMediaModel } from "./catalogMediaModel";
import {
  generationOptions,
  generationOptionDefaults,
  updateGenerationOption,
} from "./generationOptions";
import { GenerationSettings } from "../production/GenerationSettings";
import { parameterContract, taskParameters } from "../production/parameters";
import { fixture } from "../production/fixtures.test-helper";

const image = {
  ...catalogMediaModel(
    modelLibrary.find(
      (s) => s.endpoint === "openai/gpt-image-2.5/sunburst/text-to-image",
    )!,
    "image",
  ),
  id: "image",
  connectionId: "fixture",
  params: { num_images: 1, quality: "auto" },
};
const video = {
  ...catalogMediaModel(
    modelLibrary.find((s) => s.id === "wan-3.0-text-to-video")!,
    "video",
  ),
  id: "video",
  connectionId: "fixture",
};

test("generic task options preserve types, request paths and model bounds", () => {
  expect(generationOptionDefaults(image)).toMatchObject({
    num_images: 1,
    quality: "auto",
  });
  const selected = updateGenerationOption({}, "num_images", 2);
  expect(taskParameters(image, { options: selected })).toMatchObject({
    num_images: 2,
  });
  expect(updateGenerationOption(selected, "num_images", undefined)).toEqual({});
  expect(() =>
    taskParameters(image, { options: { num_images: 20 } }),
  ).toThrow();
  expect(() =>
    taskParameters(image, { options: { num_images: "2" } }),
  ).toThrow();
  expect(() => taskParameters(image, { options: { unknown: true } })).toThrow();
  expect(parameterContract(image).parameters.properties).toMatchObject({
    options: {
      properties: { num_images: { type: "integer", minimum: 1, maximum: 4 } },
    },
  });
  expect(generationOptions(video).find((o) => o.key === "audio")?.type).toBe(
    "boolean",
  );
  expect(taskParameters(video, { options: { audio: false } })).toMatchObject({
    parameters: { audio: false },
  });
  expect(() =>
    taskParameters(video, { options: { audio: "false" } }),
  ).toThrow();
});

test("task options remain visible in generation settings", () => {
  const html = renderToStaticMarkup(
    <Theme>
      <GenerationSettings
        task={{
          key: "task",
          kind: "image",
          mode: "multi",
          modelId: image.id,
          prompt: "A cube",
          inputs: [],
          parameters: { options: { num_images: 2 } },
        }}
        project={fixture()}
        models={[image]}
        close={() => {}}
        save={() => {}}
        onPatch={() => {}}
      />
    </Theme>,
  );
  expect(html).toContain("生成数量");
  expect(html).toContain("生成质量");
});

test("all addable media presets keep generation values out of connection settings", () => {
  for (const spec of modelLibrary.filter(
    (s) => s.endpoint && s.kind !== "text",
  )) {
    const media = {
      ...catalogMediaModel(spec, spec.kind === "image" ? "image" : "video"),
      connectionId: "fixture",
    };
    if (media.plugin === "codex-image") continue;
    const html = renderToStaticMarkup(
      <Theme>
        <MediaModelForm
          initial={media}
          save={async () => {}}
          cancel={() => {}}
        />
      </Theme>,
    );
    expect(html).not.toContain("默认生成参数");
    expect(html).not.toContain("请求 JSON 模板");
    for (const option of generationOptions(media)) {
      expect(parameterContract(media).parameters.properties).toHaveProperty(
        "options",
      );
      expect(option.path.startsWith("/")).toBe(true);
    }
  }
});
