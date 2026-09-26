import { type Clip, type Project } from "../model";
import { tracksOf, validateClip } from "./document";
export function PlacementFields({
  clip,
  project,
  update,
  section,
}: {
  clip: Clip;
  project: Project;
  update: (c: Clip) => void;
  section?: "time" | "visual" | "audio";
}) {
  const asset = project.assets.find((a) => a.id === clip.assetId);
  if (!asset) return null;
  const track = tracksOf(project).find((t) => t.id === clip.trackId);
  const numeric = (key: keyof Clip, value: number) => {
    const next = { ...clip, [key]: value };
    if (validateClip(next, asset)) update(next);
  };
  return (
    <>
      {(!section || section === "time") && (
        <div className="field-grid">
          <label>
            时间线起点 / 秒
            <input
              type="number"
              min={0}
              step={1 / project.fps}
              value={Number((clip.start ?? 0).toFixed(3))}
              onChange={(e) => numeric("start", e.target.valueAsNumber)}
            />
          </label>
          <label>
            轨道
            <select
              value={clip.trackId}
              onChange={(e) => update({ ...clip, trackId: e.target.value })}
            >
              {tracksOf(project)
                .filter((t) =>
                  t.kind === "video" ? asset.kind !== "audio" : asset.hasAudio,
                )
                .map((t) => (
                  <option key={t.id} value={t.id}>
                    {t.name}
                  </option>
                ))}
            </select>
          </label>
        </div>
      )}
      {(!section || section === "visual") && track?.kind === "video" && (
        <>
          <div className="field-grid">
            <label>
              中心 X / %
              <input
                type="number"
                min={-100}
                max={200}
                value={Math.round((clip.x ?? 0.5) * 100)}
                onChange={(e) => numeric("x", e.target.valueAsNumber / 100)}
              />
            </label>
            <label>
              中心 Y / %
              <input
                type="number"
                min={-100}
                max={200}
                value={Math.round((clip.y ?? 0.5) * 100)}
                onChange={(e) => numeric("y", e.target.valueAsNumber / 100)}
              />
            </label>
          </div>
          <label>
            画面大小 · {Math.round((clip.scale ?? 1) * 100)}%
            <input
              type="range"
              min={0.05}
              max={4}
              step={0.05}
              value={clip.scale ?? 1}
              onChange={(e) => numeric("scale", e.target.valueAsNumber)}
            />
          </label>
          <label>
            不透明度 · {Math.round((clip.opacity ?? 1) * 100)}%
            <input
              type="range"
              min={0}
              max={1}
              step={0.05}
              value={clip.opacity ?? 1}
              onChange={(e) => numeric("opacity", e.target.valueAsNumber)}
            />
          </label>
        </>
      )}
      {(!section || section === "audio") && asset.hasAudio && (
        <div className="field-grid">
          <label>
            声音淡入 / 秒
            <input
              type="number"
              min={0}
              step={0.1}
              value={clip.fadeIn ?? 0}
              onChange={(e) => numeric("fadeIn", e.target.valueAsNumber)}
            />
          </label>
          <label>
            声音淡出 / 秒
            <input
              type="number"
              min={0}
              step={0.1}
              value={clip.fadeOut ?? 0}
              onChange={(e) => numeric("fadeOut", e.target.valueAsNumber)}
            />
          </label>
        </div>
      )}
    </>
  );
}
