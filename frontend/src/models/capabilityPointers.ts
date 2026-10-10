/** JSON Pointer reads and writes over plain request objects. */
export function pointerSegments(path: string): string[] {
  if (!path.startsWith("/") || path.length > 300 || /~(?![01])/.test(path))
    throw new Error(`路径必须是 JSON Pointer，例如 /image_url：${path}`);
  const segments = path
    .slice(1)
    .split("/")
    .map((segment) => segment.replace(/~1/g, "/").replace(/~0/g, "~"));
  if (
    segments.some((segment) =>
      ["__proto__", "constructor", "prototype"].includes(segment),
    )
  )
    throw new Error("JSON Pointer 包含不支持的属性");
  return segments;
}

type Container = Record<string, unknown> | unknown[];
const indexOf = (segment: string) =>
  /^(0|[1-9][0-9]*)$/.test(segment) && Number.isSafeInteger(Number(segment));
function get(container: Container, segment: string): unknown {
  if (Array.isArray(container))
    return indexOf(segment) ? container[Number(segment)] : undefined;
  return Object.hasOwn(container, segment) ? container[segment] : undefined;
}
function set(container: Container, segment: string, value: unknown) {
  if (Array.isArray(container)) {
    if (!indexOf(segment) || Number(segment) > container.length)
      throw new Error("JSON Pointer 数组索引无效或超出范围");
    container[Number(segment)] = value;
  } else container[segment] = value;
}
const isContainer = (value: unknown): value is Container =>
  value !== null && typeof value === "object";

export function pointerGet(target: unknown, path: string): unknown {
  let current = target;
  for (const segment of pointerSegments(path)) {
    if (!isContainer(current)) return undefined;
    current = get(current, segment);
  }
  return current;
}

export function pointerSet(
  target: Record<string, unknown>,
  path: string,
  value: unknown,
): void {
  const segments = pointerSegments(path);
  let current: Container = target;
  for (let i = 0; i < segments.length - 1; i++) {
    const segment = segments[i];
    let next = get(current, segment);
    if (!isContainer(next)) {
      next = indexOf(segments[i + 1]) ? [] : {};
      set(current, segment, next);
    }
    current = next as Container;
  }
  set(current, segments[segments.length - 1], value);
}

export function pointerDelete(
  target: Record<string, unknown>,
  path: string,
): void {
  const segments = pointerSegments(path);
  let current: Container = target;
  for (const segment of segments.slice(0, -1)) {
    const next = get(current, segment);
    if (!isContainer(next)) return;
    current = next;
  }
  const last = segments[segments.length - 1];
  if (Array.isArray(current)) {
    if (indexOf(last)) current.splice(Number(last), 1);
  } else delete current[last];
}

/** Deep copy of model parameters, so declared paths never mutate a saved preset. */
export function copyParameters(
  value: Record<string, unknown>,
): Record<string, unknown> {
  return JSON.parse(JSON.stringify(value)) as Record<string, unknown>;
}
