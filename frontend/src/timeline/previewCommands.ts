export type PreviewCommand =
  | { action: "play" | "pause" }
  | { action: "seek"; frame: number }
  | { action: "rate"; rate: number };
/** One acknowledged operation at a time; play/pause are ordering barriers. */
export class PreviewCommands {
  private pending: PreviewCommand[] = [];
  private disposed = false;
  busy = false;
  revision = 0;
  constructor(
    private send: (command: PreviewCommand) => Promise<void>,
    private fail: (error: unknown) => void,
  ) {}
  push = (command: PreviewCommand) => {
    if (this.disposed) return;
    this.revision++;
    const { action } = command;
    if (
      (action === "seek" || action === "rate") &&
      this.pending.at(-1)?.action === action
    )
      this.pending[this.pending.length - 1] = command;
    else this.pending.push(command);
    void this.drain();
  };
  dispose() {
    this.disposed = true;
    this.revision++;
    this.pending = [];
  }
  private async drain() {
    if (this.busy || this.disposed) return;
    this.busy = true;
    try {
      while (!this.disposed && this.pending.length)
        await this.send(this.pending.shift()!);
    } catch (error) {
      if (!this.disposed) {
        this.dispose();
        this.fail(error);
      }
    } finally {
      this.busy = false;
    }
  }
}
