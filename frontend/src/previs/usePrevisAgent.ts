import { useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { bridge, native } from "../bridge";
import type { ActivityEvent } from "../assistant/activityRows";
import type { PrevisFile } from "./types";

type Request = {
  projectId: string;
  fileId: string;
  requestId: string;
  expectedRevision: number;
  prompt: string;
  modelId: string;
};
type Progress = Pick<Request, "projectId" | "fileId" | "requestId"> & {
  kind: string;
  value: { id?: string; text?: string };
};

/** A separate Agent stream; never subscribes to the project Agent's events. */
export function usePrevisAgent() {
  const [partial, setPartial] = useState("");
  const [events, setEvents] = useState<ActivityEvent[]>([]);
  async function run(request: Request) {
    setPartial("");
    setEvents([]);
    let seq = 0;
    const off = native
      ? await listen<Progress>("previs-agent-progress", ({ payload: p }) => {
          if (
            p.projectId !== request.projectId ||
            p.fileId !== request.fileId ||
            p.requestId !== request.requestId
          )
            return;
          if (p.kind === "assistant/partial") {
            setPartial(p.value.text || "");
            return;
          }
          const event = {
            seq: ++seq,
            kind: p.kind,
            created: Date.now(),
            payload: p.value,
          };
          if (["assistant/thinking", "assistant/reasoning"].includes(p.kind)) {
            event.kind = "assistant/reasoning";
            setEvents((items) => {
              const index = items.findIndex(
                (item) =>
                  item.kind === event.kind &&
                  (item.payload as { id?: string }).id === p.value.id,
              );
              return index < 0
                ? [...items, event]
                : items.map((item, i) =>
                    i === index ? { ...event, seq: item.seq } : item,
                  );
            });
          } else setEvents((items) => [...items, event]);
        })
      : () => {};
    try {
      return await bridge<{ file: PrevisFile }>("previs_chat", { request });
    } finally {
      off();
    }
  }
  return { partial, events, run };
}
