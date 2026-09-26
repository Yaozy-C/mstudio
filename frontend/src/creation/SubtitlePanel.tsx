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
  const input = useRef<HTMLInputElement>(null),
    [error, setError] = useState("");
  const captions = project.captions ?? [];
  function patch(id: string, fields: Partial<Caption>) {
    change((p) => ({
      ...p,
      captions: (p.captions ?? []).map((c) =>
        c.id === id ? { ...c, ...fields, assetId: undefined } : c,
      ),
    }));
  }
  return (
    <div className="creation-form">
      <div className="inline">
        <button
          onClick={() => {
            const start = clock.getSnapshot().time;
            change((p) => ({
              ...p,
              captions: [
                ...(p.captions ?? []),
                { id: uid(), start, end: start + 3, text: "新字幕" },
              ],
            }));
          }}
        >
          ＋当前时间字幕
        </button>
        <button onClick={() => input.current?.click()}>导入 SRT</button>
        <button
          disabled={!captions.length || captions.some((c) => !validCaption(c))}
          onClick={() =>
            void bridge("save_subtitles", { text: toSrt(captions) }).catch(
              (e) => setError(String(e)),
            )
          }
        >
          导出 SRT
        </button>
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
            setError("字幕文件过大");
            return;
          }
          void file
            .text()
            .then((text) => {
              const added = parseSrt(text);
              change((p) => ({
                ...p,
                captions: [...(p.captions ?? []), ...added],
              }));
              setError("");
            })
            .catch((e) => setError(String(e)));
        }}
      />
      {error && <ErrorNotice error={error} fallback="OPERATION_FAILED" />}
      {!captions.length && (
        <p className="subtle">
          添加字幕、导入已有 SRT，或在配音时同步生成。手动时间以秒为单位。
        </p>
      )}
      {captions.map((c, i) => (
        <article className="caption-editor" key={c.id}>
          <div className="inline">
            <button
              onClick={() => {
                clock.pause();
                clock.seek(c.start);
              }}
            >
              #{i + 1} 定位
            </button>
            <button
              className="text-button danger"
              onClick={() =>
                change((p) => ({
                  ...p,
                  captions: p.captions?.filter((x) => x.id !== c.id),
                }))
              }
            >
              删除
            </button>
          </div>
          <div className="field-grid">
            <label>
              开始 / 秒
              <input
                type="number"
                min={0}
                step={1 / project.fps}
                value={c.start}
                onChange={(e) => {
                  if (Number.isFinite(e.target.valueAsNumber))
                    patch(c.id, { start: Math.max(0, e.target.valueAsNumber) });
                }}
              />
            </label>
            <label>
              结束 / 秒
              <input
                type="number"
                min={0}
                step={1 / project.fps}
                value={c.end}
                onChange={(e) => {
                  if (Number.isFinite(e.target.valueAsNumber))
                    patch(c.id, { end: e.target.valueAsNumber });
                }}
              />
            </label>
          </div>
          <textarea
            aria-label={`字幕 ${i + 1}`}
            maxLength={1000}
            rows={2}
            value={c.text}
            onChange={(e) => patch(c.id, { text: e.target.value })}
          />
          {!validCaption(c) && (
            <small className="error">
              请填写文字，结束时间须大于开始时间。
            </small>
          )}
        </article>
      ))}
    </div>
  );
}
