import { useState } from "react";
import { Popover } from "@radix-ui/themes";
import { Check, FilmStrip, MusicNotes, Subtitles } from "@phosphor-icons/react";
import { IconButton } from "../ui/IconButton";
import { t } from "../i18n";

/** Track names remain editable without occupying the compact track rail. */
export function TrackIdentity({
  name,
  kind,
  rename,
}: {
  name: string;
  kind: "video" | "audio" | "caption";
  rename: (name: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState(name);
  const icon =
    kind === "video" ? FilmStrip : kind === "audio" ? MusicNotes : Subtitles;
  const label = t("{v0} 轨道名称", { v0: name });
  return (
    <Popover.Root
      open={open}
      onOpenChange={(next) => {
        if (next) setDraft(name);
        setOpen(next);
      }}
    >
      <Popover.Trigger>
        <IconButton icon={icon} label={label} />
      </Popover.Trigger>
      <Popover.Content
        className="track-name-popover"
        side="right"
        sideOffset={8}
      >
        <form
          onSubmit={(event) => {
            event.preventDefault();
            if (draft.trim()) {
              rename(draft.trim());
              setOpen(false);
            }
          }}
        >
          <input
            aria-label={label}
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && event.nativeEvent.isComposing)
                event.preventDefault();
            }}
            onFocus={(event) => event.target.select()}
          />
          <IconButton
            icon={Check}
            label={t("保存")}
            type="submit"
            disabled={!draft.trim()}
          />
        </form>
      </Popover.Content>
    </Popover.Root>
  );
}
