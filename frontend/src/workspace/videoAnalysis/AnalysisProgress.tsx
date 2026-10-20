import { useEffect, useState } from "react";
import { Check } from "@phosphor-icons/react";
import { t } from "../../i18n";
import type { AnalysisProgress as Progress } from "./types";

const steps = ["读取视频", "准备分析", "声画拆解", "整理结果"];
const stages = {
  selecting: [0, "选择参考视频", "在文件窗口中选择要拆解的视频。"],
  connecting: [0, "正在连接视频来源", "连接成功后，会在这里显示下载进度。"],
  downloading: [
    0,
    "正在下载原视频",
    "视频和音频可能分轨下载，进度按当前轨道显示。",
  ],
  copying: [0, "正在读取本地视频", "保留原视频，准备播放和拆解。"],
  checking: [0, "正在检查视频", "读取时长、画面尺寸与音轨。"],
  encoding: [1, "正在准备分析视频", "优化模型输入，保留完整时长与音轨。"],
  analyzing: [2, "正在拆解画面与声音", "梳理逐段声画，理解作品的创意与表达。"],
  saving: [3, "正在保存拆解记录", "保存原视频与拆解结果，方便下次继续回看。"],
  formatting: [3, "正在整理拆解结果", "检查时间码，整理逐段声画与创作方法。"],
} as const;
function bytes(value: number) {
  return `${(value / 1024 ** 2).toFixed(1)} MB`;
}
export function AnalysisProgress({
  progress,
  cancel,
  standalone = false,
}: {
  progress: Progress;
  cancel: () => void;
  standalone?: boolean;
}) {
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, []);
  const [step, title, description] = stages[progress.stage];
  const elapsed = Math.max(0, Math.floor((now - progress.startedAt) / 1000));
  const percentage =
    progress.total && progress.completed != null
      ? Math.min(100, Math.floor((progress.completed / progress.total) * 100))
      : undefined;
  const counter =
    progress.stage === "downloading" && progress.completed != null
      ? progress.fragments
        ? t("{done} / {total} 个片段", {
            done: progress.completed,
            total: progress.total ?? "—",
          })
        : `${bytes(progress.completed)}${progress.total ? ` / ${bytes(progress.total)}` : ""}`
      : undefined;
  return (
    <section
      className={`analysis-progress${standalone ? " is-standalone" : ""}`}
      aria-label={t("视频拆解进度")}
    >
      <div className="analysis-progress-topline">
        <span className="analysis-eyebrow">{t("视频拆解进度")}</span>
        <button onClick={cancel}>{t("停止")}</button>
      </div>
      <div
        className="analysis-progress-announcement"
        role="status"
        aria-live="polite"
      >
        <p className="analysis-progress-step">
          {t("第 {step} 步，共 4 步", { step: step + 1 })}
        </p>
        <h2>{t(title)}</h2>
        <p>{t(description)}</p>
      </div>
      <div className="analysis-progress-meter-heading" aria-hidden="true">
        <span>{counter ?? t(steps[step])}</span>
        <span>{percentage != null ? `${percentage}%` : t("进行中")}</span>
      </div>
      <div
        className="analysis-progress-track"
        role="progressbar"
        aria-label={t(title)}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={percentage}
        aria-valuetext={percentage != null ? `${percentage}%` : t("进行中")}
        data-indeterminate={percentage == null}
      >
        <span
          style={percentage != null ? { width: `${percentage}%` } : undefined}
        />
      </div>
      <ol className="analysis-progress-steps" aria-label={t("拆解步骤")}>
        {steps.map((label, index) => (
          <li
            key={label}
            data-state={
              index < step ? "done" : index === step ? "active" : "pending"
            }
            aria-current={index === step ? "step" : undefined}
          >
            <span>
              {index < step ? (
                <Check size={12} weight="bold" aria-hidden="true" />
              ) : (
                String(index + 1).padStart(2, "0")
              )}
            </span>
            {t(label)}
          </li>
        ))}
      </ol>
      <div className="analysis-progress-footer">
        <span>
          {t("已用时 {time}", {
            time: `${Math.floor(elapsed / 60)
              .toString()
              .padStart(2, "0")}:${(elapsed % 60).toString().padStart(2, "0")}`,
          })}
        </span>
        <p>
          {t(
            elapsed >= 45 && step === 2
              ? "模型仍在分析，较长视频需要更多时间。"
              : "可以继续观看视频，完成后结果会出现在这里。",
          )}
        </p>
      </div>
    </section>
  );
}
