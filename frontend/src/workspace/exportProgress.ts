/** Ignore malformed events instead of displaying a fabricated percentage. */
export function progressRatio({
  done,
  total,
}: {
  done: number;
  total: number;
}): number | null {
  if (
    !Number.isFinite(done) ||
    !Number.isFinite(total) ||
    total <= 0 ||
    done < 0
  )
    return null;
  return Math.min(done / total, 1);
}
