import { useContext, useEffect, useState } from "react";
import { ChatCircleText, Hand, PaperPlaneTilt } from "@phosphor-icons/react";
import { ActionButton } from "../../ui/ActionButton";
import { t, useLanguage } from "../../i18n";
import { mediaUrl, native } from "../../bridge";
import type { Asset } from "../../model";
import { ZoomableImage } from "../ZoomableImage";
import { ImageReviewContext } from "./ImageReviewContext";
import { renderAnnotations } from "./renderAnnotations";
import {
  commentDraftKey,
  readCommentDraft,
  saveCommentDraft,
} from "./commentDraft";
import { CommentList } from "./CommentList";
import { DockPanel } from "../../ui/DockPanel";
import "./image-review.css";

export function ImageReview({
  asset,
  close,
}: {
  asset: Asset;
  close: () => void;
}) {
  useLanguage();
  const context = useContext(ImageReviewContext);
  const key = commentDraftKey(context?.projectId ?? "", asset.id);
  const [comments, setComments] = useState(() => readCommentDraft(key));
  const [open, setOpen] = useState(comments.length > 0);
  const [marking, setMarking] = useState(true);
  const [selected, setSelected] = useState<string | null>(
    comments[0]?.id ?? null,
  );
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  useEffect(() => {
    saveCommentDraft(key, comments);
  }, [key, comments]);
  const src = mediaUrl(asset.path);
  if (!context || !context.hasAsset(asset.id))
    return <ZoomableImage src={src} alt={asset.name} />;
  const valid = comments.length > 0 && comments.every((c) => c.text.trim());
  function finish() {
    setSelected(null);
    setMarking(true);
    document
      .querySelector<HTMLElement>(".image-review .image-zoom-viewport")
      ?.focus();
  }
  async function submit() {
    if (!valid || busy || !context) return;
    if (!native) {
      setError(t("请在 Mstudio 桌面应用中提交图片批注"));
      return;
    }
    setBusy(true);
    setError("");
    try {
      const file = await renderAnnotations(src, comments);
      await context.submit(asset, file, comments);
      close();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className={`image-review${open ? " is-open" : ""}`}>
      <div className="image-review-canvas">
        <ZoomableImage
          src={src}
          alt={asset.name}
          annotations={
            open
              ? {
                  enabled: marking && !busy,
                  items: comments,
                  selected,
                  select: setSelected,
                  add: (region) => {
                    const id = crypto.randomUUID();
                    setComments((items) => [
                      ...items.filter((c) => c.text.trim()),
                      { ...region, id, text: "" },
                    ]);
                    setSelected(id);
                  },
                }
              : undefined
          }
        />
        {!open && (
          <ActionButton
            className="image-review-open"
            icon={ChatCircleText}
            onClick={() => setOpen(true)}
            title={t("标注修改")}
            aria-label={t("标注修改")}
          />
        )}
      </div>
      {open && (
        <DockPanel
          id="image-review"
          title={t("图片批注")}
          onClose={() => setOpen(false)}
        >
          <div className="image-review-tools">
            <ActionButton
              icon={ChatCircleText}
              aria-label={t("框选")}
              title={t("框选")}
              aria-pressed={marking}
              onClick={() => setMarking(true)}
              disabled={busy}
            />
            <ActionButton
              icon={Hand}
              aria-label={t("平移")}
              title={t("平移")}
              aria-pressed={!marking}
              onClick={() => setMarking(false)}
              disabled={busy}
            />
            <span className="image-review-count">
              {t("{v0} 条批注", {
                v0: comments.filter((c) => c.text.trim()).length,
              })}
            </span>
            <ActionButton
              icon={PaperPlaneTilt}
              className="image-review-send"
              busy={busy}
              title={t("引用批注到对话")}
              aria-label={t("引用批注到对话")}
              disabled={!valid}
              onClick={() => void submit()}
            />
          </div>
          {!comments.length && (
            <p className="image-review-empty subtle">
              {t("框选区域后输入意见，按 Enter 继续标下一处。")}
            </p>
          )}
          <CommentList
            comments={comments}
            selected={selected}
            busy={busy}
            select={setSelected}
            update={(id, text) =>
              setComments((items) =>
                items.map((c) => (c.id === id ? { ...c, text } : c)),
              )
            }
            remove={(id) => {
              setComments((items) => items.filter((c) => c.id !== id));
              if (selected === id) finish();
            }}
            finish={finish}
          />
          {comments.length > 0 && !selected && (
            <p className="image-review-next subtle">
              {t("继续在图上框选，添加下一条批注。")}
            </p>
          )}
          {error && <p role="alert">{error}</p>}
        </DockPanel>
      )}
    </div>
  );
}
