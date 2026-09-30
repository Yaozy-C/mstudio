import type { PlaybackClock } from "./clock";
import { PreviewCommands, type PreviewCommand } from "./previewCommands";
export type PreviewStatus = {
  frame: number;
  playing: boolean;
  total: number;
  rate: number;
  phase: string;
  error: string | null;
};
type Backend = {
  open: () => Promise<void>;
  control: (
    command: PreviewCommand | { action: "close" },
  ) => Promise<PreviewStatus>;
  status: () => Promise<PreviewStatus>;
};
export type PreviewPhase = "loading" | "ready" | "error" | "closed";
/** Owns one preview session. Late completions never affect a replacement session. */
export class NativePreviewController {
  phase: PreviewPhase = "loading";
  private commands: PreviewCommands;
  private polling = false;
  private timer: ReturnType<typeof setInterval> | undefined;
  private detach: () => void;
  constructor(
    private clock: PlaybackClock,
    private fps: number,
    private backend: Backend,
    private changed: (phase: PreviewPhase, error?: unknown) => void,
  ) {
    this.commands = new PreviewCommands(async (command) => {
      const revision = this.commands.revision;
      const status = await backend.control(command);
      if (this.phase === "ready" && revision === this.commands.revision)
        this.accept(status);
    }, this.fail);
    this.detach = clock.attachTransport({
      play: () => this.enqueue({ action: "play" }),
      pause: () => this.enqueue({ action: "pause" }),
      seek: (time) =>
        this.enqueue({ action: "seek", frame: Math.round(time * fps) }),
      rate: (rate) => this.enqueue({ action: "rate", rate }),
    });
    clock.ready = false;
    changed("loading");
  }
  private enqueue(command: PreviewCommand) {
    // While loading, clock retains the latest desired time/rate for initialization.
    if (this.phase === "ready") this.commands.push(command);
  }
  async start() {
    try {
      await this.backend.open();
      if (this.phase !== "loading") return;
      this.phase = "ready";
      this.commands.push({ action: "rate", rate: this.clock.getRate() });
      this.commands.push({
        action: "seek",
        frame: Math.round(this.clock.getSnapshot().time * this.fps),
      });
      this.timer = setInterval(() => {
        void this.poll();
      }, 50);
    } catch (error) {
      this.fail(error);
    }
  }
  private accept(status: PreviewStatus) {
    if (status.error) {
      this.fail(status.error);
      return;
    }
    this.clock.acceptTransportState(
      status.phase === "ended" ? this.clock.total : status.frame / this.fps,
      status.playing,
    );
  }
  async poll() {
    if (this.phase !== "ready" || this.polling || this.commands.busy) return;
    this.polling = true;
    const revision = this.commands.revision;
    try {
      const status = await this.backend.status();
      if (
        this.phase === "ready" &&
        revision === this.commands.revision &&
        !this.commands.busy
      )
        this.accept(status);
    } catch (error) {
      this.fail(error);
    } finally {
      this.polling = false;
    }
  }
  acceptFrame = (position: number) => {
    if (this.phase !== "ready" || this.commands.busy) return false;
    const state = this.clock.getSnapshot();
    const target = Math.max(
      0,
      Math.min(
        Math.ceil(this.clock.total * this.fps) - 1,
        Math.round(state.time * this.fps),
      ),
    );
    return state.playing || position === target;
  };
  presented = () => {
    if (this.phase !== "ready") return;
    this.clock.ready = true;
    this.changed("ready");
  };
  fail = (error: unknown) => {
    if (this.phase === "closed" || this.phase === "error") return;
    this.phase = "error";
    this.commands.dispose();
    clearInterval(this.timer);
    this.clock.ready = false;
    this.clock.acceptTransportState(this.clock.getSnapshot().time, false);
    this.changed("error", error);
    void this.backend.control({ action: "close" }).catch(() => {});
  };
  dispose() {
    if (this.phase === "closed") return;
    this.phase = "closed";
    this.commands.dispose();
    clearInterval(this.timer);
    this.detach();
    void this.backend.control({ action: "close" }).catch(() => {});
  }
}
