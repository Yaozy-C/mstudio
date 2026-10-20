import { useRef, useState } from "react";
import { ArrowRight, LinkSimple, UploadSimple } from "@phosphor-icons/react";
import { native } from "../../bridge";
import { t } from "../../i18n";
import type { AnalysisController } from "./controller";

export function SourcePicker({
  controller,
  busy,
  compact = false,
}: {
  controller: AnalysisController;
  busy: boolean;
  compact?: boolean;
}) {
  const file = useRef<HTMLInputElement>(null);
  const [url, setUrl] = useState("");
  return (
    <div className={`analysis-import${compact ? " is-compact" : ""}`}>
      <input
        ref={file}
        type="file"
        accept="video/*,.mp4,.mov,.m4v,.webm,.mkv"
        className="analysis-file-input"
        aria-label={t("选择视频文件")}
        onChange={(event) => {
          const selected = event.target.files?.[0];
          if (selected) controller.previewFile(selected);
          event.target.value = "";
        }}
      />
      <button
        className="primary analysis-file-button"
        disabled={busy}
        onClick={() => {
          if (native) void controller.pickFile();
          else file.current?.click();
        }}
      >
        <UploadSimple size={21} aria-hidden="true" />
        {t("选择视频文件")}
      </button>
      <form
        className="analysis-link-form"
        onSubmit={(event) => {
          event.preventDefault();
          void controller.loadUrl(url);
        }}
      >
        <div className="analysis-link-input">
          <LinkSimple size={20} aria-hidden="true" />
          <input
            type="url"
            required
            aria-label={t("视频链接")}
            placeholder={t("或粘贴视频链接")}
            value={url}
            disabled={busy || !native}
            onChange={(event) => setUrl(event.target.value)}
          />
        </div>
        <button
          type="submit"
          className="analysis-link-submit"
          aria-label={t("读取视频")}
          disabled={busy || !native || !url.trim()}
        >
          <ArrowRight size={22} aria-hidden="true" />
        </button>
      </form>
    </div>
  );
}
