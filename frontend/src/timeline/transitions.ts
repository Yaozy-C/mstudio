import { duration, type Clip, type Project } from "../model";
import { readDesign, type TransitionDesign } from "./transitionDesign";
export const transitionKinds = {
  fade: "叠化",
  fadeblack: "淡入黑场",
  fadewhite: "淡入白场",
  wipeleft: "向左擦除",
  wiperight: "向右擦除",
  slideleft: "向左推移",
  slideright: "向右推移",
  smoothleft: "平滑左移",
  smoothright: "平滑右移",
  circleopen: "圆形展开",
  circleclose: "圆形收拢",
  dissolve: "颗粒溶解",
  custom: "自定义组合",
};
export type Transition = {
  design?: TransitionDesign;
  fromClipId: string;
  kind: keyof typeof transitionKinds;
  duration: number;
};
export function seams(p: Project, trackId: string) {
  const clips = p.clips
    .filter((c) => c.trackId === trackId)
    .sort((a, b) => (a.start ?? 0) - (b.start ?? 0));
  return clips
    .slice(1)
    .flatMap((right, i) =>
      Math.abs(
        (clips[i].start ?? 0) + duration(clips[i]) - (right.start ?? 0),
      ) <=
      0.5 / p.fps
        ? [{ left: clips[i], right }]
        : [],
    );
}
export function setTransition(
  p: Project,
  from: string,
  to: string,
  kind: unknown,
  seconds: unknown,
  design?: unknown,
): Project {
  const right = p.clips.find((c) => c.id === to);
  if (!right) throw new Error("转场目标片段不存在");
  if (kind === null)
    return {
      ...p,
      clips: p.clips.map((c) =>
        c.id === to ? { ...c, transition: undefined } : c,
      ),
    };
  const base = p.tracks.find((t) => t.kind === "video");
  const pair = seams(p, right.trackId!).find(
    (s) => s.left.id === from && s.right.id === to,
  );
  if (!pair || right.trackId !== base?.id)
    throw new Error("请选择底层画面轨上两个相邻片段的接缝");
  if (typeof kind !== "string" || !Object.hasOwn(transitionKinds, kind))
    throw new Error("不支持的转场类型");
  const recipe = kind === "custom" ? readDesign(design) : undefined;
  if (kind !== "custom" && design !== undefined)
    throw new Error("只有自定义转场接受 design");
  if (
    typeof seconds !== "number" ||
    !Number.isFinite(seconds) ||
    seconds < 0.05 ||
    seconds > Math.min(3, duration(pair.left), duration(right))
  )
    throw new Error("转场须为 0.05–3 秒，且不超过相邻片段时长");
  const full = (c: Clip) =>
    (c.x ?? 0.5) === 0.5 &&
    (c.y ?? 0.5) === 0.5 &&
    (c.scale ?? 1) === 1 &&
    (c.opacity ?? 1) === 1;
  if (!full(pair.left) || !full(right))
    throw new Error("转场目前需要全幅、不透明的画面，请恢复位置与缩放");
  if (
    p.clips.some(
      (c) =>
        c.trackId === right.trackId &&
        c.id !== from &&
        c.id !== to &&
        (c.start ?? 0) < (right.start ?? 0) + seconds / 2 &&
        (c.start ?? 0) + duration(c) > (right.start ?? 0) - seconds / 2,
    )
  )
    throw new Error("接缝处存在重叠片段");
  return {
    ...p,
    clips: p.clips.map((c) =>
      c.id === to
        ? {
            ...c,
            transition: {
              fromClipId: from,
              kind: kind as Transition["kind"],
              duration: seconds,
              ...(recipe ? { design: recipe } : {}),
            },
          }
        : c,
    ),
  };
}
