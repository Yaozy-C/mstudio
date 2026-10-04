import { parseUrl } from "./parseUrl";
import { modelAdapter } from "./adapters";
import type { MediaModel } from "./mediaRegistry";
export function validateMediaModel(model: MediaModel) {
  if (!model.name.trim() || model.name.length > 80)
    throw new Error("请填写 80 字以内的模型名称");
  if (model.plugin === "codex-image") {
    if (model.kind !== "image" || model.endpoint !== "codex://local/images")
      throw new Error("Codex 仅支持本机图像连接");
  } else if (model.plugin === "fal") {
    if (
      !/^[a-zA-Z0-9_-][a-zA-Z0-9_.-]*(?:\/[a-zA-Z0-9_-][a-zA-Z0-9_.-]*)+$/.test(
        model.endpoint,
      ) ||
      model.endpoint.length > 200
    )
      throw new Error("请填写完整 fal 端点 ID");
  } else if (model.plugin === "gemini-native") {
    if (
      model.kind !== "image" ||
      !/^https:\/\/generativelanguage\.googleapis\.com(?:\/v1beta|\/v1)?\/?$/.test(
        model.endpoint,
      )
    )
      throw new Error("请使用 Google 官方图像接口地址");
    if (
      typeof model.params.model !== "string" ||
      !/^[a-zA-Z0-9._-]+$/.test(model.params.model)
    )
      throw new Error("请填写 Gemini 模型 ID");
  } else if (model.plugin === "http-json") {
    const url = parseUrl(model.endpoint);
    if (
      url.protocol !== "https:" &&
      !(
        url.protocol === "http:" &&
        ["localhost", "127.0.0.1", "[::1]"].includes(url.hostname)
      )
    )
      throw new Error("使用 HTTPS 或本机 HTTP 地址");
    if (url.username || url.password || url.hash)
      throw new Error("地址不能包含凭据或片段");
    if (typeof model.http?.outputPointer !== "string")
      throw new Error("请配置结果 URL 字段");
  }
  if (
    !model.params ||
    Array.isArray(model.params) ||
    typeof model.params !== "object"
  )
    throw new Error("模型参数须为 JSON 对象");
  if (!["image", "video", "audio"].includes(model.kind))
    throw new Error("模型插件或能力类型无效");
  modelAdapter(model.plugin, model.endpoint);
}
