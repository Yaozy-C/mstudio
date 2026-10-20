import { useEffect, useState, useSyncExternalStore } from "react";
import { Channel } from "@tauri-apps/api/core";
import { bridge } from "../../bridge";
import { createAnalysisController } from "./controller";

export function useVideoAnalysis() {
  const [controller] = useState(() =>
    createAnalysisController(bridge, (receive) => {
      const channel = new Channel<import("./types").ProgressUpdate>();
      channel.onmessage = receive;
      return channel;
    }),
  );
  const state = useSyncExternalStore(
    controller.subscribe,
    controller.getSnapshot,
    controller.getSnapshot,
  );
  useEffect(() => () => controller.dispose(), [controller]);
  return { controller, state };
}
