type Command = { action: string; frame?: number };

/** Coalesce pending seeks without moving them across play/pause boundaries. */
export class PreviewCommands {
  private pending: Command[] = [];
  busy = false;
  revision = 0;
  constructor(
    private send: (command: Command) => Promise<void>,
    private fail: (error: unknown) => void,
  ) {}
  push = (action: string, frame?: number) => {
    this.revision++;
    const command = { action, frame };
    if (action === "seek" && this.pending.at(-1)?.action === "seek")
      this.pending[this.pending.length - 1] = command;
    else this.pending.push(command);
    void this.drain();
  };
  clear() {
    this.pending = [];
  }
  private async drain() {
    if (this.busy) return;
    this.busy = true;
    try {
      while (this.pending.length) await this.send(this.pending.shift()!);
    } catch (error) {
      this.clear();
      this.fail(error);
    } finally {
      this.busy = false;
    }
  }
}
