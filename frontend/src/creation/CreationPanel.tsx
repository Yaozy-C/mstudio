import { useState } from "react";
import { DockPanel } from "../ui/DockPanel";
import { type Project } from "../model";
import type { PlaybackClock } from "../timeline/clock";
import { SpeechPanel } from "./SpeechPanel";
import { SubtitlePanel } from "./SubtitlePanel";
export function CreationPanel({
  project,
  clock,
  change,
  onClose,
  initialTab = "captions",
}: {
  project: Project;
  clock: PlaybackClock;
  change: (f: (p: Project) => Project) => void;
  onClose: () => void;
  initialTab?: string;
}) {
  const [tab, setTab] = useState(initialTab);
  return (
    <DockPanel id="creation" title="字幕与配音" onClose={onClose}>
      <div className="creation-tabs">
        {[
          ["captions", "字幕"],
          ["voice", "配音"],
        ].map(([id, label]) => (
          <button
            key={id}
            className={tab === id ? "active" : ""}
            onClick={() => setTab(id)}
          >
            {label}
          </button>
        ))}
      </div>
      {tab === "captions" && (
        <SubtitlePanel project={project} change={change} clock={clock} />
      )}
      {tab === "voice" && (
        <SpeechPanel project={project} change={change} clock={clock} />
      )}
    </DockPanel>
  );
}
