import type { ReactNode } from "react";
import { useConversationState } from "./conversationState";

export function LazyDetails({
  id,
  className,
  summaryClassName,
  summary,
  children,
}: {
  id: string;
  className?: string;
  summaryClassName?: string;
  summary: ReactNode;
  children: () => ReactNode;
}) {
  const [open, setOpen] = useConversationState<boolean>(id, false);
  return (
    <details
      className={className}
      open={open}
      onToggle={(event) => {
        if (event.target === event.currentTarget)
          setOpen(event.currentTarget.open);
      }}
    >
      <summary className={summaryClassName}>{summary}</summary>
      {open && children()}
    </details>
  );
}
