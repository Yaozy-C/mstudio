import {
  referenceRoles,
  type CapabilityDeclaration,
  type ReferenceRole,
  type ControlDeclaration,
} from "./capabilities";
import type { MediaKind } from "./mediaRegistry";
import { pointerSegments } from "./capabilityPointers";
const referenceKinds: MediaKind[] = ["image", "video", "audio"];

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
        "ratioFromReference",
        "references",
        "controls",
        "referenceLimit",
        "referenceSeconds",
      ].includes(key)
    )
      throw new Error(`capabilities 不支持字段：${key}`);
  const result: CapabilityDeclaration = {};
  if (capabilities.ratioFromReference !== undefined) {
    if (typeof capabilities.ratioFromReference !== "boolean")
      throw new Error("ratioFromReference 必须是布尔值");
    result.ratioFromReference = capabilities.ratioFromReference;
  }
  if (capabilities.references !== undefined) {
    if (!Array.isArray(capabilities.references))
      throw new Error("capabilities.references 必须是数组");
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
          entry.values.some(
            (item) => typeof item !== "string" || !item.trim().length,
          )
        )
          throw new Error(`参数控件 ${name} 的 values 必须是非空字符串数组`);
        control.values = entry.values as string[];
      }
      for (const bound of ["min", "max"] as const) {
        const value = entry[bound];
        if (value === undefined) continue;
        if (!Number.isFinite(value) || (value as number) < 0)
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
    if (key === "referenceLimit" && !Number.isInteger(raw))
      throw new Error("capabilities.referenceLimit 必须是正整数");
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
