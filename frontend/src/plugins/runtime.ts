import { Context, type Fiber } from "cordis";
import { bridge } from "../bridge";
const definitions = [
  {
    id: "assistant",
    commands: [
      "assistant_chat",
      "agent_catalog",
      "model_catalog",
      "cancel_assistant",
      "agent_history",
    ],
  },
  {
    id: "generation",
    commands: [
      "submit_job",
      "refresh_job",
      "cancel_job",
      "resolve_unknown_job",
      "list_jobs",
      "import_job_result",
      "upload_references",
    ],
  },
  {
    id: "export",
    commands: [
      "render_video",
      "store_caption_image",
      "store_caption_animation",
    ],
  },
];
export class Runtime {
  constructor(private invoke: typeof bridge = bridge) {}
  private pending: Promise<unknown> = Promise.resolve();
  private context = new Context();
  private fibers = new Map<string, Fiber>();
  private commands = new Map<
    string,
    (args: Record<string, unknown>) => Promise<unknown>
  >();
  enabled = (id: string) => this.fibers.has(id);
  toggle(id: string, enabled: boolean) {
    const task = this.pending
      .catch(() => {})
      .then(() => this.apply(id, enabled));
    this.pending = task;
    return task;
  }
  private async apply(id: string, enabled: boolean) {
    if (this.enabled(id) === enabled) return;
    const def = definitions.find((p) => p.id === id);
    if (!def) throw new Error("插件不存在");
    if (enabled) {
      const fiber = this.context.plugin({
        name: id,
        apply: (ctx) => {
          ctx.effect(() => {
            for (const command of def.commands)
              this.commands.set(command, (args) => this.invoke(command, args));
            return () => {
              for (const command of def.commands) this.commands.delete(command);
            };
          });
        },
      });
      await fiber;
      this.fibers.set(id, fiber);
    } else {
      await this.fibers.get(id)?.dispose();
      this.fibers.delete(id);
    }
  }
  async execute<T>(command: string, args: Record<string, unknown>): Promise<T> {
    const handler = this.commands.get(command);
    if (!handler) throw new Error("运行模块尚未初始化");
    return (await handler(args)) as T;
  }
}
export const runtime = new Runtime();
export async function initPlugins() {
  for (const def of definitions) await runtime.toggle(def.id, true);
}
