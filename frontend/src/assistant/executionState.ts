export type ExecutionState = {
  projectId: string;
  turnId: string;
  status:
    | "running"
    | "waiting"
    | "completed"
    | "failed"
    | "cancelled"
    | "interrupted";
  text: string;
  error: string | null;
  revision: number;
};

export function executionActive(
  state: Pick<ExecutionState, "status"> | null | undefined,
) {
  return state?.status === "running" || state?.status === "waiting";
}

// Subscribe first; a cold read must never overwrite a newer live transition.
export function executionSync(options: {
  read: () => Promise<ExecutionState | null>;
  apply: (state: ExecutionState | null) => void;
  error: (error: unknown) => void;
}) {
  let revision = 0;
  let disposed = false;
  let lastRevision = -1;
  const apply = (state: ExecutionState | null) => {
    if (state && state.revision < lastRevision) return;
    if (state) lastRevision = state.revision;
    options.apply(state);
  };
  return {
    receive(state: ExecutionState) {
      if (disposed) return;
      revision++;
      apply(state);
    },
    async refresh() {
      const version = ++revision;
      try {
        const state = await options.read();
        if (!disposed && revision === version) apply(state);
      } catch (error) {
        if (!disposed && revision === version) options.error(error);
      }
    },
    dispose() {
      disposed = true;
      revision++;
    },
  };
}
