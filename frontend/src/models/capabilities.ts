import protocolDefaults from "./config/protocols.json";
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
  /** Generated source clip length in seconds. */
  duration?: ControlDeclaration;
  /** Custom pixel size, written as `{ width, height }`. */
  imageSize?: ControlDeclaration;
};

export type CapabilityDeclaration = {
  ratioFromReference?: boolean;
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

const defaults = protocolDefaults as Record<string, CapabilityDeclaration>;

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
  return defaults[model.plugin]?.references ?? [];
}

export function controlDeclarations(model: {
  plugin: string;
  capabilities?: CapabilityDeclaration;
}): ControlDeclarations {
  const declared = declarationOf(model).controls;
  if (declared) return declared;
  return defaults[model.plugin]?.controls ?? {};
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

/** Display frame-derived ratio only when explicitly configured. */
export function frameRatioModel(model: {
  plugin: string;
  capabilities?: CapabilityDeclaration;
}): boolean {
  return declarationOf(model).ratioFromReference === true;
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
  // Each role has its own cap; first-frame max=1 and last-frame max=1 allow two images.
  return {
    count: capabilities.referenceLimit ?? null,
    seconds: capabilities.referenceSeconds ?? null,
    kinds,
  };
}

export {
  pointerSegments,
  pointerGet,
  pointerSet,
  pointerDelete,
  copyParameters,
} from "./capabilityPointers";
export {
  validateCapabilities,
  parseCapabilities,
} from "./capabilityValidation";
