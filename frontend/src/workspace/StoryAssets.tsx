import { useRef, useState } from "react";
import { Dialog } from "@radix-ui/themes";
import {
  Check,
  Eye,
  FolderPlus,
  Images,
  ChatText,
  SquaresFour,
  ArrowUUpLeft,
  X,
  FilmSlate,
  Plus,
  ListChecks,
} from "@phosphor-icons/react";
import { t, useLanguage } from "../i18n";
import type { Asset, Project } from "../model";
import { ActionButton } from "../ui/ActionButton";
import { IconButton } from "../ui/IconButton";
import { ObjectActions } from "../ui/ObjectActions";
import { ObjectMenu } from "../ui/ObjectMenu";
import { SuccessToast } from "../ui/SuccessToast";
import { MediaPreview } from "./MediaPreview";
import { MediaTile } from "./MediaTile";
import { isConfirmedStoryAsset, setAssetUsage } from "./assetLibrary";
import {
  addStoryAsset,
  availableStoryMaterials,
  matchesAsset,
  storyAssets,
} from "./storyAssetLibrary";
import "./story-assets.css";
import { useImportMedia } from "./useImportMedia";
import { native } from "../bridge";
import { ErrorNotice } from "../errors/ErrorNotice";
import { AssetShotLinks } from "./AssetShotLinks";
import { requestCreativeTask } from "../creative/aiTasks";
import { PrevisLibrary } from "../previs/PrevisLibrary";
import { useOptionalPrevis } from "../previs/context";

export function StoryAssets({
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
  const previs = useOptionalPrevis();
  const [query, setQuery] = useState("");
  const [kind, setKind] = useState("all");
  const [scope, setScope] = useState("collected");
  const [preview, setPreview] = useState<string | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [picking, setPicking] = useState(false);
  const [pickQuery, setPickQuery] = useState("");
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const [linking, setLinking] = useState<string | null>(null);
  const imported = useImportMedia(project.id, change, setError, "asset");
  const pickerTrigger = useRef<HTMLButtonElement>(null);
  const linkTrigger = useRef<HTMLButtonElement | null>(null);
  const all = storyAssets(project);
  const collected = all.filter(isConfirmedStoryAsset);
  const pending = all.filter((a) => !isConfirmedStoryAsset(a));
  const scoped = scope === "collected" ? collected : pending;
  const assets = scoped.filter((a) => matchesAsset(a, query, kind));
  const materials = availableStoryMaterials(project);
  const choices = materials.filter((a) => matchesAsset(a, pickQuery, "all"));
  const add = (id: string) => {
    change((p) => addStoryAsset(p, id));
    setMessage(t("资产已确认"));
  };
  return (
    <section className="story-assets-workspace">
      <header className="story-assets-heading">
        <div>
          <h1>{t("资产库")}</h1>
          <p>
            {t(
              "角色、场景与物品的制作依据。确认后关联镜头，统一画面与视频中的身份。",
            )}
          </p>
        </div>
        <div className="story-assets-heading-actions">
          <ActionButton
            icon={ListChecks}
            onClick={() =>
              requestCreativeTask({
                agentId: "assets",
                text: "请读取当前脚本、镜头和已确认资产，整理需要复用的角色、场景和物品。只处理当前项目缺失或明确需要修订的资产，保留已有身份和镜头引用。通过已分配的素材和资产读取工具核对来源资料与现有资产。两者可引用同一文件 ID，建立资产后保留来源素材；已有已确认原图可直接复用，不必生成替代图。仅对缺失或指定修订的资产准备独立参考图任务，结果等待用户确认后才用于镜头；收录为素材不等于确认资产；不生成分镜图和视频。",
              })
            }
          >
            {t("与资产 Agent 整理")}
          </ActionButton>
          <ActionButton
            icon={Plus}
            busy={imported.busy}
            disabled={!native}
            onClick={() => void imported.importMedia()}
          >
            {t("导入资产")}
          </ActionButton>
          <ActionButton
            ref={pickerTrigger}
            icon={FolderPlus}
            onClick={() => {
              setPickQuery("");
              setPicking(true);
            }}
          >
            {t("从素材建立资产")}
          </ActionButton>
        </div>
      </header>
      <div
        className="story-assets-scopes"
        role="group"
        aria-label={t("资产确认状态")}
      >
        {[
          ["collected", "已确认", collected.length],
          ["pending", "待确认", pending.length],
        ].map(([id, label, count]) => (
          <button
            key={id}
            aria-pressed={scope === id}
            onClick={() => {
              setScope(String(id));
              setSelected(null);
            }}
          >
            {t(String(label))} <span>{count}</span>
          </button>
        ))}
      </div>
      {error && <ErrorNotice error={error} fallback="ASSET_IMPORT_FAILED" />}
      <div className="story-assets-tools">
        <nav aria-label={t("资产类型")}>
          {[
            ["all", "全部"],
            ["image", "图片"],
            ["video", "视频"],
            ["audio", "音频"],
            ...(previs ? [["models", "三维模型"]] : []),
          ].map(([id, label]) => (
            <button
              key={id}
              aria-pressed={kind === id}
              onClick={() => setKind(id)}
            >
              {t(label)}
            </button>
          ))}
        </nav>
        <input
          type="search"
          aria-label={t("搜索资产")}
          placeholder={t("搜索资产")}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
      </div>
      <div className="story-assets-results" aria-label={t("镜头资产")}>
        {scope === "pending" && (
          <p className="story-assets-hint">
            {t("生成结果先作为候选资产。预览确认后，才能关联镜头并用于制作。")}
          </p>
        )}
        {kind === "models" && previs ? (
          <PrevisLibrary />
        ) : assets.length ? (
          <div className="story-assets-grid">
            {assets.map((a) => {
              const shots = project.nodes.filter(
                (n) =>
                  n.kind === "shot" &&
                  n.references?.some((r) => r.assetId === a.id),
              );
              const actions = [
                { icon: Eye, label: t("预览"), run: () => setPreview(a.id) },
                {
                  icon: FilmSlate,
                  label: t("关联镜头"),
                  disabled: !isConfirmedStoryAsset(a) || !!a.missing,
                  run: () => setLinking(a.id),
                },
                {
                  icon: ChatText,
                  label: t("引用到对话"),
                  disabled: !!a.missing,
                  run: () => reference(a),
                },
                {
                  icon: SquaresFour,
                  label: t("放到草稿画布"),
                  disabled: !!a.missing,
                  run: () => place(a),
                },
                {
                  icon: ArrowUUpLeft,
                  label: shots.length
                    ? t("先解除镜头关联，才能移出资产库")
                    : t("移出资产库"),
                  disabled: shots.length > 0,
                  run: () => change((p) => setAssetUsage(p, a.id, "material")),
                },
              ];
              return (
                <ObjectMenu key={a.id} actions={actions}>
                  <article className="story-asset-card">
                    <MediaTile
                      asset={a}
                      scope="project"
                      selected={selected === a.id}
                      select={setSelected}
                      preview={setPreview}
                    />
                    <div className="story-asset-meta">
                      <span>
                        {t("用于 {count} 个镜头", { count: shots.length })}
                      </span>
                      <ObjectActions actions={actions} />
                    </div>
                    <div className="story-asset-actions">
                      <ActionButton icon={Eye} onClick={() => setPreview(a.id)}>
                        {t("预览")}
                      </ActionButton>
                      {isConfirmedStoryAsset(a) && (
                        <ActionButton
                          icon={FilmSlate}
                          disabled={a.missing}
                          onClick={(event) => {
                            linkTrigger.current = event.currentTarget;
                            setLinking(a.id);
                          }}
                        >
                          {t("关联镜头")}
                        </ActionButton>
                      )}
                      {!isConfirmedStoryAsset(a) && (
                        <ActionButton
                          icon={Check}
                          disabled={a.missing}
                          onClick={() => add(a.id)}
                        >
                          {t("确认资产")}
                        </ActionButton>
                      )}
                    </div>
                  </article>
                </ObjectMenu>
              );
            })}
          </div>
        ) : (
          <div className="story-assets-empty" role="status">
            <Images size={36} />
            <h2>
              {scoped.length
                ? t("没有匹配的资产")
                : scope === "pending"
                  ? t("没有待确认的资产")
                  : t("还没有镜头资产")}
            </h2>
            <p>
              {scoped.length
                ? t("试试其他关键词或资产类型。")
                : scope === "pending"
                  ? t("资产 Agent 生成的候选会显示在这里，确认后供镜头复用。")
                  : t(
                      "把角色、场景、物品等参考素材添加为资产，方便在分镜制作时复用。",
                    )}
            </p>
            {!scoped.length && scope === "collected" && (
              <ActionButton
                icon={FolderPlus}
                onClick={() => {
                  setPickQuery("");
                  setPicking(true);
                }}
              >
                {t("从素材建立资产")}
              </ActionButton>
            )}
          </div>
        )}
      </div>
      <SuccessToast message={message} onDismiss={() => setMessage("")} />
      <MediaPreview
        asset={project.assets.find((a) => a.id === preview)}
        close={() => setPreview(null)}
        place={place}
        reference={reference}
        actions={(a) =>
          !isConfirmedStoryAsset(a) ? (
            <ActionButton
              icon={Check}
              disabled={a.missing}
              onClick={() => {
                add(a.id);
                setPreview(null);
              }}
            >
              {t("确认资产")}
            </ActionButton>
          ) : null
        }
      />
      <Dialog.Root open={picking} onOpenChange={setPicking}>
        <Dialog.Content
          className="story-assets-picker"
          maxWidth="720px"
          onCloseAutoFocus={(event) => {
            event.preventDefault();
            pickerTrigger.current?.focus();
          }}
        >
          <header>
            <Dialog.Title>{t("从素材建立资产")}</Dialog.Title>
            <Dialog.Close>
              <IconButton icon={X} label={t("关闭")} />
            </Dialog.Close>
          </header>
          <Dialog.Description>
            {t(
              "为选中的素材建立已确认资产，供镜头复用。原素材继续保留在素材库。",
            )}
          </Dialog.Description>
          <input
            type="search"
            aria-label={t("搜索项目素材")}
            placeholder={t("搜索项目素材")}
            value={pickQuery}
            onChange={(e) => setPickQuery(e.target.value)}
          />
          <div className="story-assets-choices">
            {choices.map((a) => (
              <div className="story-asset-choice" key={a.id}>
                <span title={a.name}>{a.name}</span>
                <ActionButton
                  icon={FolderPlus}
                  disabled={a.missing}
                  onClick={() => {
                    add(a.id);
                    setScope("collected");
                    setQuery("");
                    setKind("all");
                  }}
                >
                  {a.missing ? t("文件缺失") : t("添加资产")}
                </ActionButton>
              </div>
            ))}
            {!choices.length && (
              <p role="status">
                {materials.length
                  ? t("没有匹配的素材")
                  : t("没有可添加的普通素材，请先在素材库中导入。")}
              </p>
            )}
          </div>
          <footer>
            <Dialog.Close>
              <button>{t("完成")}</button>
            </Dialog.Close>
          </footer>
        </Dialog.Content>
      </Dialog.Root>
      <AssetShotLinks
        asset={project.assets.find((a) => a.id === linking)}
        project={project}
        change={change}
        close={() => setLinking(null)}
        restoreFocus={() =>
          (linkTrigger.current?.isConnected
            ? linkTrigger.current
            : pickerTrigger.current
          )?.focus()
        }
      />
    </section>
  );
}
