import { parseUrl } from "./parseUrl";
import { mediaAdapter } from "./adapters";
import { validateCapabilities } from "./capabilities";
import type { MediaModel } from "./mediaRegistry";
/**
 * A capability declaration is data, never code: field pointers, roles and enum values are
 * checked against fixed vocabularies, and a protocol that builds its own request body —
 * or posts a fixed template — rejects declarations it could not honour.
 */
export function validateMediaModel(model: MediaModel) {
  validateCapabilities(model.capabilities);
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
  } else if (model.plugin === "dashscope") {
    const url = parseUrl(model.endpoint);
    if (url.protocol !== "https:" || url.username || url.password || url.hash)
      throw new Error("请填写百炼官方 HTTPS 地址");
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
  const fields = mediaAdapter(model).fields;
  if (
    model.plugin === "http-json" &&
    model.capabilities?.references !== undefined
  )
    throw new Error("自定义 HTTP 的参考素材写在请求模板里，不能声明参考字段");
  if (
    ["gemini-native", "codex-image"].includes(model.plugin) &&
    fields.some((field) => field.kind !== "image")
  )
    throw new Error("此连接的请求格式只支持图片参考");
  if (["gemini-native", "codex-image"].includes(model.plugin)) {
    const key = model.plugin === "gemini-native" ? "/image_urls" : "/image";
    if (
      fields.some(
        (field) =>
          field.key !== key || field.role !== "reference" || !field.multiple,
      )
    )
      throw new Error(
        `此连接的图片参考必须使用固定字段 ${key}、reference 用途和数组格式`,
      );
    const controls = model.capabilities?.controls;
    if (
      model.plugin === "codex-image" &&
      controls &&
      Object.keys(controls).length
    )
      throw new Error("Codex 不支持声明生成参数控件");
    if (model.plugin === "gemini-native" && controls) {
      if (controls.duration || controls.imageSize)
        throw new Error("Gemini 生图不支持时长或自定义尺寸控件");
      if (
        (controls.aspectRatio &&
          controls.aspectRatio.path !==
            "/generationConfig/imageConfig/aspectRatio") ||
        (controls.resolution &&
          controls.resolution.path !==
            "/generationConfig/imageConfig/imageSize")
      )
        throw new Error("Gemini 参数控件必须使用固定字段");
    }
  }
}
