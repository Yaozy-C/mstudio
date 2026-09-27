import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { useEffect, useState } from "react";
import { bridge } from "../bridge";
import { uid, type Asset, type Project } from "../model";
import { appendAsset, tracksOf } from "../timeline/document";
import type { PlaybackClock } from "../timeline/clock";
export function SpeechPanel({
  project,
  change,
  clock,
}: {
  project: Project;
  change: (f: (p: Project) => Project) => void;
  clock: PlaybackClock;
}) {
  useLanguage();
  const [voices, setVoices] = useState<{ name: string; language: string }[]>(
    [],
  );
  const [voice, setVoice] = useState("");
  const [text, setText] = useState("");
  const [rate, setRate] = useState(180);
  const [track, setTrack] = useState(
    tracksOf(project).find((t) => t.kind === "audio")?.id ?? "",
  );
  const [captions, setCaptions] = useState(true);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState<unknown>();
  useEffect(() => {
    void bridge<typeof voices>("list_voices")
      .then((v) => {
        setVoices(v);
        setVoice(
          v.find((x) => x.language.startsWith("zh"))?.name ?? v[0]?.name ?? "",
        );
      })
      .catch(setError);
  }, []);
  async function generate() {
    setError(undefined);
    const lines = text
      .split(/\n+/)
      .map((s) => s.trim())
      .filter(Boolean);
    if (!lines.length || lines.length > 50 || text.length > 8000) {
      setMessage(t("每次最多 50 段、8000 字"));
      return;
    }
    setBusy(true);
    clock.pause();
    let start = clock.getSnapshot().time;
    try {
      for (let i = 0; i < lines.length; i++) {
        setMessage(
          t("正在生成第 {v0}/{v1} 段…", { v0: i + 1, v1: lines.length }),
        );
        const asset = await bridge<Asset>("generate_voice", {
          projectId: project.id,
          text: lines[i],
          voice,
          rate,
        });
        asset.name = t("配音 · {v0}", { v0: lines[i].slice(0, 18) });
        const at = start,
          line = lines[i];
        change((p) => {
          const next = appendAsset(
            { ...p, assets: [...p.assets, asset] },
            asset,
            at,
            track,
          );
          return captions
            ? {
                ...next,
                captions: [
                  ...(next.captions ?? []),
                  {
                    id: uid(),
                    start: at,
                    end: at + asset.duration,
                    text: line,
                  },
                ],
              }
            : next;
        });
        start += asset.duration;
      }
      setMessage(t("已加入时间线；字幕按每段实际配音时长对齐，可继续调整。"));
    } catch (e) {
      setError(e);
      setMessage(t("已完成的配音片段保留在时间线。"));
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="creation-form">
      <p className="subtle">
        {t("本机配音 · 每行一段。分段生成后，字幕与每段实际声音长度对齐。")}
      </p>
      <label>
        {t("配音稿")}
        <textarea
          rows={5}
          value={text}
          maxLength={8000}
          onChange={(e) => setText(e.target.value)}
          placeholder={t("输入台词，每行对应一段配音")}
        />
      </label>
      <label>
        {t("声音")}
        <select value={voice} onChange={(e) => setVoice(e.target.value)}>
          {voices.map((v) => (
            <option key={v.name} value={v.name}>
              {v.name} · {v.language}
            </option>
          ))}
        </select>
      </label>
      <div className="field-grid">
        <label>
          {t("语速")}
          <input
            type="number"
            min={80}
            max={450}
            value={rate}
            onChange={(e) => setRate(e.target.valueAsNumber)}
          />
        </label>
        <label>
          {t("目标音轨")}
          <select value={track} onChange={(e) => setTrack(e.target.value)}>
            {tracksOf(project)
              .filter((t) => t.kind === "audio")
              .map((t) => (
                <option key={t.id} value={t.id}>
                  {t.name}
                </option>
              ))}
          </select>
        </label>
      </div>
      <label className="check-row">
        <input
          type="checkbox"
          checked={captions}
          onChange={(e) => setCaptions(e.target.checked)}
        />
        {t("同时创建分段字幕")}
      </label>
      <button
        className="primary wide"
        disabled={
          busy ||
          !voice ||
          !text.trim() ||
          !Number.isFinite(rate) ||
          rate < 80 ||
          rate > 450
        }
        onClick={() => void generate()}
      >
        {busy ? t("正在配音…") : t("从播放头开始加入配音")}
      </button>
      <ErrorNotice error={error} />
      {message && (
        <p role="status" className="subtle">
          {message}
        </p>
      )}
    </div>
  );
}
