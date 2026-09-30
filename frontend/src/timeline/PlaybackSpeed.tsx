import { StudioSelect } from "../ui/StudioSelect";
import { useSyncExternalStore } from "react";
import { t, useLanguage } from "../i18n";
import type { PlaybackClock } from "./clock";
export function PlaybackSpeed({ clock }: { clock: PlaybackClock }) {
  useLanguage();
  const rate = useSyncExternalStore(clock.subscribe, clock.getRate);
  return (
    <div className="preview-speed" title={t("仅影响预览，不改变成片速度")}>
      <StudioSelect
        label={t("播放倍速")}
        value={String(rate)}
        onValueChange={(value) => clock.setRate(Number(value))}
        options={Array.from({ length: 8 }, (_, i) => {
          const value = (i + 1) / 4;
          return { value: String(value), label: `${value}×` };
        })}
      />
    </div>
  );
}
