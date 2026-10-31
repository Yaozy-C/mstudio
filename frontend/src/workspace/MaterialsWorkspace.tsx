import { useState } from "react";
import type { Asset, Project } from "../model";
import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { MediaPanel } from "./MediaPanel";
import { useImportMedia } from "./useImportMedia";
import { deleteAssets } from "./deleteAssets";
import "./materials-workspace.css";

export function MaterialsWorkspace({
  project,
  change,
  reference,
  place,
}: {
  project: Project;
  change: (fn: (p: Project) => Project) => void;
  reference: (a: Asset) => void;
  place: (a: Asset) => void;
}) {
  useLanguage();
  const [error, setError] = useState("");
  const { busy, importMedia } = useImportMedia(project.id, change, setError);
  return (
    <section className="materials-workspace">
      <header>
        <h1>{t("素材资料")}</h1>
        <p>{t("产品事实、品牌文档与参考内容，供创意和脚本查阅。")}</p>
      </header>
      {error && <ErrorNotice error={error} fallback="ASSET_IMPORT_FAILED" />}
      <MediaPanel
        project={project}
        change={change}
        onImport={() => void importMedia()}
        busy={busy}
        onReference={reference}
        onPlace={place}
        onRemove={(a) => change((p) => deleteAssets(p, [a.id]))}
      />
    </section>
  );
}
