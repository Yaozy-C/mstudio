import { useState } from "react";
import { createRoot } from "react-dom/client";
import { Theme } from "@radix-ui/themes";
import type { MediaModel } from "../src/models/mediaRegistry";
import type { ProductionTask } from "../src/production/types";
import type { Asset } from "../src/model";
import "@radix-ui/themes/styles.css";
import "./studio-fixture.css";
import "../src/styles/model-center.css";
import "../src/styles/model-cards.css";
import "../src/styles/model-hub.css";

// Synthetic data and an isolated host: no account, project or generation calls.
Object.assign(window, {
  isTauri: true,
  __TAURI_INTERNALS__: {
    invoke: async (command: string) => {
      if (command === "image_processing_runtime") return null;
      if (command === "service_connections")
        return [
          {
            id: "fixture-fal",
            name: "演示服务",
            kind: "fal",
            endpoint: "https://queue.fal.run",
            hasKey: true,
            modelCount: 3,
          },
          {
            id: "fixture-dashscope",
            name: "演示百炼服务",
            kind: "dashscope",
            endpoint: "https://example.cn-beijing.maas.aliyuncs.com",
            hasKey: true,
            modelCount: 3,
          },
        ];
      throw new Error(`Unexpected fixture command: ${command}`);
    },
  },
});
const { GenerationSettings } =
  await import("../src/production/GenerationSettings");
const { MediaModels } = await import("../src/models/MediaModels");
const { ImageProcessingPreview } =
  await import("../src/workspace/imageReview/ImageProcessingPreview");
const { ImageReviewContext } =
  await import("../src/workspace/imageReview/ImageReviewContext");
const { newProject } = await import("../src/model");
const { effectiveParameters } = await import("../src/production/parameters");
const { modelLibrary } = await import("../src/models/catalogSpecs");
const { catalogMediaModel } = await import("../src/models/catalogMediaModel");
const models: MediaModel[] = modelLibrary
  .filter((spec) => spec.family)
  .map((spec) => ({
    ...catalogMediaModel(spec, "video"),
    connectionId: `fixture-${spec.plugin ?? "fal"}`,
  }));
const image: Asset = {
  id: "image",
  name: "产品参考",
  kind: "image",
  path: `data:image/svg+xml,${encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="640" height="400"><rect width="640" height="400" fill="#efece5"/><rect x="220" y="80" width="200" height="240" rx="40" fill="#679178"/><path d="M260 80v-20q60-60 120 0v20" fill="none" stroke="#365940" stroke-width="15"/></svg>')}`,
  preview: "",
  width: 640,
  height: 400,
  duration: 0,
  hasAudio: false,
  inLibrary: true,
  imageId: "image",
  imageVersionId: "original:image",
};
function Fixture() {
  const [items, setItems] = useState(models);
  const [asset, setAsset] = useState(image);
  const [task, setTask] = useState<ProductionTask>({
    key: "fixture",
    title: "产品镜头",
    kind: "video",
    mode: "multi",
    modelId: models.find((m) => m.endpoint.endsWith("/reference-to-video"))!.id,
    prompt: "相机缓慢推进",
    inputs: [],
    parameters: { duration: 8, resolution: "2K" },
  });
  const project = { ...newProject("UI 验收"), assets: [asset] };
  return (
    <Theme appearance="light" accentColor="tomato" radius="small">
      <main style={{ maxWidth: 1200, margin: "32px auto", padding: 20 }}>
        <h2>生成设置与参考交互验收</h2>
        <output data-testid="duration">
          任务时长：{task.parameters?.duration} 秒；生效时长：
          {
            effectiveParameters(
              items.find((m) => m.id === task.modelId),
              task.parameters,
            ).duration
          }{" "}
          秒
        </output>
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "420px 1fr",
            gap: 32,
            marginTop: 20,
          }}
        >
          <section
            style={{
              padding: "0 24px 24px",
              border: "1px solid var(--desk-rule)",
              borderRadius: 8,
            }}
          >
            <GenerationSettings
              task={task}
              project={project}
              models={items}
              close={() => {}}
              save={() => {}}
              onPatch={(patch) => setTask((t) => ({ ...t, ...patch }))}
            />
          </section>
          <section style={{ minWidth: 0 }}>
            <MediaModels
              kind="video"
              hub={{
                models: items,
                loading: false,
                error: "",
                refresh: () => {},
                save: async (model) =>
                  setItems((all) =>
                    all.map((m) => (m.id === model.id ? model : m)),
                  ),
                remove: async (id) =>
                  setItems((all) => all.filter((m) => m.id !== id)),
              }}
            />
          </section>
        </div>
        <section
          style={{
            marginTop: 32,
            height: 400,
            border: "1px solid var(--desk-rule)",
            borderRadius: 8,
            overflow: "hidden",
          }}
        >
          <ImageReviewContext.Provider
            value={{
              projectId: "fixture",
              hasAsset: () => true,
              submit: async () => {},
              process: async () => {
                throw new Error("Not used");
              },
              versions: async () => [{ id: "original:image", image: asset }],
              selectReference: async () => {
                const next = { ...asset, inLibrary: true };
                setAsset(next);
                return next;
              },
              clearReference: async () =>
                setAsset((a) => ({ ...a, inLibrary: false })),
            }}
          >
            <ImageProcessingPreview asset={asset} close={() => {}} />
          </ImageReviewContext.Provider>
        </section>
      </main>
    </Theme>
  );
}
createRoot(document.getElementById("root")!).render(<Fixture />);
