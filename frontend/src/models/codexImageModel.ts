import nativeRequests from "./config/native-requests.json";

/** Resolve old transport aliases while keeping explicitly selected IDs unchanged. */
export function codexImageModel(params: Record<string, unknown>): string {
  const id = params.model;
  if (id === undefined || id === "codex-image")
    return nativeRequests["codex-image"].legacyModel;
  if (
    typeof id !== "string" ||
    id.length > 100 ||
    !id.startsWith("gpt-image-") ||
    id.length <= "gpt-image-".length ||
    /[^a-zA-Z0-9._-]/.test(id)
  )
    throw new Error("请填写有效的 Codex 图像模型 ID");
  return id;
}
