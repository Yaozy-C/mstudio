import { useEffect, useRef, useState } from "react";
import type { Asset } from "../model";
import { bridge, native } from "../bridge";
import type { PlaybackClock } from "./clock";
const requests = new Map<string, Promise<string>>();
export function usePreviewSources(
  projectId: string,
  assets: (Asset | undefined)[],
  clock: PlaybackClock,
  smooth: boolean,
) {
  const [paths, setPaths] = useState<Record<string, string>>({});
  const [error, setError] = useState("");
  const ready = useRef<Record<string, string>>({});
  const ids = [
    ...new Set(
      assets.filter((a) => a?.kind === "video" && !a.missing).map((a) => a!.id),
    ),
  ]
    .sort()
    .join(",");
  useEffect(() => {
    setError("");
    let alive = true;
    const publish = () => {
      if (alive && !clock.getSnapshot().playing)
        setPaths((p) =>
          Object.keys(ready.current).some((id) => p[id] !== ready.current[id])
            ? { ...ready.current }
            : p,
        );
    };
    if (native && smooth)
      for (const id of ids.split(",").filter(Boolean)) {
        const key = `${projectId}:${id}`;
        let request = requests.get(key);
        if (!request) {
          request = bridge<string>("prepare_preview", {
            assetId: id,
            projectId,
          });
          requests.set(key, request);
          request.catch(() => requests.delete(key));
        }
        void request
          .then((path) => {
            if (alive) {
              ready.current[id] = path;
              publish();
            }
          })
          .catch((e) => {
            if (alive) setError(`代理准备失败，继续使用原片：${String(e)}`);
          });
      }
    publish();
    const off = clock.subscribe(publish);
    return () => {
      alive = false;
      off();
    };
  }, [projectId, ids, clock, smooth]);
  return {
    paths: smooth ? paths : {},
    error,
    pending:
      native &&
      smooth &&
      assets.some((a) => a?.kind === "video" && !a.missing && !paths[a.id]),
  };
}
