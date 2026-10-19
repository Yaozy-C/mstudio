import { useRef, useState } from "react";
import { Popover } from "@radix-ui/themes";
import { Check, Image, UploadSimple, X } from "@phosphor-icons/react";
import { mediaUrl } from "../bridge";
import { t } from "../i18n";
import type { Project } from "../model";
import { isSelectableAsset } from "../workspace/assetLibrary";
import type { PrevisImages } from "./usePrevisImages";
import "../styles/reference-picker.css";

export function PrevisImagePicker({
  project,
  images,
  disabled,
}: {
  project: Project;
  images: PrevisImages;
  disabled: boolean;
}) {
  const input = useRef<HTMLInputElement>(null);
  const [query, setQuery] = useState("");
  const full = images.items.length >= 12;
  const assets = project.assets.filter(
    (a) =>
      a.kind === "image" &&
      isSelectableAsset(a) &&
      a.name.toLocaleLowerCase().includes(query.toLocaleLowerCase()),
  );
  return (
    <>
      <input
        hidden
        ref={input}
        type="file"
        multiple
        accept="image/png,image/jpeg,image/webp,image/gif"
        aria-label={t("上传参考图片")}
        onChange={(e) => {
          const files = Array.from(e.target.files ?? []);
          e.target.value = "";
          void images.importFiles(files);
        }}
      />
      <button
        type="button"
        className="composer-icon-button"
        disabled={disabled || full}
        title={t("上传参考图片")}
        aria-label={t("上传参考图片")}
        onClick={() => input.current?.click()}
      >
        <UploadSimple size={18} />
      </button>
      <Popover.Root>
        <Popover.Trigger>
          <button
            type="button"
            className="composer-icon-button"
            disabled={disabled || full}
            title={t("引用项目图片")}
            aria-label={t("引用项目图片")}
          >
            <Image size={18} />
          </button>
        </Popover.Trigger>
        <Popover.Content side="top" align="start" className="reference-picker">
          <header>
            <strong>{t("引用项目图片")}</strong>
          </header>
          <input
            type="search"
            value={query}
            aria-label={t("搜索参考素材")}
            placeholder={t("搜索素材")}
            onChange={(e) => setQuery(e.target.value)}
          />
          <div className="reference-picker-grid">
            {assets.map((asset) => {
              const added = images.items.some((a) => a.id === asset.id);
              return (
                <button
                  type="button"
                  key={asset.id}
                  title={asset.name}
                  disabled={disabled || added || full}
                  aria-label={`${t(added ? "已引用" : "引用")} ${asset.name}`}
                  onClick={() => images.add(asset)}
                >
                  <div>
                    <img src={mediaUrl(asset.preview || asset.path)} alt="" />
                    {added && (
                      <span className="reference-added">
                        <Check size={15} />
                      </span>
                    )}
                  </div>
                  <span>{asset.name}</span>
                </button>
              );
            })}
          </div>
          {!assets.length && (
            <p className="reference-picker-empty">
              {t(query ? "没有匹配的素材" : "暂无可引用素材，可从电脑上传")}
            </p>
          )}
          <footer>
            <span>{t("已引用 {v0} 项", { v0: images.items.length })}</span>
            <Popover.Close>
              <button type="button">{t("完成")}</button>
            </Popover.Close>
          </footer>
        </Popover.Content>
      </Popover.Root>
    </>
  );
}

export function PrevisImageStrip({
  images,
  disabled,
}: {
  images: PrevisImages;
  disabled: boolean;
}) {
  return (
    <>
      {!!images.items.length && (
        <div className="previs-image-strip" aria-label={t("参考图片")}>
          {images.items.map((asset) => (
            <div className="previs-image" key={asset.id} title={asset.name}>
              <img
                src={mediaUrl(asset.preview || asset.path)}
                alt={asset.name}
              />
              <button
                type="button"
                disabled={disabled}
                aria-label={t("移除参考图片 {v0}", { v0: asset.name })}
                onClick={() => images.remove(asset.id)}
              >
                <X size={12} />
              </button>
            </div>
          ))}
        </div>
      )}
      {images.notice && (
        <p className="agent-attachment-hint" role="status">
          {images.notice}
        </p>
      )}
    </>
  );
}
