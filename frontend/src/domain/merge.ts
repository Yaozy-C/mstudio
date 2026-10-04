// Three-way merge of JSON project values. Objects and entity arrays merge by
// identity; ordered scalar arrays conflict as a unit. No last-writer-wins edits.
export const equal = (a: unknown, b: unknown): boolean => {
  if (a === b) return true;
  if (!a || !b || typeof a !== "object" || typeof b !== "object") return false;
  if (Array.isArray(a) || Array.isArray(b))
    return (
      Array.isArray(a) &&
      Array.isArray(b) &&
      a.length === b.length &&
      a.every((v, i) => equal(v, b[i]))
    );
  const left = a as Record<string, unknown>,
    right = b as Record<string, unknown>;
  const keys = new Set([...Object.keys(left), ...Object.keys(right)]);
  return [...keys].every((k) => equal(left[k], right[k]));
};
const record = (v: unknown): v is Record<string, unknown> =>
  !!v && typeof v === "object" && !Array.isArray(v);
const entityArray = (v: unknown[]): v is Record<string, unknown>[] =>
  v.every((item) => record(item) && typeof item.id === "string") &&
  new Set(v.map((item) => (item as { id: string }).id)).size === v.length;
export function mergeValue(
  base: unknown,
  local: unknown,
  current: unknown,
  path = "",
  resolve?: "local" | "current",
): unknown {
  if (equal(base, local)) return current;
  if (equal(base, current) || equal(local, current)) return local;
  if (record(base) && record(local) && record(current)) {
    const output: Record<string, unknown> = {};
    for (const key of new Set([
      ...Object.keys(base),
      ...Object.keys(local),
      ...Object.keys(current),
    ])) {
      if (!path && ["revision", "storageVersion"].includes(key)) {
        output[key] = current[key];
        continue;
      }
      const value = mergeValue(
        base[key],
        local[key],
        current[key],
        `${path}/${key}`,
        resolve,
      );
      if (value !== undefined) output[key] = value;
    }
    return output;
  }
  if (
    Array.isArray(base) &&
    Array.isArray(local) &&
    Array.isArray(current) &&
    entityArray(base) &&
    entityArray(local) &&
    entityArray(current)
  ) {
    const map = (rows: Record<string, unknown>[]) =>
      new Map(rows.map((row) => [row.id, row]));
    const b = map(base),
      l = map(local),
      c = map(current);
    const common = base.map((v) => v.id).filter((id) => l.has(id) && c.has(id));
    const order = (rows: Record<string, unknown>[]) =>
      rows.map((v) => v.id).filter((id) => common.includes(id));
    const localOrder = order(local),
      currentOrder = order(current);
    if (
      !equal(common, localOrder) &&
      !equal(common, currentOrder) &&
      !equal(localOrder, currentOrder)
    ) {
      if (!resolve) throw conflict(`Concurrent order change at ${path}`);
    }
    const first =
      resolve === "current" || equal(common, localOrder) ? current : local;
    const second = first === current ? local : current;
    const ids = [
      ...new Set([
        ...first.map((v) => v.id),
        ...second.map((v) => v.id),
        ...base.map((v) => v.id),
      ]),
    ];
    return ids
      .map((id) =>
        mergeValue(b.get(id), l.get(id), c.get(id), `${path}/${id}`, resolve),
      )
      .filter((v) => v !== undefined);
  }
  if (resolve) return resolve === "local" ? local : current;
  throw conflict(`Concurrent change at ${path || "/"}`);
}

function conflict(details: string) {
  return new Error(
    JSON.stringify({ code: "PROJECT_CONFLICT", message: details, details }),
  );
}
