import { useEffect, useId, useRef, useState } from "react";
import { PencilSimple } from "@phosphor-icons/react";
import { registerPendingEdit } from "../workspace/pendingEdits";

export function InlineText({
  label,
  context,
  value,
  placeholder,
  limit,
  commit,
}: {
  label: string;
  context: string;
  value: string;
  placeholder: string;
  limit: number;
  commit: (value: string) => void;
}) {
  const id = useId();
  const [text, setText] = useState(value);
  const [focused, setFocused] = useState(false);
  const draft = useRef({ value, dirty: false });
  const composing = useRef(false);
  const save = useRef(() => {});
  save.current = () => {
    if (!draft.current.dirty) return;
    draft.current.dirty = false;
    commit(draft.current.value);
  };
  useEffect(() => {
    draft.current = { value, dirty: false };
    setText(value);
  }, [value]);
  useEffect(() => {
    const unregister = registerPendingEdit(() => save.current());
    return () => {
      unregister();
      save.current();
    };
  }, []);
  return (
    <div className="creative-inline-field">
      <label htmlFor={id}>
        <span>{label}</span>
        <PencilSimple aria-hidden="true" />
      </label>
      <div className="creative-inline-input">
        <span aria-hidden="true">{(text || placeholder) + "\n"}</span>
        <textarea
          id={id}
          aria-label={`${context} · ${label}`}
          aria-describedby={`${id}-hint`}
          value={text}
          placeholder={placeholder}
          maxLength={limit}
          rows={1}
          onFocus={() => setFocused(true)}
          onBlur={() => {
            save.current();
            setFocused(false);
          }}
          onChange={(e) => {
            draft.current = { value: e.target.value, dirty: true };
            setText(e.target.value);
          }}
          onCompositionStart={() => {
            composing.current = true;
          }}
          onCompositionEnd={() => {
            composing.current = false;
          }}
          onKeyDown={(e) => {
            e.stopPropagation();
            if (composing.current || e.nativeEvent.isComposing) return;
            if (e.key === "Escape") {
              e.preventDefault();
              draft.current = { value, dirty: false };
              setText(value);
              e.currentTarget.blur();
            } else if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
              e.preventDefault();
              e.currentTarget.blur();
            } else if (
              (e.metaKey || e.ctrlKey) &&
              e.key.toLowerCase() === "s"
            ) {
              e.preventDefault();
              save.current();
            }
          }}
        />
      </div>
      <span
        id={`${id}-hint`}
        className={`creative-inline-hint ${focused ? "visible" : ""}`}
      >
        离开自动保存 · Esc 取消
      </span>
    </div>
  );
}
