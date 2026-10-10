import { activityRows, type ActivityEvent } from "./activityRows";

type TimelineEntry = {
  id: number;
  kind: "tool" | "thought" | "progress" | "skill";
  text: string;
  tool?: ReturnType<typeof activityRows>[number];
};

export function activityTimeline(events: ActivityEvent[]) {
  const tools = new Map(activityRows(events).map((row) => [row.id, row]));
  return [...events]
    .sort((a, b) => a.seq - b.seq)
    .flatMap<TimelineEntry>((event) => {
      const payload = event.payload as {
        text?: string;
        skill?: string;
        path?: string;
      };
      const tool = tools.get(event.seq);
      if (tool)
        return [{ id: event.seq, kind: "tool" as const, text: "", tool }];
      if (
        ["assistant/progress", "assistant/reasoning"].includes(event.kind) &&
        payload?.text?.trim()
      )
        return [
          {
            id: event.seq,
            kind: event.kind === "assistant/reasoning" ? "thought" : "progress",
            text: payload.text,
            tool: undefined,
          },
        ];
      if (event.kind === "skill/loaded")
        return [
          {
            id: event.seq,
            kind: "skill" as const,
            text: `${payload.skill} · ${payload.path}`,
            tool: undefined,
          },
        ];
      return [];
    });
}

export function withLiveThinking(
  events: ActivityEvent[],
  live: { id: string; text: string }[],
) {
  const saved = new Set(
    events
      .filter((e) => e.kind === "assistant/reasoning")
      .map((e) => (e.payload as { id?: string }).id),
  );
  return [
    ...events,
    ...live
      .filter((block) => !saved.has(block.id))
      .map((payload, index) => ({
        seq: Number.MAX_SAFE_INTEGER - live.length + index,
        kind: "assistant/reasoning",
        created: 0,
        payload,
      })),
  ];
}
