import { animationPlan } from "./captionAnimation";
import type { Caption } from "../model";
import { captionKey } from "./captionStyle";
import { captionImage, validCaption } from "./captions";
export async function prepareCaptions(
  captions: Caption[],
  width: number,
  height: number,
  store: (data: string) => Promise<string>,
  animation?: {
    fps: number;
    store: (
      frames: { data: string; duration: number }[],
      duration: number,
      loop: boolean,
    ) => Promise<string>;
    cancelled?: () => boolean;
  },
) {
  const result: Caption[] = [];
  const cached = new Map<string, string>();
  for (const c of captions) {
    if (!validCaption(c)) throw new Error("请先修正字幕的文字与时间");
    if (animation?.cancelled?.()) throw new Error("字幕准备已取消");
    if (c.animation && c.animation !== "none") {
      if (!animation) throw new Error("动态字幕需要动画渲染器");
      const plan = animationPlan(c, animation.fps);
      const frames: { data: string; duration: number }[] = [];
      for (const frame of plan.frames) {
        if (animation.cancelled?.()) throw new Error("字幕准备已取消");
        frames.push({
          data: captionImage(c, width, height, frame.time),
          duration: frame.duration,
        });
        if (frames.length % 8 === 0)
          await new Promise((resolve) => setTimeout(resolve, 0));
      }
      if (animation.cancelled?.()) throw new Error("字幕准备已取消");
      const assetId = await animation.store(frames, c.end - c.start, plan.loop);
      result.push({ ...c, assetId });
      continue;
    }
    const key = captionKey(c);
    const assetId =
      cached.get(key) ?? (await store(captionImage(c, width, height)));
    cached.set(key, assetId);
    result.push({ ...c, assetId });
  }
  return result;
}
