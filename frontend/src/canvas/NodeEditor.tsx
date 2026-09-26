import { ObjectActions } from "../ui/ObjectActions";
import { useEffect, useRef, useState } from "react";
import { X, Trash, ArrowDown } from "@phosphor-icons/react";
import type { Asset, BoardNode, Project } from "../model";
import { mediaUrl } from "../bridge";
import { registerPendingEdit } from "../workspace/pendingEdits";
export function NodeEditor({
  node,
  asset,
  view,
  onCommit,
  onClose,
  onReference,
  onRemove,
  onAdd,
}: {
  node: BoardNode;
  asset?: Asset;
  view: Project["viewport"];
  onCommit: (title: string, text: string) => void;
  onClose: () => void;
  onReference: () => void;
  onRemove: () => void;
  onAdd: () => void;
}) {
  const [title, setTitle] = useState(node.title),
    [text, setText] = useState(node.text);
  const latest = useRef({ title, text });
  latest.current = { title, text };
  const previous = useRef(latest.current);
  const commit = useRef(onCommit);
  commit.current = onCommit;
  const save = () => {
    const value = latest.current;
    if (
      value.title !== previous.current.title ||
      value.text !== previous.current.text
    ) {
      commit.current(value.title, value.text);
      previous.current = value;
    }
  };
  useEffect(() => registerPendingEdit(save), []);
  const media = node.kind === "asset";
  return (
    <section
      className="canvas-node-editor"
      aria-label={media ? "查看素材" : "编辑画布内容"}
      style={{
        left: Math.max(
          16,
          Math.min(window.innerWidth - 450, node.x * view.scale + view.x),
        ),
        top: Math.max(
          130,
          Math.min(window.innerHeight - 480, node.y * view.scale + view.y),
        ),
      }}
      onKeyDown={(e) => {
        e.stopPropagation();
        if (e.key === "Escape") onClose();
        if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
          e.preventDefault();
          save();
          onClose();
        }
      }}
    >
      <header>
        <small>
          {media ? "素材" : node.kind === "shot" ? "镜头内容" : "文字 / 脚本"}
        </small>
        <ObjectActions
          actions={[
            {
              label: "引用到对话",
              run: () => {
                save();
                onClose();
                onReference();
              },
            },
          ]}
        />
        <button aria-label="关闭内容编辑" onClick={onClose}>
          <X />
        </button>
      </header>
      <input
        aria-label="内容标题"
        value={title}
        onChange={(e) => setTitle(e.target.value)}
      />
      {asset && (
        <div className="node-media-preview">
          {asset.kind === "image" ? (
            <img src={mediaUrl(asset.path)} alt={asset.name} />
          ) : asset.kind === "video" ? (
            <video
              controls
              playsInline
              preload="metadata"
              src={mediaUrl(asset.path)}
            />
          ) : (
            <audio controls preload="metadata" src={mediaUrl(asset.path)} />
          )}
        </div>
      )}
      {!media && (
        <textarea
          autoFocus
          aria-label="内容正文"
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder="写下脚本、镜头内容或想法…"
        />
      )}
      <footer>
        <button
          className="node-remove"
          aria-label="移除画布对象"
          title="移除画布对象，可撤销"
          onClick={onRemove}
        >
          <Trash />
        </button>
        {asset && (
          <button
            aria-label="将素材加入时间线"
            title="加入时间线"
            onClick={onAdd}
          >
            <ArrowDown />
          </button>
        )}
        <span />
        <button
          onClick={() => {
            save();
            onClose();
          }}
        >
          完成
        </button>
      </footer>
    </section>
  );
}
