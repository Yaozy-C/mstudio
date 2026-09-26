import { useEffect, useMemo, useState } from "react";
import { bridge, native } from "../bridge";
import type { Asset, Clip } from "../model";
import "../styles/waveform.css";
const cache = new Map<string, Promise<number[]>>();
export function waveformPath(
  peaks: number[],
  clip: Pick<Clip, "trimIn" | "trimOut" | "volume">,
  duration: number,
) {
  let path = "";
  const first = (clip.trimIn / duration) * peaks.length,
    last = (clip.trimOut / duration) * peaks.length;
  for (let x = 0; x < 512; x++) {
    const from = Math.max(0, Math.floor(first + ((last - first) * x) / 512));
    const to = Math.min(
      peaks.length,
      Math.max(from + 1, Math.ceil(first + ((last - first) * (x + 1)) / 512)),
    );
    let peak = 0;
    for (let i = from; i < to; i++) peak = Math.max(peak, peaks[i] ?? 0);
    const h = Math.max(0.3, Math.sqrt(peak) * 14 * clip.volume);
    path += `M${x},${16 - h}v${2 * h}`;
  }
  return path;
}
export function Waveform({
  asset,
  clip,
  projectId,
}: {
  asset: Asset;
  clip: Clip;
  projectId: string;
}) {
  const [peaks, setPeaks] = useState<number[] | null>(null);
  const [error, setError] = useState(false);
  useEffect(() => {
    let alive = true;
    setError(false);
    setPeaks(null);
    if (!native) return;
    const key = `${projectId}:${asset.id}:${asset.path}:${asset.duration}`;
    let result = cache.get(key);
    if (!result) {
      result = bridge<number[]>("audio_waveform", {
        projectId,
        assetId: asset.id,
      });
      if (cache.size >= 64) cache.delete(cache.keys().next().value!);
      cache.set(key, result);
      void result.catch(() => cache.delete(key));
    }
    void result
      .then((p) => {
        if (alive) setPeaks(p);
      })
      .catch(() => {
        if (alive) setError(true);
      });
    return () => {
      alive = false;
    };
  }, [asset.id, asset.path, asset.duration, projectId]);
  const path = useMemo(
    () => (peaks ? waveformPath(peaks, clip, asset.duration) : ""),
    [peaks, clip.trimIn, clip.trimOut, clip.volume, asset.duration],
  );
  if (!peaks)
    return (
      <div className="waveform-status">
        {error
          ? "波形读取失败"
          : native
            ? "正在分析波形…"
            : "桌面版显示音频波形"}
      </div>
    );
  return (
    <svg
      className="audio-waveform"
      viewBox="0 0 512 32"
      preserveAspectRatio="none"
      role="img"
      aria-label="音频波形"
    >
      <path d={path} />
    </svg>
  );
}
