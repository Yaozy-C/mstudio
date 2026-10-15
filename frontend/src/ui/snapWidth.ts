/** Screen-pixel thresholds; hysteresis prevents jitter around a stop. */
export function snapWidth(
  value: number,
  stops: number[],
  locked: number | null,
) {
  if (
    locked !== null &&
    stops.includes(locked) &&
    Math.abs(value - locked) <= 12
  )
    return locked;
  const nearest = stops.reduce<number | null>(
    (best, stop) =>
      Math.abs(value - stop) <= 6 &&
      (best === null || Math.abs(value - stop) < Math.abs(value - best))
        ? stop
        : best,
    null,
  );
  return nearest;
}
