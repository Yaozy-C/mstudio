import type { Caption } from "../model";
import { captionImage, validCaption } from "./captions";
export async function prepareCaptions(
  captions: Caption[],
  width: number,
  height: number,
  store: (data: string) => Promise<string>,
) {
  const result: Caption[] = [];
  const cached = new Map<string, string>();
  for (const c of captions) {
    if (!validCaption(c)) throw new Error("请先修正字幕的文字与时间");
    const assetId =
      cached.get(c.text) ?? (await store(captionImage(c.text, width, height)));
    cached.set(c.text, assetId);
    result.push({ ...c, assetId });
  }
  return result;
}
