import type { Asset } from "../../model";
import type { CreativeTask } from "../../creative/aiTasks";

export type Point = { x: number; y: number };
export type ImageComment = {
  id: string;
  x: number;
  y: number;
  width: number;
  height: number;
  text: string;
};
export function imagePoint(
  point: Point,
  viewport: { width: number; height: number },
  natural: { width: number; height: number },
  view: { scale: number; x: number; y: number },
): Point | null {
  if (!natural.width || !natural.height || view.scale <= 0) return null;
  const x =
    (point.x - viewport.width / 2 - view.x) / view.scale / natural.width + 0.5;
  const y =
    (point.y - viewport.height / 2 - view.y) / view.scale / natural.height +
    0.5;
  return x < 0 || x > 1 || y < 0 || y > 1 ? null : { x, y };
}
export function commentRegion(a: Point, b: Point) {
  const width = Math.max(0.025, Math.abs(b.x - a.x));
  const height = Math.max(0.025, Math.abs(b.y - a.y));
  return {
    x: Math.max(0, Math.min(1 - width, Math.min(a.x, b.x))),
    y: Math.max(0, Math.min(1 - height, Math.min(a.y, b.y))),
    width,
    height,
  };
}
export function imageReviewTask(
  original: Asset,
  marked: Asset,
  comments: ImageComment[],
): CreativeTask {
  return {
    agentId: "image",
    refs: [{ kind: "asset", id: original.id }],
    text: `请根据图片批注修改这张已有图片，创建图片编辑任务。\n原图素材 ID：${original.id}（${original.name}），这是编辑底图。\n内部批注说明图素材 ID：${marked.id}；先用 mstudio_read_image(assetId="${marked.id}") 读取批注位置。它附属于原图，仅用于定位修改区域，红框和编号不是图片内容，也不是严格的编辑遮罩；不要把批注图当作编辑底图。\n按编号理解修改意见，坐标以原图左上角为原点，单位为百分比：\n${comments.map((c, i) => `${i + 1}. 区域 x=${+(c.x * 100).toFixed(1)}%, y=${+(c.y * 100).toFixed(1)}%, 宽=${+(c.width * 100).toFixed(1)}%, 高=${+(c.height * 100).toFixed(1)}%：${c.text.trim()}`).join("\n")}\n保留未要求修改的产品结构、人物身份、构图、背景和光线；修改产生的必要接触、遮挡和阴影相应调整。生成任务的 references 只包含原图（role=edit）和用户提供的实际产品参考图；不要把内部批注说明图添加到生成任务的 references，也不要作为内容参考传给生成模型。将批注区域和意见落实到编辑描述。若当前模型支持，原图用作 edit 输入；不支持时说明限制。输出干净的新图片，不含批注框或编号，保留原图供对比和继续修改。`,
  };
}
