// Presentation only. These hints must never decide whether a job can be resubmitted.
export function inputHint(
  details = "",
): { message: string; recovery: string } | undefined {
  const ratio =
    /(?:reference_image_urls|image_url)[^\n]*aspect ratio[^\n]*between\s+(\d+(?:\.\d+)?)\s+and\s+(\d+(?:\.\d+)?)/i.exec(
      details,
    );
  if (ratio)
    return {
      message: "参考图片的宽高比不符合要求",
      recovery: `请更换或裁剪参考图片，宽高比须在 ${ratio[1]}～${ratio[2]} 之间。然后重新设置并生成。`,
    };
  const duration =
    /\bduration\b[^\n]*(?:between|range)\s+(\d+(?:\.\d+)?)\s+(?:and|to)\s+(\d+(?:\.\d+)?)/i.exec(
      details,
    );
  if (duration)
    return {
      message: "视频时长不符合模型要求",
      recovery: `请将视频时长调整到 ${duration[1]}～${duration[2]} 秒，再重新生成。`,
    };
  return undefined;
}
