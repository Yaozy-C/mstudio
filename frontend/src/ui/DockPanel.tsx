import type { ReactNode } from "react";
import { Graph, X } from "@phosphor-icons/react";

export function DockPanel({
  id,
  title,
  children,
  onClose,
  visible = true,
}: {
  id: string;
  title: string;
  children: ReactNode;
  onClose: () => void;
  visible?: boolean;
}) {
  return (
    <section
      className={`dock-panel dock-${id}`}
      aria-label={title}
      hidden={!visible}
    >
      <header className="dock-heading">
        {id === "agent" && <Graph size={24} />}
        <h2>{title}</h2>
        <button
          className="icon-button"
          onClick={onClose}
          aria-label={`关闭${title}`}
        >
          <X />
        </button>
      </header>
      <div className="dock-content">{children}</div>
    </section>
  );
}
