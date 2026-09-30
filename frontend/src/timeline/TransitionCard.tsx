import { useRef } from "react";
export function TransitionCard({
  kind,
  title,
  selected,
  onClick,
}: {
  kind: string;
  title: string;
  selected: boolean;
  onClick: () => void;
}) {
  const video = useRef<HTMLVideoElement>(null);
  const play = () => {
    void video.current?.play().catch(() => {});
  };
  const stop = () => {
    if (video.current) {
      video.current.pause();
      video.current.currentTime = 0;
    }
  };
  return (
    <button
      className="transition-card"
      aria-pressed={selected}
      onClick={onClick}
      onMouseEnter={play}
      onMouseLeave={stop}
      onFocus={play}
      onBlur={stop}
    >
      <video
        ref={video}
        src={`/transition-previews/${kind}.mp4`}
        poster={`/transition-previews/${kind}.jpg`}
        muted
        loop
        playsInline
        preload="none"
        aria-hidden="true"
      />
      <span>{title}</span>
      <span className="transition-card-play" aria-hidden="true">
        ▶
      </span>
    </button>
  );
}
