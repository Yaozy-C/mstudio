import type { Project, Clip } from "../model";
import { ClipFields } from "../timeline/ClipFields";
import { PlacementFields } from "../timeline/PlacementFields";
type Props = {
  project: Project;
  clipId: string | null;
  onChange: (fn: (p: Project) => Project) => void;
};
export function Inspector({ project, clipId, onChange }: Props) {
  const clip = project.clips.find((c) => c.id === clipId);
  const asset = project.assets.find((a) => a.id === clip?.assetId);
  const updateClip = (updated: Clip) =>
    onChange((p) => ({
      ...p,
      clips: p.clips.map((c) => (c.id === updated.id ? updated : c)),
    }));
  return (
    <div className="inspector">
      <header>
        <h3>{clip ? "片段属性" : "项目设置"}</h3>
      </header>
      {clip ? (
        <>
          <h4>{asset?.name}</h4>
          <PlacementFields clip={clip} project={project} update={updateClip} />
          <ClipFields
            clip={clip}
            assetDuration={asset?.duration || 0}
            image={asset?.kind === "image"}
            onUpdate={updateClip}
          />
        </>
      ) : (
        <>
          <label>
            项目名称
            <input
              value={project.name}
              onChange={(e) =>
                onChange((p) => ({ ...p, name: e.target.value }))
              }
            />
          </label>
          <label>
            画面比例
            <select
              value={`${project.width}x${project.height}`}
              onChange={(e) => {
                const [width, height] = e.target.value.split("x").map(Number);
                onChange((p) => ({ ...p, width, height }));
              }}
            >
              <option value="1080x1920">9:16 · 竖屏</option>
              <option value="1920x1080">16:9 · 横屏</option>
              <option value="1080x1080">1:1 · 方形</option>
              <option value="720x1280">9:16 · 720p</option>
            </select>
          </label>
          <label>
            帧率
            <select
              value={project.fps}
              onChange={(e) =>
                onChange((p) => ({ ...p, fps: +e.target.value }))
              }
            >
              {[24, 25, 30, 60].map((v) => (
                <option key={v}>{v}</option>
              ))}
            </select>
          </label>
          <label>
            创作要求与已确认事实
            <textarea
              rows={5}
              placeholder="视频目标、受众、风格、锁定要求…"
              value={project.brief}
              onChange={(e) =>
                onChange((p) => ({ ...p, brief: e.target.value }))
              }
            />
          </label>
          <p className="subtle">
            精确剪辑请选中时间线片段。画布内容可以直接引用给 Agent。
          </p>
        </>
      )}
    </div>
  );
}
