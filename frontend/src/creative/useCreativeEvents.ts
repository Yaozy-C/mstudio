import { useEffect } from "react";
export function useCreativeEvents({
  assist,
  arranged,
  reading,
}: {
  assist: () => void;
  arranged: () => void;
  reading: () => void;
}) {
  useEffect(() => {
    window.addEventListener("studio-creative-request", assist);
    window.addEventListener("studio-plan-assembled", arranged);
    window.addEventListener("studio-focus-plan", reading);
    return () => {
      window.removeEventListener("studio-creative-request", assist);
      window.removeEventListener("studio-plan-assembled", arranged);
      window.removeEventListener("studio-focus-plan", reading);
    };
  }, [arranged, reading, assist]);
}
