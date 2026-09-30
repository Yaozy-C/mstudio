import { captionTracksOf, captionTrackId } from "../timeline/captionTracks";
import { StudioSelect } from "../ui/StudioSelect";
import { CaptionAnimationFields } from "./CaptionAnimationFields";
import { useCaptionSelection } from "./captionSelection";
import { ActionButton } from "../ui/ActionButton";
import { InspectorControl } from "../timeline/InspectorControl";
import {
  Plus,
  UploadSimple,
  DownloadSimple,
  Trash,
  Crosshair,
} from "@phosphor-icons/react";
import { t, useLanguage } from "../i18n";
import { CaptionPresets } from "./CaptionPresets";
import { CaptionStyleFields } from "./CaptionStyleFields";
import { ErrorNotice } from "../errors/ErrorNotice";
import { useRef, useState } from "react";
import { bridge } from "../bridge";
import { uid, type Project, type Caption } from "../model";
import type { PlaybackClock } from "../timeline/clock";
import { parseSrt, toSrt, validCaption } from "./captions";
export function SubtitlePanel({
  project,
  change,
  clock,
}: {
  project: Project;
  change: (f: (p: Project) => Project) => void;
  clock: PlaybackClock;
}) {
  useLanguage();
  const input = useRef<HTMLInputElement>(null),
    [error, setError] = useState("");
  const captions = project.captions ?? [];
  const [selectedId, selectCaption] = useCaptionSelection(clock);
  const c = captions.find((item) => item.id === selectedId);
  const i = captions.findIndex((item) => item.id === selectedId);
  function patch(id: string, fields: Partial<Caption>) {
    change((p) => ({
      ...p,
      captions: (p.captions ?? []).map((c) =>
        c.id === id
          ? {
              ...c,
              ...fields,
              words:
                fields.text !== undefined ||
                fields.start !== undefined ||
                fields.end !== undefined
                  ? undefined
                  : c.words,
              assetId: undefined,
            }
          : c,
      ),
    }));
  }
  return (
    <div className="creation-form">
      <div className="subtitle-actions">
        <ActionButton
          icon={Plus}
          onClick={() => {
            const start = clock.getSnapshot().time;
            const id = uid();
            change((p) => ({
              ...p,
              captions: [
                ...(p.captions ?? []),
                {
                  id,
                  trackId: c ? captionTrackId(p, c) : captionTracksOf(p)[0].id,
                  start,
                  end: start + 3,
                  text: t("新字幕"),
                },
              ],
            }));
            selectCaption(id);
          }}
        >
          {t("添加字幕")}
        </ActionButton>
        <ActionButton
          icon={UploadSimple}
          onClick={() => input.current?.click()}
        >
          {t("导入 SRT")}
        </ActionButton>
        <ActionButton
          icon={DownloadSimple}
          disabled={!captions.length || captions.some((c) => !validCaption(c))}
          onClick={() =>
            void bridge("save_subtitles", { text: toSrt(captions) }).catch(
              (e) => setError(String(e)),
            )
          }
        >
          {t("导出 SRT")}
        </ActionButton>
      </div>
      <input
        hidden
        ref={input}
        type="file"
        accept=".srt"
        onChange={(e) => {
          const file = e.target.files?.[0];
          if (!file) return;
          e.target.value = "";
          if (file.size > 5_000_000) {
            setError(t("字幕文件过大"));
            return;
          }
          void file
            .text()
            .then((text) => {
              const added = parseSrt(text);
              change((p) => ({
                ...p,
                captions: [
                  ...(p.captions ?? []),
                  ...added.map((item) => ({
                    ...item,
                    trackId: c
                      ? captionTrackId(p, c)
                      : captionTracksOf(p)[0].id,
                  })),
                ],
              }));
              selectCaption(added[0]?.id ?? null);
              setError("");
            })
            .catch((e) => setError(String(e)));
        }}
      />
      {error && <ErrorNotice error={error} fallback="OPERATION_FAILED" />}
      {!captions.length && <p className="subtle">{t("暂无字幕")}</p>}
      {!!captions.length && !c && (
        <p className="subtitle-selection-empty">
          {t("请在时间线选中一条字幕，查看和调整它的设置。")}
        </p>
      )}
      {c && (
        <article className="caption-editor" key={c.id}>
          <div className="inline">
            <button
              onClick={() => {
                clock.pause();
                clock.seek(c.start);
              }}
            >
              <Crosshair size={20} />
              {String(i + 1).padStart(2, "0")}
            </button>
            <button
              className="text-button danger"
              aria-label={t("删除字幕 {number}", { number: i + 1 })}
              onClick={() =>
                change((p) => ({
                  ...p,
                  captions: p.captions?.filter((x) => x.id !== c.id),
                }))
              }
            >
              <Trash size={20} />
            </button>
          </div>
          <StudioSelect
            label={t("字幕轨道")}
            value={captionTrackId(project, c)}
            options={captionTracksOf(project).map((track) => ({
              value: track.id,
              label: track.name,
            }))}
            onValueChange={(trackId) => patch(c.id, { trackId })}
          />
          <InspectorControl
            label={t("开始")}
            value={c.start}
            unit="s"
            min={0}
            max={86400}
            step={1 / project.fps}
            change={(start) => patch(c.id, { start })}
          />
          <InspectorControl
            label={t("结束")}
            value={c.end}
            unit="s"
            min={0}
            max={86400}
            step={1 / project.fps}
            change={(end) => patch(c.id, { end })}
          />
          <textarea
            aria-label={t("字幕 {v0}", { v0: i + 1 })}
            maxLength={1000}
            rows={2}
            value={c.text}
            onChange={(e) => patch(c.id, { text: e.target.value })}
          />
          <CaptionAnimationFields
            width={project.width}
            height={project.height}
            caption={c}
            patch={(fields) => patch(c.id, fields)}
          />
          <details className="caption-library">
            <summary>{t("字幕模板")}</summary>
            <p className="subtle">{t("仅应用到当前选中的字幕。")}</p>
            <CaptionPresets caption={c} apply={(style) => patch(c.id, style)} />
          </details>
          <details className="caption-appearance">
            <summary>{t("字幕样式")}</summary>
            <CaptionStyleFields
              caption={c}
              width={project.width}
              height={project.height}
              patch={(fields) => patch(c.id, fields)}
            />
          </details>
          {!validCaption(c) && (
            <small className="error">
              {t("请填写文字，结束时间须大于开始时间。")}
            </small>
          )}
        </article>
      )}
    </div>
  );
}
