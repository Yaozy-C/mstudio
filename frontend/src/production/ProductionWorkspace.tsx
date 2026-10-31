import { useEffect, useState } from "react";
import {
  FilmSlate,
  FolderSimple,
  ImageSquare,
  VideoCamera,
  PencilSimple,
  ChatText,
  ArrowRight,
  Check,
  Cube,
} from "@phosphor-icons/react";
import { t, useLanguage } from "../i18n";
import { native, mediaUrl } from "../bridge";
import type { Asset, Project } from "../model";
import { ActionButton } from "../ui/ActionButton";
import { StudioSelect } from "../ui/StudioSelect";
import { ErrorNotice } from "../errors/ErrorNotice";
import { creativeTask, requestCreativeTask } from "../creative/aiTasks";
import { chooseTake, assembleScreenplay } from "../creative/timeline";
import { openPrevis } from "../previs/context";
import { isShotReference } from "../workspace/resourceRoles";
import { MediaPreview } from "../workspace/MediaPreview";
import { productionShots } from "./items";
import { ShotReferences } from "./ShotReferences";
import { editReferences } from "./editReferences";
import { ShotEditor } from "./ShotEditor";
import { TaskEntry } from "./TaskEntry";
import { type ChangeProject } from "./types";
import type { ProductionController } from "./useProduction";
import "./production-workspace.css";
import { Dialog } from "@radix-ui/themes";
import { EffectLibrary } from "../workspace/EffectLibrary";
import { effectAgentTask } from "../workspace/effectAgentTask";

export function ProductionWorkspace({
  project,
  change,
  canvas,
  onAdd,
  selected,
  select,
  navigate,
}: {
  project: Project;
  change: ChangeProject;
  canvas: ProductionController;
  onAdd: (a: Asset) => void;
  selected: string | null;
  select: (id: string | null) => void;
  navigate: (view: "script" | "assets" | "storyboard" | "film") => void;
}) {
  useLanguage();
  const [paragraph, setParagraph] = useState("all");
  const [preview, setPreview] = useState<string | null>(null);
  const [editing, setEditing] = useState(false);
  const [error, setError] = useState("");
  const [effectsOpen, setEffectsOpen] = useState(false);
  const shots = productionShots(project);
  const screenplay = project.nodes.find((n) => n.kind === "screenplay");
  const paragraphs = screenplay?.screenplay?.script ?? [];
  const shown = shots.filter(
    (n) => paragraph === "all" || n.shot?.scriptId === paragraph,
  );
  const focusedId = canvas.focus?.id;
  const shot =
    shown.find((n) => n.id === selected) ??
    shown.find((n) => n.id === focusedId) ??
    shown[0];
  useEffect(() => {
    if (shot && !shown.some((n) => n.id === selected)) select(shot.id);
  }, [shot?.id, selected]);
  useEffect(() => {
    const focus = canvas.focus;
    const id = focus?.id;
    if (
      id &&
      id !== selected &&
      project.nodes.some((n) => n.id === id && n.shot)
    ) {
      setParagraph("all");
      select(id);
    }
  }, [canvas.focus?.tick]);
  const frames = shot?.shot?.frames ?? [];
  const takes = shot?.shot?.takes ?? [];
  const runs = (canvas.runs ?? []).filter(
    (r) => (r.targetNodeId ?? r.ownerId) === shot?.id && !r.hiddenFromList,
  );
  const generate = (kind: "image" | "video", frameId?: string) => {
    if (!shot) return;
    canvas.setComposerMode("agent");
    requestCreativeTask({
      agentId: kind === "image" ? "image" : "production",
      targetNodeId: shot.id,
      refs: [
        { kind: "node", id: shot.id },
        ...(frameId ? [{ kind: "asset" as const, id: frameId }] : []),
      ],
      text: `请为镜头「${shot.title}」（ID: ${shot.id}）准备${kind === "image" ? "分镜图" : "视频片段"}生成任务。读取当前镜头设计与已关联资产，保留脚本和其他镜头。镜头身份、场景和物品只使用已确认资产；缺失时明确指出并交给资产 Agent 建立，同一文件也在素材库中不影响其已确认资产资格；未确认的来源资料不能直接作为镜头身份参考。${frameId ? `本次明确选用分镜图 ${frameId} 作为视频控制输入。` : "已有分镜图、预演和视频片段可按本次任务需要作为制作输入。"}复用已有任务，已有结果保留。提交完整可检查的任务，等待用户开始生成。`,
    });
  };
  const run = (action: () => void) => {
    try {
      action();
      setError("");
    } catch (e) {
      setError(String(e));
    }
  };
  return (
    <section className="shot-workspace">
      <header className="shot-workspace-heading">
        <div>
          <h1>{t("制作")}</h1>
          <p>{t("以镜头为中心：关联资产，制作分镜图，再选择视频片段。")}</p>
        </div>
        <ActionButton icon={FolderSimple} onClick={() => navigate("assets")}>
          {t("管理镜头资产")}
        </ActionButton>
      </header>
      {!shots.length ? (
        <div className="shot-workspace-empty">
          <FilmSlate size={36} />
          <h2>{t("先把脚本拆成镜头")}</h2>
          <p>
            {t(
              "镜头承载设计、资产引用、分镜图和视频片段。制作结果会回到对应镜头。",
            )}
          </p>
          <ActionButton
            icon={ArrowRight}
            onClick={() => {
              navigate("script");
              if (screenplay)
                requestCreativeTask(creativeTask("split", screenplay.id));
            }}
          >
            {t("前往脚本设计镜头")}
          </ActionButton>
        </div>
      ) : (
        <div className="shot-workbench">
          <aside className="shot-directory" aria-label={t("镜头目录")}>
            <StudioSelect
              value={paragraph}
              onValueChange={(id) => {
                setParagraph(id);
                select(
                  shots.find((n) => id === "all" || n.shot?.scriptId === id)
                    ?.id ?? null,
                );
              }}
              label={t("脚本段落")}
              options={[
                { value: "all", label: t("全部镜头") },
                ...paragraphs.map((p) => ({
                  value: p.id,
                  label: p.title || t("未命名段落"),
                })),
              ]}
            />
            <nav>
              {shown.map((n) => (
                <button
                  key={n.id}
                  aria-pressed={shot?.id === n.id}
                  onClick={() => {
                    select(n.id);
                    setError("");
                  }}
                >
                  <small>
                    {String(n.shot!.order).padStart(2, "0")} ·{" "}
                    {t("{v0} 秒", { v0: n.shot!.duration })}
                  </small>
                  <strong>{n.title}</strong>
                  <span>
                    {t("资产 {assets} · 图片 {images} · 视频 {videos}", {
                      assets: n.references?.length ?? 0,
                      images: n.shot?.frames?.length ?? 0,
                      videos: n.shot?.takes?.length ?? 0,
                    })}
                  </span>
                  {n.resultAssetId && <em>{t("已选用片段")}</em>}
                </button>
              ))}
            </nav>
            <ActionButton
              icon={ArrowRight}
              disabled={!screenplay || !shots.every((n) => n.resultAssetId)}
              onClick={() =>
                run(() => {
                  change((p) => assembleScreenplay(p, screenplay!.id));
                  navigate("film");
                })
              }
            >
              {t("编排到剪辑")}
            </ActionButton>
          </aside>
          <div className="shot-detail">
            {error && (
              <ErrorNotice error={error} fallback="VALIDATION_FAILED" />
            )}
            {!shot ? (
              <p>{t("这个段落还没有镜头。")}</p>
            ) : (
              <>
                <header className="shot-detail-heading">
                  <div>
                    <small>
                      {paragraphs.find((p) => p.id === shot.shot?.scriptId)
                        ?.title ?? t("独立镜头")}
                    </small>
                    <h2>{shot.title}</h2>
                  </div>
                  <ActionButton
                    icon={PencilSimple}
                    onClick={() => setEditing(true)}
                  >
                    {t("编辑镜头")}
                  </ActionButton>
                </header>
                <section className="shot-design">
                  <h3>{t("镜头设计")}</h3>
                  <p>{shot.text || t("尚未填写画面描述")}</p>
                  {shot.shot?.dialogue && (
                    <p>
                      <strong>{t("台词 / 旁白")}</strong>
                      <br />
                      {shot.shot.dialogue}
                    </p>
                  )}
                  <div className="shot-inline-actions">
                    <ActionButton
                      icon={ChatText}
                      onClick={() =>
                        requestCreativeTask(
                          creativeTask("revise-shot", shot.id),
                        )
                      }
                    >
                      {t("与导演修改镜头")}
                    </ActionButton>
                    <ActionButton
                      icon={Cube}
                      disabled={!native}
                      onClick={() => openPrevis(shot.id)}
                    >
                      {t("三维预演")}
                    </ActionButton>
                    <ActionButton
                      icon={VideoCamera}
                      onClick={() => setEffectsOpen(true)}
                    >
                      {t("特效库")}
                    </ActionButton>
                  </div>
                </section>
                <ShotReferences
                  project={project}
                  references={shot.references ?? []}
                  onChange={(refs) =>
                    run(() =>
                      change((p) =>
                        editReferences(p, shot.id, shot.references ?? [], refs),
                      ),
                    )
                  }
                />
                {(shot.references ?? []).some(
                  (r) =>
                    !isShotReference(
                      project,
                      [...project.assets, ...(project.previsAssets ?? [])].find(
                        (a) => a.id === r.assetId,
                      ) ?? ({} as Asset),
                    ),
                ) && (
                  <p className="shot-reference-issue" role="status">
                    {t(
                      "镜头引用必须是已确认资产。请在资产库确认后再开始制作。",
                    )}
                  </p>
                )}
                <section className="shot-output-section">
                  <header>
                    <h3>{t("分镜图")}</h3>
                    <ActionButton
                      icon={ImageSquare}
                      onClick={() => generate("image")}
                    >
                      {t("制作分镜图")}
                    </ActionButton>
                  </header>
                  {!frames.length ? (
                    <p className="shot-output-empty">
                      {t("还没有分镜图。可以先制作图片，也可以直接制作视频。")}
                    </p>
                  ) : (
                    <div className="shot-output-grid">
                      {frames.map((f) => {
                        const a = project.assets.find(
                          (a) => a.id === f.assetId,
                        );
                        return (
                          a && (
                            <article key={a.id}>
                              <button
                                className="shot-output-preview"
                                onClick={() => setPreview(a.id)}
                                aria-label={t("预览 {name}", { name: a.name })}
                              >
                                {a.missing ? (
                                  t("文件缺失")
                                ) : (
                                  <img
                                    src={mediaUrl(a.preview || a.path)}
                                    alt={f.title || a.name}
                                    loading="lazy"
                                  />
                                )}
                              </button>
                              <strong>{f.title || a.name}</strong>
                              <ActionButton
                                icon={VideoCamera}
                                disabled={a.missing}
                                onClick={() => generate("video", a.id)}
                              >
                                {t("用这版制作视频")}
                              </ActionButton>
                            </article>
                          )
                        );
                      })}
                    </div>
                  )}
                </section>
                <section className="shot-output-section">
                  <header>
                    <h3>{t("视频片段")}</h3>
                    <ActionButton
                      icon={VideoCamera}
                      onClick={() => generate("video")}
                    >
                      {t("制作视频片段")}
                    </ActionButton>
                  </header>
                  {!takes.length ? (
                    <p className="shot-output-empty">
                      {t("视频生成结果会保留在当前镜头，选定后用于剪辑。")}
                    </p>
                  ) : (
                    <div className="shot-output-grid">
                      {takes.map((take) => {
                        const a = project.assets.find(
                          (a) => a.id === take.assetId,
                        );
                        return (
                          a && (
                            <article
                              key={a.id}
                              data-chosen={shot.resultAssetId === a.id}
                            >
                              <button
                                className="shot-output-preview"
                                onClick={() => setPreview(a.id)}
                                aria-label={t("预览 {name}", { name: a.name })}
                              >
                                {a.missing ? (
                                  t("文件缺失")
                                ) : a.preview ? (
                                  <img
                                    src={mediaUrl(a.preview)}
                                    alt={a.name}
                                    loading="lazy"
                                  />
                                ) : (
                                  <VideoCamera size={32} />
                                )}
                              </button>
                              <strong>{a.name}</strong>
                              <ActionButton
                                icon={Check}
                                disabled={
                                  a.missing || shot.resultAssetId === a.id
                                }
                                onClick={() =>
                                  run(() =>
                                    change((p) => chooseTake(p, shot.id, a.id)),
                                  )
                                }
                              >
                                {shot.resultAssetId === a.id
                                  ? t("已选用片段")
                                  : t("选用片段")}
                              </ActionButton>
                            </article>
                          )
                        );
                      })}
                    </div>
                  )}
                </section>
                {!!runs.length && (
                  <section className="shot-output-section">
                    <header>
                      <h3>{t("当前镜头的生成任务")}</h3>
                    </header>
                    {runs.map((r) => (
                      <TaskEntry key={r.key} task={r} canvas={canvas} />
                    ))}
                  </section>
                )}
              </>
            )}
          </div>
        </div>
      )}
      <MediaPreview
        asset={project.assets.find((a) => a.id === preview)}
        close={() => setPreview(null)}
        add={onAdd}
      />
      {editing && shot && (
        <ShotEditor
          key={shot.id}
          node={shot}
          project={project}
          change={change}
          close={() => setEditing(false)}
        />
      )}
      <Dialog.Root open={effectsOpen} onOpenChange={setEffectsOpen}>
        <Dialog.Content className="shot-effects-dialog">
          <Dialog.Title>{t("特效库")}</Dialog.Title>
          <Dialog.Description>
            {t("为当前镜头选择效果，沿用镜头设计和已确认资产。")}
          </Dialog.Description>
          <EffectLibrary
            onUse={(effect) => {
              setEffectsOpen(false);
              if (shot)
                requestCreativeTask(
                  effectAgentTask(effect, { kind: "node", id: shot.id }),
                );
            }}
          />
          <footer>
            <Dialog.Close>
              <button>{t("关闭")}</button>
            </Dialog.Close>
          </footer>
        </Dialog.Content>
      </Dialog.Root>
    </section>
  );
}
