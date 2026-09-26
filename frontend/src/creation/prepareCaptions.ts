import type { Caption } from "../model";
import { captionKey } from "./captionStyle";
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
    const key = captionKey(c);
    const assetId =
      cached.get(key) ?? (await store(captionImage(c, width, height)));
    cached.set(key, assetId);
    result.push({ ...c, assetId });
  }
  return result;
}
