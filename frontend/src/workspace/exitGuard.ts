export function createExitGuard(
  save: () => Promise<unknown>,
  finish: () => Promise<unknown>,
) {
  let state = { busy: false, error: "" };
  let pending: Promise<void> | undefined;
  const listeners = new Set<() => void>();
  const publish = (next: typeof state) => {
    state = next;
    listeners.forEach((listener) => listener());
  };
  function request() {
    if (pending) return pending;
    publish({ ...state, busy: true });
    pending = Promise.resolve()
      .then(save)
      .then(finish)
      .then(() => publish({ busy: false, error: "" }))
      .catch((error) => {
        publish({ busy: false, error: `退出前保存失败：${String(error)}` });
      })
      .finally(() => {
        pending = undefined;
      });
    return pending;
  }
  return {
    request,
    getSnapshot: () => state,
    subscribe(listener: () => void) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    dismiss() {
      if (!pending) publish({ busy: false, error: "" });
    },
  };
}
