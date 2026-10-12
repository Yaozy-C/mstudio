import { useEffect, useRef } from "react";
import { Check, Trash } from "@phosphor-icons/react";
import { ActionButton } from "../../ui/ActionButton";
import { t } from "../../i18n";
import type { ImageComment } from "./annotations";

export function CommentList({
  comments,
  selected,
  busy,
  select,
  update,
  remove,
  finish,
}: {
  comments: ImageComment[];
  selected: string | null;
  busy: boolean;
  select: (id: string) => void;
  update: (id: string, text: string) => void;
  remove: (id: string) => void;
  finish: () => void;
}) {
  const input = useRef<HTMLTextAreaElement>(null);
  useEffect(() => {
    input.current?.focus();
    input.current?.scrollIntoView({ block: "nearest" });
  }, [selected]);
  return (
    <div className="image-review-comments">
      {comments.map((comment, index) => (
        <div
          className="image-review-comment"
          key={comment.id}
          data-selected={selected === comment.id}
        >
          <span className="image-review-number">{index + 1}</span>
          {selected === comment.id ? (
            <div className="image-review-editor">
              <textarea
                ref={input}
                aria-label={t("批注 {v0}", { v0: index + 1 })}
                value={comment.text}
                disabled={busy}
                rows={3}
                placeholder={t("描述这个区域应该怎么改…")}
                onChange={(event) => update(comment.id, event.target.value)}
                onKeyDown={(event) => {
                  if (event.nativeEvent.isComposing) return;
                  if (event.key === "Enter" && !event.shiftKey) {
                    event.preventDefault();
                    if (comment.text.trim()) finish();
                  }
                }}
              />
              <div className="image-review-comment-actions">
                <small>{t("Enter 继续标注 · Shift+Enter 换行")}</small>
                <ActionButton
                  icon={Check}
                  title={t("完成并继续标注")}
                  aria-label={t("完成并继续标注")}
                  disabled={busy || !comment.text.trim()}
                  onClick={finish}
                />
              </div>
            </div>
          ) : (
            <button
              className="image-review-comment-text"
              type="button"
              disabled={busy}
              onClick={() => select(comment.id)}
            >
              {comment.text}
            </button>
          )}
          <ActionButton
            className="image-review-delete"
            icon={Trash}
            title={t("删除批注 {v0}", { v0: index + 1 })}
            aria-label={t("删除批注 {v0}", { v0: index + 1 })}
            disabled={busy}
            onClick={() => remove(comment.id)}
          />
        </div>
      ))}
    </div>
  );
}
