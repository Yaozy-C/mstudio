import catalog from "./config/text-context-limits.json";
import type { ModelConnection, TextModelLimits } from "./types";

export function automaticLimits(
  profile: ModelConnection,
): TextModelLimits | null {
  if (profile.adapter === "codex") return null;
  const cached = profile.modelLimits;
  const endpoint = profile.endpoint.replace(/\/+$/, "");
  if (
    cached?.source === "service" &&
    cached.endpoint === endpoint &&
    cached.model === profile.model
  )
    return cached;
  let known: (typeof catalog)[number] | undefined;
  try {
    const url = new URL(endpoint);
    known = catalog.find(
      (entry) =>
        url.protocol === "https:" &&
        !url.port &&
        entry.host === url.hostname &&
        entry.models.includes(profile.model.replace(/^models\//, "")),
    );
  } catch {
    /* An incomplete form uses the labelled fallback until it can resolve. */
  }
  return {
    endpoint,
    model: profile.model,
    contextWindow: known?.contextWindow ?? 262144,
    maxOutputTokens: known?.maxOutputTokens ?? null,
    defaultMaxTokens:
      known?.defaultMaxTokens ??
      (profile.adapter === "anthropic-native" ? 8192 : null),
    source: known ? "catalog" : "fallback",
    sourceUrl: known?.source,
  };
}

export function contextPolicy(profile: ModelConnection) {
  const limits = automaticLimits(profile);
  const automatic =
    profile.contextWindowMode === "auto" || profile.contextWindow == null;
  const window = automatic ? limits?.contextWindow : profile.contextWindow;
  const output =
    profile.adapter === "codex"
      ? null
      : (profile.maxOutputTokens ??
        (limits?.defaultMaxTokens == null
          ? null
          : Math.min(
              limits.defaultMaxTokens,
              limits.maxOutputTokens ?? Infinity,
            )));
  const headroom = profile.compactionHeadroom ?? 65536;
  const threshold =
    window == null
      ? null
      : Math.floor(Math.min(window * 0.8, window - (output ?? 0) - headroom));
  return { limits, automatic, window, output, headroom, threshold };
}

export function contextPolicyIssue(profile: ModelConnection): string {
  const { window, output, headroom, threshold, limits } =
    contextPolicy(profile);
  if (profile.adapter === "codex" && profile.maxOutputTokens != null)
    return "Codex 的输出上限由本机服务管理";
  if (
    window != null &&
    (!Number.isInteger(window) || window < 8192 || window > 2000000)
  )
    return "上下文窗口须在 8192 到 2000000 token 之间";
  if (
    output != null &&
    (!Number.isInteger(output) || output < 1 || output > 2000000)
  )
    return "输出上限须为 1 到 2000000 token";
  if (
    output != null &&
    limits?.maxOutputTokens != null &&
    output > limits.maxOutputTokens
  )
    return "输出上限超过此模型公布的输出能力";
  if (!Number.isInteger(headroom) || headroom < 0 || headroom > 2000000)
    return "压缩余量须在 0 到 2000000 token 之间";
  if (
    threshold != null &&
    window != null &&
    Math.floor((window - (output ?? 0)) * 0.16) >= threshold
  )
    return "上下文窗口不足以容纳输出预留和压缩余量，请调整这三项设置";
  return "";
}
