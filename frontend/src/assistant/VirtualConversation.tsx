import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useState,
  type ReactNode,
} from "react";
import {
  defaultRangeExtractor,
  useVirtualizer,
  type Range,
} from "@tanstack/react-virtual";
import {
  ConversationState,
  type ConversationStates,
} from "./conversationState";

export function VirtualConversation<Entry extends { key: string }>({
  entries,
  viewport,
  scrollRequest,
  children,
}: {
  entries: Entry[];
  viewport: HTMLDivElement | null;
  scrollRequest: number;
  children: (entry: Entry) => ReactNode;
}) {
  const [states] = useState<ConversationStates>(() => new Map());
  const [focused, setFocused] = useState<string | null>(null);
  const getItemKey = useCallback(
    (index: number) => entries[index].key,
    [entries],
  );
  const focusedIndex = entries.findIndex((entry) => entry.key === focused);
  const virtualizer = useVirtualizer<HTMLDivElement, HTMLDivElement>({
    count: entries.length,
    getScrollElement: () => viewport,
    getItemKey,
    estimateSize: () => 240,
    anchorTo: "end",
    followOnAppend: true,
    scrollEndThreshold: 24,
    overscan: 3,
    useAnimationFrameWithResizeObserver: true,
    rangeExtractor: useCallback(
      (range: Range) => {
        const visible = defaultRangeExtractor(range);
        if (focusedIndex < 0 || visible.includes(focusedIndex)) return visible;
        return [...visible, focusedIndex].sort((left, right) => left - right);
      },
      [focusedIndex],
    ),
  });
  useLayoutEffect(() => {
    virtualizer.scrollToEnd();
  }, [virtualizer, viewport, scrollRequest]);
  useEffect(() => {
    const keys = new Set(entries.map((entry) => entry.key));
    for (const key of states.keys()) if (!keys.has(key)) states.delete(key);
  }, [entries, states]);
  return (
    <div
      className="conversation-timeline"
      style={{ height: virtualizer.getTotalSize(), position: "relative" }}
    >
      {virtualizer.getVirtualItems().map((item) => {
        const entry = entries[item.index];
        return (
          <div
            key={entry.key}
            ref={virtualizer.measureElement}
            data-index={item.index}
            data-conversation-key={entry.key}
            style={{
              position: "absolute",
              top: item.start,
              left: 0,
              width: "100%",
              display: "flow-root",
            }}
            onFocusCapture={() => setFocused(entry.key)}
            onBlurCapture={(event) => {
              if (!event.currentTarget.contains(event.relatedTarget))
                setFocused(null);
            }}
          >
            <ConversationState id={entry.key} states={states}>
              {children(entry)}
            </ConversationState>
          </div>
        );
      })}
    </div>
  );
}
