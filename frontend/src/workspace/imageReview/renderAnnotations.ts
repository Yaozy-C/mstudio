import { t } from "../../i18n";
import type { ImageComment } from "./annotations";

export async function renderAnnotations(
  src: string,
  comments: ImageComment[],
): Promise<File> {
  const image = new Image();
  image.crossOrigin = "anonymous";
  image.src = src;
  await image.decode();
  const ratio = Math.min(
    1,
    2560 / Math.max(image.naturalWidth, image.naturalHeight),
  );
  const canvas = document.createElement("canvas");
  canvas.width = Math.max(1, Math.round(image.naturalWidth * ratio));
  canvas.height = Math.max(1, Math.round(image.naturalHeight * ratio));
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error(t("无法创建批注图片"));
  ctx.drawImage(image, 0, 0, canvas.width, canvas.height);
  const unit = Math.max(14, Math.min(canvas.width, canvas.height) * 0.025);
  ctx.lineWidth = Math.max(2, unit / 6);
  ctx.font = `bold ${unit * 0.75}px sans-serif`;
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  comments.forEach((comment, index) => {
    const x = comment.x * canvas.width,
      y = comment.y * canvas.height;
    ctx.strokeStyle = "#e53935";
    ctx.strokeRect(
      x,
      y,
      comment.width * canvas.width,
      comment.height * canvas.height,
    );
    const cx = Math.max(unit / 2, Math.min(canvas.width - unit / 2, x));
    const cy = Math.max(unit / 2, Math.min(canvas.height - unit / 2, y));
    ctx.fillStyle = "#e53935";
    ctx.beginPath();
    ctx.arc(cx, cy, unit / 2, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = "white";
    ctx.fillText(String(index + 1), cx, cy);
  });
  const blob = await new Promise<Blob>((resolve, reject) =>
    canvas.toBlob(
      (value) =>
        value ? resolve(value) : reject(new Error(t("无法保存批注图片"))),
      "image/png",
    ),
  );
  return new File([blob], "image-comments.png", { type: "image/png" });
}
