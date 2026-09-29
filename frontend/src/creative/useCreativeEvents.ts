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
    window.addEventListener("studio-screenplay-assembled", arranged);
    window.addEventListener("studio-focus-screenplay", reading);
    return () => {
      window.removeEventListener("studio-creative-request", assist);
      window.removeEventListener("studio-screenplay-assembled", arranged);
      window.removeEventListener("studio-focus-screenplay", reading);
    };
  }, [arranged, reading, assist]);
}
