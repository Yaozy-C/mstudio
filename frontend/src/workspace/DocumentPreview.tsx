import { t, useLanguage } from "../i18n";
import { useEffect, useState } from "react";
import type { Asset } from "../model";
import { mediaUrl } from "../bridge";
import { MissingAsset } from "./MissingAsset";
export function DocumentPreview({ asset }: { asset: Asset }) {
  useLanguage();
  const [text, setText] = useState("");
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    setText("");
    setFailed(false);
    if (asset.missing || asset.kind !== "text") return;
    const controller = new AbortController();
    void fetch(mediaUrl(asset.path), { signal: controller.signal })
      .then((r) => {
        if (!r.ok) throw new Error(t("读取失败"));
        return r.text();
      })
      .then(setText)
      .catch(() => {
        if (!controller.signal.aborted) setFailed(true);
      });
    return () => controller.abort();
  }, [asset.id, asset.kind, asset.path, asset.missing]);
  if (asset.missing || failed) return <MissingAsset asset={asset} />;
  return asset.kind === "document" ? (
    <iframe
      sandbox=""
      title={asset.name}
      src={mediaUrl(asset.path)}
      style={{ width: "100%", height: "60vh", border: 0 }}
    />
  ) : (
    <pre
      style={{
        whiteSpace: "pre-wrap",
        overflow: "auto",
        maxHeight: "60vh",
        width: "100%",
        textAlign: "left",
      }}
    >
      {text || t("读取中…")}
    </pre>
  );
}
