import { ActionButton } from "../ui/ActionButton";
import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { useEffect, useState } from "react";
import { InspectorControl } from "../timeline/InspectorControl";
import { Gauge, SpeakerHigh, Stack, ArrowRight } from "@phosphor-icons/react";
import { StudioSelect } from "../ui/StudioSelect";
import { bridge, native } from "../bridge";
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
  const language = useLanguage();
  const [voices, setVoices] = useState<{ name: string; language: string }[]>(
    [],
  );
  const [loading, setLoading] = useState(true);
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
          v.find((x) =>
            x.language.startsWith(language === "zh-CN" ? "zh" : "en"),
          )?.name ??
            v[0]?.name ??
            "",
        );
      })
      .catch(setError)
      .finally(() => setLoading(false));
  }, []);
  const audioTracks = tracksOf(project).filter((item) => item.kind === "audio");
  const selectedTrack = audioTracks.some((item) => item.id === track)
    ? track
    : (audioTracks[0]?.id ?? "");
  async function generate() {
    if (busy || !voice || !native) return;
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
            selectedTrack,
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
    <div className="creation-form speech-form" aria-busy={busy}>
      <div className="speech-source">
        <strong>{t("系统配音")}</strong>
        <span>macOS</span>
      </div>
      <fieldset disabled={busy}>
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
        <div className="speech-text-meta">
          <span>{t("每行一段")}</span>
          <span>{text.length} / 8000</span>
        </div>
        <label className="inspector-select-row">
          <span>
            <SpeakerHigh size={20} />
            {t("声音")}
          </span>
          <StudioSelect
            label={t("声音")}
            value={voice}
            onValueChange={setVoice}
            disabled={loading || busy || !voices.length}
            placeholder={loading ? t("正在加载声音…") : t("没有可用声音")}
            options={voices.map((v) => ({ value: v.name, label: v.name }))}
          />
        </label>
        <InspectorControl
          label={t("语速")}
          icon={Gauge}
          value={rate}
          unit={t("字/分钟")}
          min={80}
          max={450}
          step={10}
          slider
          change={setRate}
        />
        <label className="inspector-select-row">
          <span>
            <Stack size={20} />
            {t("目标音轨")}
          </span>
          <StudioSelect
            label={t("目标音轨")}
            value={selectedTrack}
            onValueChange={setTrack}
            disabled={busy}
            options={
              audioTracks.length
                ? audioTracks.map((item) => ({
                    value: item.id,
                    label: item.name,
                  }))
                : [{ value: "", label: t("自动创建音轨") }]
            }
          />
        </label>
        <label className="check-row">
          <input
            type="checkbox"
            checked={captions}
            onChange={(e) => setCaptions(e.target.checked)}
          />
          {t("同时生成字幕")}
        </label>
      </fieldset>
      <div className="speech-submit">
        <ActionButton
          icon={ArrowRight}
          className="speech-generate"
          disabled={
            busy ||
            !native ||
            !voice ||
            !text.trim() ||
            !Number.isFinite(rate) ||
            rate < 80 ||
            rate > 450
          }
          onClick={() => void generate()}
        >
          {busy ? t("正在配音…") : t("生成并加入时间线")}
        </ActionButton>
        <p className="subtle">
          {!text.trim() ? t("填写配音稿后即可生成") : t("从当前播放位置插入")}
        </p>
      </div>
      <ErrorNotice error={error} />
      {message && (
        <p role="status" className="subtle">
          {message}
        </p>
      )}
    </div>
  );
}
