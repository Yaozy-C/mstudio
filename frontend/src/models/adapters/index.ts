/**
 * Business input is independent of vendor JSON. Reference roles come from the model's own
 * capability declaration, so a new endpoint is added without touching this file.
 *
 * A protocol adapter only knows where a reference field lives in the request body and what
 * the transport can carry: the Gemini and Codex builders embed images themselves, custom
 * HTTP posts a fixed template, and fal posts configured parameters verbatim.
 */
import { parseUrl } from "../parseUrl";
import type { MediaModel } from "../mediaRegistry";
import {
  pointerDelete,
  pointerGet,
  pointerSet,
  referenceLimits,
  referenceFields,
  type ReferenceField,
} from "../capabilities";

export type { ReferenceField } from "../capabilities";

export type ModelInput = {
  kind: "image" | "video" | "audio";
  role: ReferenceField["role"];
  url: string;
};
export type ModelRequest = {
  prompt: string;
  inputs?: ModelInput[];
  options?: Record<string, unknown>;
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
          pointerSet(result, field.key, field.multiple ? values : values[0]);
        else pointerDelete(result, field.key);
      }
      return result;
    },
  };
}

/** Counts declared across the whole request, so one model can cap mixed references. */
function declaredLimitCheck(model: MediaModel) {
  const limits = referenceLimits(model);
  const fields = referenceFields(model);
  return ({ inputs = [] }: ModelRequest) => {
    if (limits.count !== null && inputs.length > limits.count)
      throw new Error(`此模型最多 ${limits.count} 个参考输入`);
    for (const field of fields) {
      if (field.max === null) continue;
      const used = inputs.filter(
        (i) => i.kind === field.kind && i.role === field.role,
      ).length;
      if (used > field.max)
        throw new Error(`此模型最多 ${field.max} 个 ${field.kind} 参考`);
    }
  };
}

function codexAdapter(model: MediaModel): ModelAdapter {
  return create("codex-image", referenceFields(model), ({ inputs = [] }) => {
    if (inputs.some((i) => !/^data:image\/(png|jpeg|webp);base64,/.test(i.url)))
      throw new Error("Codex 请使用项目图片或内嵌图片数据");
  });
}

function geminiAdapter(model: MediaModel): ModelAdapter {
  return create("gemini-image", referenceFields(model), (request) => {
    declaredLimitCheck(model)(request);
    if (
      (request.inputs ?? []).some(
        (i) => !/^data:image\/(png|jpeg|webp);base64,/.test(i.url),
      )
    )
      throw new Error("Google 官方生图请使用项目中的参考图片，或内嵌图片数据");
  });
}

/** Custom HTTP carries references only through its request template, never as inputs. */
const httpJsonAdapter: ModelAdapter = {
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

/**
 * Resolves the adapter a model actually uses. Reference roles and limits come from the
 * model's declaration; the protocol contributes only what its request builder fixes.
 */
export function mediaAdapter(model: MediaModel): ModelAdapter {
  switch (model.plugin) {
    case "codex-image":
      return codexAdapter(model);
    case "gemini-native":
      return geminiAdapter(model);
    case "http-json":
      return httpJsonAdapter;
    case "fal":
      return create(
        "fal-model",
        referenceFields(model),
        declaredLimitCheck(model),
      );
    default:
      throw new Error(`服务适配器尚未安装：${model.plugin}`);
  }
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
    const value = pointerGet(options, field.key);
    pointerDelete(options, field.key);
    if (value === undefined || value === "") continue;
    const values = field.multiple ? value : [value];
    if (!Array.isArray(values) || values.some((v) => typeof v !== "string"))
      throw new Error(`参考字段 ${field.key} 格式无效`);
    for (const url of values)
      inputs.push({ kind: field.kind, role: field.role, url });
  }
  return { prompt, inputs, options };
}
