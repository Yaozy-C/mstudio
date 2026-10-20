import { useId, useState } from "react";
import {
  ArrowRight,
  CaretDown,
  SlidersHorizontal,
  Cpu,
} from "@phosphor-icons/react";
import { native } from "../../bridge";
import { t } from "../../i18n";
import { StudioSelect } from "../../ui/StudioSelect";
import type { ModelConnection } from "../../models/types";
import type { AnalysisState } from "./types";
import type { AnalysisController } from "./controller";

export function AnalysisRequest({
  state,
  controller,
  models,
  selected,
  select,
  analyze,
  onSettings,
}: {
  state: AnalysisState;
  controller: AnalysisController;
  models: ModelConnection[];
  selected: string;
  select: (id: string) => void;
  analyze: () => void;
  onSettings: () => void;
}) {
  const busy = !!state.busy || !!state.openingRecord;
  const [open, setOpen] = useState(false);
  const optionsId = useId();
  return (
    <div className="analysis-request">
      <div className="analysis-request-toolbar">
        <button
          className="analysis-options-toggle"
          aria-expanded={open}
          aria-controls={open ? optionsId : undefined}
          onClick={() => setOpen(!open)}
        >
          <SlidersHorizontal size={16} aria-hidden="true" />
          {t("拆解设置")}
          <CaretDown size={12} aria-hidden="true" />
        </button>
        {!busy && (
          <button
            className="primary"
            disabled={!native || !selected || state.video?.missing}
            onClick={analyze}
          >
            {state.result ? t("重新拆解") : t("开始拆解")}
            <ArrowRight size={18} aria-hidden="true" />
          </button>
        )}
      </div>
      {open && (
        <div className="analysis-options-content" id={optionsId}>
          <div className="analysis-focus-heading">
            <label htmlFor="analysis-question">{t("拆解重点")}</label>
            <span>{t("选填")}</span>
          </div>
          <textarea
            id="analysis-question"
            rows={2}
            maxLength={2000}
            placeholder={t("例如，重点看开场钩子、剪辑节奏或声音设计。")}
            value={state.question}
            disabled={busy}
            onChange={(event) => controller.setQuestion(event.target.value)}
          />
          <div className="analysis-model-row">
            <span className="analysis-model-caption">
              <Cpu size={15} aria-hidden="true" />
              {t("分析模型")}
            </span>
            <div className="analysis-model-control">
              <StudioSelect
                label={t("视频分析模型")}
                placeholder={t("选择视频模型")}
                value={selected}
                onValueChange={select}
                options={models.map((model) => ({
                  value: model.id,
                  label: model.name,
                }))}
                disabled={busy || !models.length}
              />
            </div>
            {native && !models.length && (
              <button className="analysis-setup-model" onClick={onSettings}>
                {t("设置视频模型")}
                <ArrowRight size={13} aria-hidden="true" />
              </button>
            )}
          </div>
        </div>
      )}
      {native && !selected && !open && (
        <button className="analysis-setup-model" onClick={onSettings}>
          {t("设置视频模型")}
          <ArrowRight size={14} aria-hidden="true" />
        </button>
      )}
      {!native && (
        <p className="analysis-preview-note">
          {t("可在此预览本地视频，AI 拆解请使用桌面应用。")}
        </p>
      )}
    </div>
  );
}
