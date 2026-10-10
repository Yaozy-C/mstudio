/**
 * Media model capabilities are declared by the model, not inferred from its endpoint.
 *
 * A protocol (fal queue, custom HTTP, Gemini native, Codex) only defines how bytes travel:
 * authentication, submission, polling and result normalization. Which reference roles a
 * model accepts and which generation controls it exposes are model facts, so they live in
 * the model record (`MediaModel.capabilities`) and travel with the model through the
 * catalog, the project and the Agent runtime.
 *
 * Protocol defaults exist only where the wire format itself fixes the answer: the Gemini
 * and Codex adapters build their own request bodies, so their reference vocabulary cannot
 * be changed by configuration. fal and custom HTTP keep no defaults — an endpoint ID or a
 * request template proves nothing about references.
 */
import type { MediaKind } from "./mediaRegistry";

/** Reference positions the request encoder understands. Mirrors `ModelInput["role"]`. */
export type ReferenceRole = "reference" | "first-frame" | "last-frame" | "mask";
export const referenceRoles: ReferenceRole[] = [
  "reference",
  "first-frame",
  "last-frame",
  "mask",
];
const referenceKinds: MediaKind[] = ["image", "video", "audio"];

/**
 * One accepted reference input. `key` is a JSON Pointer into the request body, so a vendor
 * field nested under `input` is expressed as `/input/img_url` and a top-level field as
 * `/image_url`. The pointer is the same vocabulary the custom HTTP result mapping uses.
 */
export type ReferenceDeclaration = {
  key: string;
  kind: MediaKind;
  role: ReferenceRole;
  multiple?: boolean;
  required?: boolean;
  /** Maximum count when `multiple`; enforced before submission. */
  max?: number;
};

/** One generation control the model accepts, and where its value belongs in the request. */
export type ControlDeclaration = {
  path: string;
  /** Allowed values, in the order the UI should present them. */
  values?: string[];
  /** Inclusive bounds for numeric controls. */
  min?: number;
  max?: number;
};

export type ControlDeclarations = {
  /** Output width:height ratio, e.g. `16:9`. */
  aspectRatio?: ControlDeclaration;
  /** Output resolution tier, e.g. `1080P`. Values are case-sensitive. */
  resolution?: ControlDeclaration;
  /** Generated source clip length in whole seconds. */
  duration?: ControlDeclaration;
  /** Custom pixel size, written as `{ width, height }`. */
  imageSize?: ControlDeclaration;
};

export type CapabilityDeclaration = {
  references?: ReferenceDeclaration[];
  controls?: ControlDeclarations;
  /** Total references accepted in one request. */
  referenceLimit?: number;
  /** Total reference video seconds accepted in one request. */
  referenceSeconds?: number;
};

export type ReferenceField = {
  key: string;
  kind: MediaKind;
  role: ReferenceRole;
  multiple: boolean;
  required: boolean;
  max: number | null;
};

export type ResolvedControl = {
  path: string;
  values: string[];
  min: number | null;
  max: number | null;
};

export type ResolvedControls = {
  aspectRatio: ResolvedControl | null;
  resolution: ResolvedControl | null;
  duration: ResolvedControl | null;
  imageSize: ResolvedControl | null;
};

export const imageRatios = ["1:1", "9:16", "16:9", "3:4", "4:3", "2:3", "3:2"];
export const videoRatios = ["9:16", "16:9", "1:1", "3:4", "4:3", "21:9"];
export const imageResolutions = ["1K", "2K", "4K"];
export const videoResolutions = ["480P", "768P", "2K", "4K"];
export const videoMaxResolutions = ["480P", "768P", "1080P"];
export const defaultDurationRange = { min: 5, max: 15 };
/**
 * System caps for one request. A model may lower them by declaring smaller limits; the
 * upload path enforces the same ceiling so a declaration can never exceed what the app can
 * prepare and submit.
 */
export const referenceSafetyCaps = {
  image: 9,
  video: 3,
  audio: 3,
  count: 12,
  seconds: 15,
};

/**
 * Reference vocabulary fixed by a request builder, not by the model.
 *
 * `gemini-native` and `codex-image` encode references into their own JSON, so every model
 * on those protocols accepts the same image references. fal and `http-json` post the
 * configured body verbatim: whatever a model accepts there must be declared.
 */
const protocolReferences: Record<string, ReferenceDeclaration[]> = {
  "gemini-native": [
    {
      key: "/image_urls",
      kind: "image",
      role: "reference",
      multiple: true,
      max: 9,
    },
  ],
  "codex-image": [
    { key: "/image", kind: "image", role: "reference", multiple: true },
  ],
};

/**
 * Control vocabulary fixed by a request builder. Gemini writes its options into
 * `generationConfig`; every other protocol posts the configured body, so its models
 * declare their own control paths.
 */
const protocolControls: Record<string, ControlDeclarations> = {
  "gemini-native": {
    aspectRatio: {
      path: "/generationConfig/imageConfig/aspectRatio",
      values: imageRatios,
    },
    resolution: {
      path: "/generationConfig/imageConfig/imageSize",
      values: imageResolutions,
    },
  },
};

export function declarationOf(model: {
  plugin: string;
  capabilities?: CapabilityDeclaration;
}): CapabilityDeclaration {
  return model.capabilities ?? {};
}

/** Declared references, falling back to the protocol vocabulary when nothing is declared. */
export function referenceDeclarations(model: {
  plugin: string;
  capabilities?: CapabilityDeclaration;
}): ReferenceDeclaration[] {
  const declared = declarationOf(model).references;
  if (declared) return declared;
  return protocolReferences[model.plugin] ?? [];
}

export function controlDeclarations(model: {
  plugin: string;
  capabilities?: CapabilityDeclaration;
}): ControlDeclarations {
  const declared = declarationOf(model).controls;
  if (declared) return declared;
  return protocolControls[model.plugin] ?? {};
}

export function referenceFields(model: {
  plugin: string;
  capabilities?: CapabilityDeclaration;
}): ReferenceField[] {
  return referenceDeclarations(model).map((entry) => ({
    key: entry.key,
    kind: entry.kind,
    role: entry.role,
    multiple: entry.multiple === true,
    required: entry.required === true,
    max: entry.max ?? null,
  }));
}

export function resolvedControls(model: {
  plugin: string;
  capabilities?: CapabilityDeclaration;
}): ResolvedControls {
  const controls = controlDeclarations(model);
  const resolve = (control?: ControlDeclaration): ResolvedControl | null =>
    control
      ? {
          path: control.path,
          values: control.values ?? [],
          min: control.min ?? null,
          max: control.max ?? null,
        }
      : null;
  return {
    aspectRatio: resolve(controls.aspectRatio),
    resolution: resolve(controls.resolution),
    duration: resolve(controls.duration),
    imageSize: resolve(controls.imageSize),
  };
}

/**
 * A first-frame model derives the output ratio from the supplied image, so it exposes no
 * ratio control while accepting a first frame. The UI states that instead of offering
 * ratios the provider would ignore.
 */
export function frameRatioModel(model: {
  plugin: string;
  capabilities?: CapabilityDeclaration;
}): boolean {
  const controls = resolvedControls(model);
  if (controls.aspectRatio) return false;
  return referenceFields(model).some((field) => field.role === "first-frame");
}

export function referenceLimits(model: {
  plugin: string;
  capabilities?: CapabilityDeclaration;
}): {
  count: number | null;
  seconds: number | null;
  kinds: Record<MediaKind, number | null>;
} {
  const capabilities = declarationOf(model);
  const kinds: Record<MediaKind, number | null> = {
    image: null,
    video: null,
    audio: null,
  };
  for (const field of referenceFields(model)) {
    if (field.max === null) continue;
    kinds[field.kind] = Math.max(kinds[field.kind] ?? 0, field.max);
  }
  return {
    count: capabilities.referenceLimit ?? null,
    seconds: capabilities.referenceSeconds ?? null,
    kinds,
  };
}

/** JSON Pointer reads and writes over plain request objects. */
export function pointerSegments(path: string): string[] {
  if (!path.startsWith("/"))
    throw new Error(`路径必须是 JSON Pointer，例如 /image_url：${path}`);
  return path
    .slice(1)
    .split("/")
    .map((segment) => segment.replace(/~1/g, "/").replace(/~0/g, "~"));
}

export function pointerGet(target: unknown, path: string): unknown {
  let current: unknown = target;
  for (const segment of pointerSegments(path)) {
    if (Array.isArray(current)) {
      const index = Number(segment);
      if (!Number.isInteger(index)) return undefined;
      current = current[index];
      continue;
    }
    if (!current || typeof current !== "object") return undefined;
    current = (current as Record<string, unknown>)[segment];
  }
  return current;
}

export function pointerSet(
  target: Record<string, unknown>,
  path: string,
  value: unknown,
): void {
  const segments = pointerSegments(path);
  let current = target;
  for (const segment of segments.slice(0, -1)) {
    const next = current[segment];
    if (!next || typeof next !== "object" || Array.isArray(next))
      current[segment] = {};
    current = current[segment] as Record<string, unknown>;
  }
  current[segments[segments.length - 1]] = value;
}

export function pointerDelete(
  target: Record<string, unknown>,
  path: string,
): void {
  const segments = pointerSegments(path);
  let current: Record<string, unknown> | undefined = target;
  for (const segment of segments.slice(0, -1)) {
    const next = current[segment];
    if (!next || typeof next !== "object" || Array.isArray(next)) return;
    current = next as Record<string, unknown>;
  }
  delete current[segments[segments.length - 1]];
}

/** Deep copy of model parameters, so declared paths can be written without mutating them. */
export function copyParameters(
  value: Record<string, unknown>,
): Record<string, unknown> {
  return JSON.parse(JSON.stringify(value)) as Record<string, unknown>;
}

/**
 * Rejects a declaration the request encoder could not honour. Capabilities are declarative:
 * names, roles and pointers are checked against fixed vocabularies, and nothing here can
 * execute provider logic.
 */
export function validateCapabilities(value: unknown): CapabilityDeclaration {
  if (value === undefined || value === null) return {};
  if (typeof value !== "object" || Array.isArray(value))
    throw new Error("capabilities 必须是 JSON 对象");
  const capabilities = value as Record<string, unknown>;
  for (const key of Object.keys(capabilities))
    if (
      ![
        "references",
        "controls",
        "referenceLimit",
        "referenceSeconds",
      ].includes(key)
    )
      throw new Error(`capabilities 不支持字段：${key}`);
  const result: CapabilityDeclaration = {};
  if (capabilities.references !== undefined) {
    if (!Array.isArray(capabilities.references))
      throw new Error("capabilities.references 必须是数组");
    if (capabilities.references.length > 24)
      throw new Error("参考输入最多声明 24 条");
    const seen = new Set<string>();
    result.references = capabilities.references.map((raw) => {
      if (!raw || typeof raw !== "object" || Array.isArray(raw))
        throw new Error("参考输入必须是对象");
      const entry = raw as Record<string, unknown>;
      for (const key of Object.keys(entry))
        if (
          !["key", "kind", "role", "multiple", "required", "max"].includes(key)
        )
          throw new Error(`参考输入不支持字段：${key}`);
      const key = entry.key;
      if (typeof key !== "string" || !key.startsWith("/"))
        throw new Error("参考字段 key 必须是 JSON Pointer，例如 /image_url");
      pointerSegments(key);
      if (seen.has(key)) throw new Error(`参考字段重复：${key}`);
      seen.add(key);
      if (!referenceKinds.includes(entry.kind as MediaKind))
        throw new Error(`参考类型无效：${String(entry.kind)}`);
      if (!referenceRoles.includes(entry.role as ReferenceRole))
        throw new Error(`参考用途无效：${String(entry.role)}`);
      for (const flag of ["multiple", "required"])
        if (entry[flag] !== undefined && typeof entry[flag] !== "boolean")
          throw new Error(`参考字段 ${flag} 必须是布尔值`);
      const max = entry.max;
      if (max !== undefined && (!Number.isInteger(max) || (max as number) < 1))
        throw new Error("参考上限 max 必须是正整数");
      return {
        key,
        kind: entry.kind as MediaKind,
        role: entry.role as ReferenceRole,
        ...(entry.multiple === undefined
          ? {}
          : { multiple: entry.multiple as boolean }),
        ...(entry.required === undefined
          ? {}
          : { required: entry.required as boolean }),
        ...(max === undefined ? {} : { max: max as number }),
      };
    });
  }
  if (capabilities.controls !== undefined) {
    if (
      !capabilities.controls ||
      typeof capabilities.controls !== "object" ||
      Array.isArray(capabilities.controls)
    )
      throw new Error("capabilities.controls 必须是对象");
    const controls = capabilities.controls as Record<string, unknown>;
    for (const key of Object.keys(controls))
      if (!["aspectRatio", "resolution", "duration", "imageSize"].includes(key))
        throw new Error(`参数控件不支持字段：${key}`);
    result.controls = {};
    for (const name of [
      "aspectRatio",
      "resolution",
      "duration",
      "imageSize",
    ] as const) {
      const raw = controls[name];
      if (raw === undefined) continue;
      if (!raw || typeof raw !== "object" || Array.isArray(raw))
        throw new Error(`参数控件 ${name} 必须是对象`);
      const entry = raw as Record<string, unknown>;
      for (const key of Object.keys(entry))
        if (!["path", "values", "min", "max"].includes(key))
          throw new Error(`参数控件 ${name} 不支持字段：${key}`);
      if (typeof entry.path !== "string" || !entry.path.startsWith("/"))
        throw new Error(`参数控件 ${name} 的 path 必须是 JSON Pointer`);
      pointerSegments(entry.path);
      const control: ControlDeclaration = { path: entry.path };
      if (entry.values !== undefined) {
        if (
          !Array.isArray(entry.values) ||
          !entry.values.length ||
          entry.values.length > 64 ||
          entry.values.some((item) => typeof item !== "string" || !item.length)
        )
          throw new Error(`参数控件 ${name} 的 values 必须是非空字符串数组`);
        control.values = entry.values as string[];
      }
      for (const bound of ["min", "max"] as const) {
        const value = entry[bound];
        if (value === undefined) continue;
        if (!Number.isFinite(value))
          throw new Error(`参数控件 ${name} 的 ${bound} 必须是数字`);
        control[bound] = value as number;
      }
      if (
        control.min !== undefined &&
        control.max !== undefined &&
        control.min > control.max
      )
        throw new Error(`参数控件 ${name} 的 min 不能大于 max`);
      if (["duration", "imageSize"].includes(name) && control.values)
        throw new Error(`参数控件 ${name} 使用 path 与 min/max`);
      if (!["duration", "imageSize"].includes(name) && !control.values)
        throw new Error(`参数控件 ${name} 需要 values`);
      result.controls[name] = control;
    }
  }
  for (const key of ["referenceLimit", "referenceSeconds"] as const) {
    const raw = capabilities[key];
    if (raw === undefined) continue;
    if (!Number.isFinite(raw) || (raw as number) <= 0)
      throw new Error(`capabilities.${key} 必须是正数`);
    if (key === "referenceLimit" && (raw as number) > referenceSafetyCaps.count)
      throw new Error(
        `capabilities.referenceLimit 不能超过系统上限 ${referenceSafetyCaps.count}`,
      );
    if (
      key === "referenceSeconds" &&
      (raw as number) > referenceSafetyCaps.seconds
    )
      throw new Error(
        `capabilities.referenceSeconds 不能超过系统上限 ${referenceSafetyCaps.seconds}`,
      );
    result[key] = raw as number;
  }
  return result;
}

/** Parses the declaration a user typed in the model form. */
export function parseCapabilities(
  text: string,
): CapabilityDeclaration | undefined {
  if (!text.trim()) return undefined;
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    throw new Error("能力声明不是有效的 JSON");
  }
  const parsed = validateCapabilities(value);
  return Object.keys(parsed).length ? parsed : undefined;
}
