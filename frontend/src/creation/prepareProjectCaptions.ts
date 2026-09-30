import { orderedCaptions } from "../timeline/captionTracks";
import { validCaption } from "./captions";
import { bridge } from "../bridge";
import type { Project } from "../model";
import { captionKey } from "./captionStyle";
import { prepareCaptions } from "./prepareCaptions";
const cache = new Map<string, string>();
/** Cache asset IDs, never full-frame base64 images, across native preview and export. */
export async function prepareProjectCaptions(
  project: Pick<
    Project,
    "id" | "captions" | "captionTracks" | "width" | "height" | "fps"
  >,
  cancelled?: () => boolean,
) {
  const result = [];
  for (const c of project.captionTracks
    ? orderedCaptions(project)
    : project.captions) {
    if (!validCaption(c)) throw new Error("请先修正字幕的文字与时间");
    if (cancelled?.()) throw new Error("字幕准备已取消");
    const key = JSON.stringify([
      project.id,
      project.width,
      project.height,
      project.fps,
      captionKey(c),
      c.animation,
      c.highlightColor,
      c.words,
      c.end - c.start,
    ]);
    let assetId = cache.get(key);
    if (!assetId) {
      const [prepared] = await prepareCaptions(
        [c],
        project.width,
        project.height,
        (data) =>
          bridge<string>("store_caption_image", {
            projectId: project.id,
            data,
          }),
        {
          fps: project.fps,
          cancelled,
          store: (frames, duration, loop) =>
            bridge<string>("store_caption_animation", {
              projectId: project.id,
              frames,
              fps: project.fps,
              duration,
              loopAnimation: loop,
            }),
        },
      );
      assetId = prepared.assetId!;
      if (cache.size >= 128) cache.delete(cache.keys().next().value!);
      cache.set(key, assetId);
    }
    result.push({ ...c, assetId });
  }
  return result;
}
