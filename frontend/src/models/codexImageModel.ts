/** Generation requests require an explicit model ID. */
export function codexImageModel(params: Record<string, unknown>): string {
  const id = params.model;
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
