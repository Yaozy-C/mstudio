import { mediaUrl } from "../bridge";
import { t } from "../i18n";
import type { PrevisImage } from "./types";

export function PrevisMessageImages({
  images = [],
}: {
  images?: PrevisImage[];
}) {
  if (!images.length) return null;
  return (
    <div className="previs-message-images" aria-label={t("已发送的参考图片")}>
      {images.map((image) => (
        <figure key={image.id} title={image.name}>
          <img
            src={mediaUrl(image.preview || image.path)}
            alt={image.name}
            loading="lazy"
          />
          <figcaption>{image.name}</figcaption>
        </figure>
      ))}
    </div>
  );
}
