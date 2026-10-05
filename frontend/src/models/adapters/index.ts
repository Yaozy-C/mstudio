import { parseUrl } from "../parseUrl";
/** Business input is independent of vendor JSON. Reference roles stay explicit. */
export type ModelInput = {
  kind: "image" | "video" | "audio";
  role: "reference" | "first-frame" | "last-frame" | "mask";
  url: string;
};
export type ModelRequest = {
  prompt: string;
  inputs?: ModelInput[];
  options?: Record<string, unknown>;
};
export type ReferenceField = {
  key: string;
  kind: ModelInput["kind"];
  role: ModelInput["role"];
  multiple: boolean;
  required?: boolean;
};
export interface ModelAdapter {
  id: string;
  fields: ReferenceField[];
  encode(request: ModelRequest): Record<string, unknown>;
}
function create(
  id: string,
  fields: ReferenceField[],
  check?: (r: ModelRequest) => void,
): ModelAdapter {
  return {
    id,
    fields,
    encode({ prompt, inputs = [], options = {} }) {
      if (!prompt.trim()) throw new Error("请输入生成描述");
      check?.({ prompt, inputs, options });
      for (const input of inputs) {
        if (!fields.some((f) => f.kind === input.kind && f.role === input.role))
          throw new Error(
            `此模型适配器不支持 ${input.kind} / ${input.role} 输入`,
          );
        let valid = false;
        try {
          const url = parseUrl(input.url);
          valid =
            ["https:", "http:"].includes(url.protocol) &&
            !url.username &&
            !url.password;
        } catch {
          /* validate data URI below */
        }
        if (
          !valid &&
          !new RegExp(`^data:${input.kind}/[a-zA-Z0-9.+-]+;base64,`).test(
            input.url,
          )
        )
          throw new Error("参考媒体需要有效 URL 或 Base64 数据");
      }
      const result: Record<string, unknown> = { ...options, prompt };
      for (const field of fields) {
        const values = inputs
          .filter((i) => i.kind === field.kind && i.role === field.role)
          .map((i) => i.url);
        if (field.required && !values.length)
          throw new Error("图片编辑需要至少一个有效的参考图片 URL");
        if (!field.multiple && values.length > 1)
          throw new Error("此输入角色只允许一个文件");
        if (values.length)
          result[field.key] = field.multiple ? values : values[0];
        else delete result[field.key];
      }
      return result;
    },
  };
}
const imageEdit = create("image-edit", [
  {
    key: "image_urls",
    kind: "image",
    role: "reference",
    multiple: true,
    required: true,
  },
]);
const firstFrame = create("first-frame-video", [
  { key: "image_url", kind: "image", role: "first-frame", multiple: false },
]);
const h3Frames = create("h3-keyframes", [
  { key: "image_url", kind: "image", role: "first-frame", multiple: false },
  { key: "end_image_url", kind: "image", role: "last-frame", multiple: false },
  {
    key: "target_audio_url",
    kind: "audio",
    role: "reference",
    multiple: false,
  },
]);
const h3Reference = create(
  "h3-reference-video",
  [
    {
      key: "reference_image_urls",
      kind: "image",
      role: "reference",
      multiple: true,
    },
    {
      key: "reference_video_urls",
      kind: "video",
      role: "reference",
      multiple: true,
    },
    {
      key: "reference_audio_urls",
      kind: "audio",
      role: "reference",
      multiple: true,
    },
  ],
  ({ inputs = [] }) => {
    if (inputs.length > 12) throw new Error("H3 全参考最多 12 个文件");
    for (const [kind, limit] of [
      ["image", 9],
      ["video", 3],
      ["audio", 3],
    ] as const)
      if (inputs.filter((i) => i.kind === kind).length > limit)
        throw new Error(`H3 最多 ${limit} 个 ${kind} 参考`);
  },
);
const text = create("text-to-media", []);
const custom = create("custom-provider-parameters", []);
export function modelAdapter(provider: string, endpoint: string): ModelAdapter {
  if (provider === "codex-image")
    return create(
      "codex-image",
      [{ key: "image", kind: "image", role: "reference", multiple: true }],
      ({ inputs = [] }) => {
        if (
          inputs.some(
            (i) => !/^data:image\/(png|jpeg|webp);base64,/.test(i.url),
          )
        )
          throw new Error("Codex 请使用项目图片或内嵌图片数据");
      },
    );
  if (provider === "gemini-native")
    return create(
      "gemini-image",
      [{ key: "image_urls", kind: "image", role: "reference", multiple: true }],
      ({ inputs = [] }) => {
        if (inputs.length > 9) throw new Error("最多 9 张参考图片");
        if (
          inputs.some(
            (i) => !/^data:image\/(png|jpeg|webp);base64,/.test(i.url),
          )
        )
          throw new Error(
            "Google 官方生图请使用项目中的参考图片，或内嵌图片数据",
          );
      },
    );
  if (provider === "http-json")
    return {
      id: "http-json",
      fields: [],
      encode({ prompt, inputs = [], options = {} }) {
        if (!prompt.trim()) throw new Error("请输入生成描述");
        if (inputs.length)
          throw new Error("自定义 HTTP 请在请求模板中配置服务可访问的参考 URL");
        const replace = (value: unknown): unknown =>
          typeof value === "string"
            ? value.replaceAll("{{prompt}}", prompt)
            : Array.isArray(value)
              ? value.map(replace)
              : value && typeof value === "object"
                ? Object.fromEntries(
                    Object.entries(value).map(([k, v]) => [k, replace(v)]),
                  )
                : value;
        return replace(options) as Record<string, unknown>;
      },
    };
  if (provider !== "fal") throw new Error(`服务适配器尚未安装：${provider}`);
  if (endpoint === "minimax/h3/image-to-video") return h3Frames;
  if (endpoint === "minimax/h3/reference-to-video") return h3Reference;
  if (endpoint === "minimax/h3-max/image-to-video") return firstFrame;
  if (
    /^openai\/gpt-image-2\.5\/(sunburst|flare)\/edit$/.test(endpoint) ||
    endpoint === "fal-ai/nano-banana-2/edit"
  )
    return imageEdit;
  if (
    endpoint.endsWith("/text-to-video") ||
    endpoint.endsWith("/text-to-image") ||
    endpoint === "fal-ai/nano-banana-2"
  )
    return text;
  return custom;
}
/** Convert the model form parameters into the common request at the adapter boundary. */
export function presetRequest(
  adapter: ModelAdapter,
  prompt: string,
  params: Record<string, unknown>,
): ModelRequest {
  const options = { ...params };
  const inputs: ModelInput[] = [];
  for (const field of adapter.fields) {
    const value = options[field.key];
    delete options[field.key];
    if (value === undefined || value === "") continue;
    const values = field.multiple ? value : [value];
    if (!Array.isArray(values) || values.some((v) => typeof v !== "string"))
      throw new Error(`参考字段 ${field.key} 格式无效`);
    for (const url of values)
      inputs.push({ kind: field.kind, role: field.role, url });
  }
  return { prompt, inputs, options };
}
