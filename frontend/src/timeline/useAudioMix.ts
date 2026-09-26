import { useEffect, useState } from "react";
import { bridge, native } from "../bridge";
import type { Project } from "../model";
import { endTime, tracksOf } from "./document";
import type { PlaybackClock } from "./clock";
/** One cached audio stream avoids WebKit's repeated per-clip audio-session transitions. */
export function useAudioMix(project: Project, clock: PlaybackClock) {
  const [state, setState] = useState({
    key: "",
    path: null as string | null,
    error: "",
  });
  const spec = {
    clips: project.clips
      .filter((c) => !project.assets.find((a) => a.id === c.assetId)?.missing)
      .map(
        ({
          id,
          assetId,
          start,
          trackId,
          trimIn,
          trimOut,
          speed,
          volume,
          fadeIn,
          fadeOut,
        }) => ({
          id,
          assetId,
          start,
          trackId,
          trimIn,
          trimOut,
          speed,
          volume,
          fadeIn,
          fadeOut,
        }),
      ),
    tracks: tracksOf(project).map(({ id, kind, muted }) => ({
      id,
      kind,
      muted,
    })),
    captions: (project.captions ?? []).map((c) => ({
      start: c.start,
      end: c.end,
      text: "",
    })),
    width: project.width,
    height: project.height,
    fps: project.fps,
  };
  const key = JSON.stringify(spec);
  const pending = native && state.key !== key;
  useEffect(() => {
    if (!native) return;
    let alive = true;
    clock.pause();
    clock.ready = false;
    const timer = setTimeout(() => {
      void bridge<string | null>("prepare_audio_preview", {
        projectId: project.id,
        spec: JSON.parse(key),
      })
        .then((path) => {
          if (alive) {
            setState({ key, path, error: "" });
            clock.ready = true;
          }
        })
        .catch((e) => {
          if (alive)
            setState({
              key,
              path: null,
              error: `声音预览准备失败：${String(e)}`,
            });
        });
    }, 150);
    return () => {
      alive = false;
      clearTimeout(timer);
    };
  }, [key, clock, project.id]);
  useEffect(
    () => () => {
      clock.ready = true;
    },
    [clock],
  );
  return {
    path: pending ? null : state.path,
    pending,
    error: state.key === key ? state.error : "",
    total: endTime(project),
  };
}
