import { TransitionPanel } from "../timeline/TransitionPanel";
import "../styles/inspector-desk.css";
import "../styles/creation-panel.css";
import { t, useLanguage } from "../i18n";
import { Subtitles, Microphone } from "@phosphor-icons/react";
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
  useLanguage();
  const [tab, setTab] = useState(initialTab);
  if (initialTab.startsWith("transition:")) {
    const [leftId, rightId] = JSON.parse(initialTab.slice(11)) as [
      string,
      string,
    ];
    return (
      <TransitionPanel
        {...{ project, clock, change, onClose, leftId, rightId }}
      />
    );
  }
  return (
    <DockPanel id="creation" title={t("字幕与配音")} onClose={onClose}>
      <div className="creation-tabs inspector-tabs">
        {(
          [
            ["captions", t("字幕"), Subtitles],
            ["voice", t("配音"), Microphone],
          ] as const
        ).map(([id, label, Icon]) => (
          <button
            key={id}
            className={tab === id ? "active" : ""}
            aria-pressed={tab === id}
            onClick={() => setTab(id)}
          >
            <Icon size={22} />
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
