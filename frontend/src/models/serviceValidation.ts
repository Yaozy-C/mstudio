import type { ServiceConnection } from "./connectionStore";
import { localEndpoint } from "./types";
export type ServiceField = "name" | "endpoint" | "key";
export type ServiceErrors = Partial<Record<ServiceField, string>>;
export function validateService(
  service: ServiceConnection,
  initial: ServiceConnection,
  key: string,
  clearKey: boolean,
): ServiceErrors {
  const errors: ServiceErrors = {};
  if (!service.name.trim()) errors.name = "请填写连接名称";
  const endpoint = service.endpoint.trim().replace(/\/+$/, "");
  try {
    const url = new URL(endpoint);
    if (
      url.protocol !== "https:" &&
      !(url.protocol === "http:" && localEndpoint(endpoint))
    )
      errors.endpoint = "请使用 HTTPS 或本机 HTTP 地址";
    else if (url.username || url.password || url.search || url.hash)
      errors.endpoint = "服务地址不能含凭据、查询参数或片段";
    else if (
      service.kind === "gemini-native" &&
      url.pathname.replace(/\//g, "")
    )
      errors.endpoint = "Gemini 服务地址不附加 /v1 或 /v1beta";
  } catch {
    errors.endpoint = "请填写有效的服务地址";
  }
  if (key.trim().length > 8192 || /[\r\n]/.test(key.trim()))
    errors.key = "API Key 格式无效";
  else if (
    initial.hasKey &&
    endpoint !== initial.endpoint.trim().replace(/\/+$/, "") &&
    !key.trim() &&
    !clearKey
  )
    errors.key = "地址已修改，请重新输入 API Key";
  return errors;
}
