/** Playback is independent of the project document: no saves or canvas renders per frame. */
export class PlaybackClock {
  private state = { time: 0, playing: false };
  private listeners = new Set<() => void>();
  private frame = 0;
  private previous = 0;
  seekRevision = 0;
  ready = true;
  buffering = false;
  setBuffering = (value: boolean) => {
    if (this.buffering !== value) {
      this.buffering = value;
      this.listeners.forEach((fn) => fn());
    }
  };
  private transport: {
    play: () => void;
    pause: () => void;
    seek: (time: number) => void;
  } | null = null;
  attachTransport(transport: NonNullable<PlaybackClock["transport"]>) {
    this.pause();
    this.transport = transport;
    return () => {
      if (this.transport === transport) {
        this.transport = null;
        this.publish(this.state.time, false);
      }
    };
  }
  acceptTransportState(time: number, playing: boolean) {
    if (this.transport)
      this.publish(Math.min(this.total, Math.max(0, time)), playing);
  }
  private source: (() => number) | null = null;
  setSource = (source: () => number) => {
    this.source = source;
  };
  clearSource = (source: () => number) => {
    if (this.source === source) this.source = null;
  };
  total = 0;
  fps = 30;
  getSnapshot = () => this.state;
  subscribe = (fn: () => void) => {
    this.listeners.add(fn);
    return () => {
      this.listeners.delete(fn);
    };
  };
  private publish(time: number, playing = this.state.playing) {
    if (time === this.state.time && playing === this.state.playing) return;
    this.state = { time, playing };
    this.listeners.forEach((fn) => fn());
  }
  configure(total: number, fps: number) {
    const changedFps = this.fps !== fps;
    this.total = total;
    this.fps = fps;
    if (changedFps) this.listeners.forEach((fn) => fn());
    if (this.state.time > total) this.seek(total);
    if (!total) this.pause();
  }
  seek = (time: number) => {
    const value = Math.max(
      0,
      Math.min(this.total, Math.round(time * this.fps) / this.fps),
    );
    if (value === this.state.time) return;
    this.seekRevision++;
    this.transport?.seek(value);
    this.publish(value);
    this.previous = performance.now();
  };
  step = (frames: number) => {
    this.pause();
    this.seek(this.state.time + frames / this.fps);
  };
  play = () => {
    if (!this.ready || !this.total || this.state.playing) return;
    if (this.state.time >= this.total) this.seek(0);
    this.publish(this.state.time, true);
    if (this.transport) {
      this.transport.play();
      return;
    }
    this.previous = performance.now();
    this.frame = requestAnimationFrame(this.tick);
  };
  private tick = (now: number) => {
    if (!this.state.playing) return;
    const time = Math.min(
      this.total,
      this.source
        ? this.source()
        : this.state.time + Math.max(0, (now - this.previous) / 1000),
    );
    this.previous = now;
    this.publish(time, time < this.total);
    if (time < this.total) this.frame = requestAnimationFrame(this.tick);
  };
  pause = () => {
    cancelAnimationFrame(this.frame);
    if (this.state.playing) this.transport?.pause();
    this.publish(this.state.time, false);
  };
  toggle = () => {
    if (this.state.playing) this.pause();
    else this.play();
  };
  dispose = () => {
    this.pause();
    this.listeners.clear();
  };
}
