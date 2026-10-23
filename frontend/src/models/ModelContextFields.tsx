import { useEffect, useState } from "react";
import { bridge, native } from "../bridge";
import { t } from "../i18n";
import { contextPolicy } from "./contextLimits";
import type { ModelConnection, TextModelLimits } from "./types";
import { ErrorNotice } from "../errors/ErrorNotice";

export function ModelContextFields({
  profile,
  update,
}: {
  profile: ModelConnection;
  update: (value: Partial<ModelConnection>) => void;
}) {
  const [refresh, setRefresh] = useState(0);
  const [loading, setLoading] = useState(false);
  const [notice, setNotice] = useState("");
  const [error, setError] = useState("");
  const policy = contextPolicy(profile);
  useEffect(() => {
    setLoading(false);
    setError("");
    setNotice("");
    if (
      !native ||
      !profile.connectionId ||
      !profile.model.trim() ||
      profile.adapter === "codex"
    )
      return;
    let active = true;
    const timer = window.setTimeout(() => {
      setLoading(true);
      setNotice("");
      setError("");
      void bridge<TextModelLimits | null>("resolve_text_model_limits", {
        id: profile.connectionId,
        model: profile.model.trim(),
      })
        .then((limits) => {
          if (!active) return;
          if (limits) update({ modelLimits: limits });
          else setNotice(t("服务未提供此型号的容量，请按官方说明手动填写。"));
        })
        .catch((error: unknown) => {
          if (active) setError(String(error));
        })
        .finally(() => {
          if (active) setLoading(false);
        });
    }, 400);
    return () => {
      active = false;
      window.clearTimeout(timer);
    };
    // Only route changes or an explicit refresh may refetch; editing caps never does.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    profile.connectionId,
    profile.endpoint,
    profile.model,
    profile.adapter,
    refresh,
  ]);
  const source = !policy.automatic
    ? t("手动覆盖")
    : profile.adapter === "codex"
      ? t("Codex 运行时报告")
      : policy.limits?.source === "service"
        ? t("服务元数据")
        : policy.limits?.source === "catalog"
          ? t("官方模型目录")
          : t("容量未知，请刷新或手动填写");
  return (
    <>
      <div>
        <label>
          {t("上下文窗口（token）")}
          <input
            type="number"
            min={8192}
            max={2000000}
            step={1}
            disabled={policy.automatic}
            value={policy.window ?? ""}
            placeholder={t(
              profile.adapter === "codex" ? "运行时获取" : "填写服务公布的容量",
            )}
            onChange={(e) =>
              update({
                contextWindow: e.target.value ? Number(e.target.value) : null,
                contextWindowMode: "manual",
              })
            }
          />
        </label>
        <small>{t("来源：{v0}", { v0: source })}</small>
        <label className="model-context-auto">
          <input
            type="checkbox"
            checked={policy.automatic}
            onChange={(e) =>
              update({
                contextWindowMode: e.target.checked ? "auto" : "manual",
                contextWindow: e.target.checked
                  ? null
                  : (policy.window ?? null),
              })
            }
          />{" "}
          {t("自动获取上下文窗口")}
        </label>
        {profile.adapter !== "codex" && (
          <button
            type="button"
            disabled={loading || !profile.connectionId || !profile.model}
            onClick={() => setRefresh((n) => n + 1)}
          >
            {loading ? t("正在获取…") : t("刷新模型容量")}
          </button>
        )}
      </div>
      <label>
        {t("输出上限与预留（token）")}
        <input
          type="number"
          min={1}
          max={2000000}
          step={1}
          value={profile.maxOutputTokens ?? ""}
          required={
            profile.adapter === "anthropic-native" && policy.output == null
          }
          disabled={
            profile.adapter === "codex" && profile.maxOutputTokens == null
          }
          placeholder={String(policy.output ?? t("服务默认"))}
          onChange={(e) =>
            update({
              maxOutputTokens: e.target.value ? Number(e.target.value) : null,
            })
          }
        />
        <small>
          {t(
            profile.adapter === "codex"
              ? "Codex 的输出上限由本机服务管理"
              : "预留与实际请求的输出上限一致，留空使用模型默认值。",
          )}
        </small>
      </label>
      <label>
        {t("压缩余量（token）")}
        <input
          type="number"
          min={0}
          max={2000000}
          step={1}
          value={profile.compactionHeadroom ?? ""}
          placeholder="65536"
          onChange={(e) =>
            update({
              compactionHeadroom: e.target.value
                ? Number(e.target.value)
                : null,
            })
          }
        />
        <small>{t("预留给压缩摘要及请求增长，默认 65,536。")}</small>
      </label>
      <p className="model-hint model-field-wide" role="status">
        {policy.threshold == null
          ? t(
              profile.adapter === "codex"
                ? "压缩阈值将在服务报告窗口后计算。"
                : "上下文容量未知；刷新容量或手动填写后才能使用此模型。",
            )
          : t("预计压缩阈值：{v0} token", {
              v0: policy.threshold.toLocaleString(),
            })}{" "}
        {notice}
      </p>
      {error && <ErrorNotice error={error} fallback="VALIDATION_FAILED" />}
    </>
  );
}
