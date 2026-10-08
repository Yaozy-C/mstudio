import { SuccessToast } from "../ui/SuccessToast";
import { AsyncButton, LoadingState } from "../ui/AsyncState";
import { useState } from "react";
import { AlertDialog } from "@radix-ui/themes";
import { Images, Plus, Trash, Eye } from "@phosphor-icons/react";
import { t, useLanguage } from "../i18n";
import { native } from "../bridge";
import type { Asset } from "../model";
import { ErrorNotice } from "../errors/ErrorNotice";
import { useAssetLibrary } from "./useAssetLibrary";
import { MediaTile } from "./MediaTile";
import { MediaPreview } from "./MediaPreview";
import { ObjectMenu } from "../ui/ObjectMenu";
import "./resource-library.css";

export function PublicAssets() {
  useLanguage();
  const library = useAssetLibrary();
  const [query, setQuery] = useState("");
  const [kind, setKind] = useState("all");
  const [selected, setSelected] = useState<string | null>(null);
  const [preview, setPreview] = useState<string | null>(null);
  const [removing, setRemoving] = useState<Asset | null>(null);
  const groups = [
    ["all", "全部素材"],
    ["image", "图片"],
    ["video", "视频"],
    ["audio", "音频"],
    ["document", "文档"],
  ];
  const matchesKind = (asset: Asset, value: string) =>
    value === "all" ||
    asset.kind === value ||
    (value === "document" && asset.kind === "text");
  const assets = library.assets.filter(
    (asset) =>
      matchesKind(asset, kind) &&
      asset.name.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase()),
  );
  return (
    <main className="library-main public-assets">
      <header className="library-heading">
        <div>
          <h1>{t("公共素材")}</h1>
          <p className="subtle">{t("跨项目复用的素材，在这里统一管理。")}</p>
        </div>
        <AsyncButton
          busy={library.busy}
          className="primary"
          disabled={!native || library.busy}
          onClick={() => void library.upload()}
        >
          <Plus />
          {t("导入素材")}
        </AsyncButton>
      </header>
      {!native && (
        <p className="subtle">
          {t("浏览器界面预览 · 素材导入、生成与导出请使用桌面应用")}
        </p>
      )}
      <div className="public-assets-tools">
        <nav aria-label={t("素材分类")}>
          {groups.map(([value, label]) => (
            <button
              key={value}
              aria-pressed={kind === value}
              onClick={() => setKind(value)}
            >
              {t(label)}{" "}
              <span>
                {library.assets.filter((a) => matchesKind(a, value)).length}
              </span>
            </button>
          ))}
        </nav>
        <input
          type="search"
          aria-label={t("搜索公共素材")}
          placeholder={t("搜索公共素材")}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
      </div>
      {library.error && !removing && (
        <ErrorNotice error={library.error} fallback="ASSET_OPERATION_FAILED">
          <button disabled={library.busy} onClick={() => void library.retry()}>
            {t(library.retryLabel)}
          </button>
        </ErrorNotice>
      )}
      <SuccessToast
        message={library.message}
        onDismiss={library.dismissMessage}
      />
      {library.loading && !library.assets.length && (
        <LoadingState label={t("正在读取公共素材…")} />
      )}
      <section
        className="public-assets-grid"
        aria-label={t("公共素材")}
        aria-busy={library.loading}
      >
        {assets.map((asset) => (
          <ObjectMenu
            key={asset.id}
            actions={[
              { label: t("预览素材"), run: () => setPreview(asset.id) },
              {
                label: t("从公共素材移除"),
                run: () => setRemoving(asset),
                disabled: library.busy,
                danger: true,
              },
            ]}
          >
            <div className="public-asset">
              <MediaTile
                asset={asset}
                scope="global"
                selected={selected === asset.id}
                select={setSelected}
                preview={setPreview}
              />
              <div className="public-asset-actions">
                <button onClick={() => setPreview(asset.id)}>
                  <Eye />
                  {t("预览素材")}
                </button>
                <button
                  aria-label={t("移除素材 {name}", { name: asset.name })}
                  disabled={library.busy}
                  onClick={() => setRemoving(asset)}
                >
                  <Trash />
                  {t("移除")}
                </button>
              </div>
            </div>
          </ObjectMenu>
        ))}
      </section>
      {!library.loading && !assets.length && !library.error && (
        <div className="resource-empty" role="status">
          <Images size={36} />
          <h2>
            {library.assets.length
              ? t("没有匹配的素材")
              : t("把常用素材放在这里")}
          </h2>
          <p>
            {library.assets.length
              ? t("试试其他关键词或素材分类。")
              : t(
                  "导入品牌图片、视频、音乐或文档，在任意项目的「素材 → 公共素材」中使用。",
                )}
          </p>
        </div>
      )}
      <MediaPreview
        asset={library.assets.find((a) => a.id === preview)}
        close={() => setPreview(null)}
      />
      <AlertDialog.Root
        open={!!removing}
        onOpenChange={(open) => {
          if (!open && !library.busy) setRemoving(null);
        }}
      >
        <AlertDialog.Content className="modal small" aria-busy={library.busy}>
          <AlertDialog.Title>{t("移除公共素材")}</AlertDialog.Title>
          <AlertDialog.Description>
            {t("将「{name}」从公共素材库移除？其他项目中已使用的素材会保留。", {
              name: removing?.name,
            })}
          </AlertDialog.Description>
          {library.error && (
            <ErrorNotice
              error={library.error}
              fallback="ASSET_OPERATION_FAILED"
            />
          )}
          <footer>
            <AlertDialog.Cancel>
              <button disabled={library.busy}>{t("取消")}</button>
            </AlertDialog.Cancel>
            <AsyncButton
              busy={library.busy}
              busyLabel={t("正在移除…")}
              className="primary"
              disabled={library.busy}
              onClick={async () => {
                if (removing && (await library.remove(removing))) {
                  if (selected === removing.id) setSelected(null);
                  if (preview === removing.id) setPreview(null);
                  setRemoving(null);
                }
              }}
            >
              {t("确认移除")}
            </AsyncButton>
          </footer>
        </AlertDialog.Content>
      </AlertDialog.Root>
    </main>
  );
}
