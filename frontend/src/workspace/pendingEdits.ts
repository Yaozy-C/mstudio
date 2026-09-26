const pending = new Set<() => void>();
export function registerPendingEdit(flush: () => void) {
  pending.add(flush);
  return () => {
    pending.delete(flush);
  };
}
export function flushPendingEdits() {
  for (const flush of pending) flush();
}
