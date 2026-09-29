import { expect, test } from "bun:test";
import { newProject } from "../model";
import { asset } from "../production/fixtures.test-helper";
import type { ProductionTask, ProductionSource } from "../production/types";
import {
  generatedJobTracker,
  type JobTrackerHost,
} from "./generatedJobTracker";
import type { GeneratedJob } from "./generatedJobTracking";
const tick = () => new Promise((resolve) => setTimeout(resolve, 0));
function fixture(count: number) {
  let p = newProject("batch");
  const jobs: GeneratedJob[] = [];
  p.production = { drafts: {} };
  for (let i = 0; i < count; i++) {
    const task: ProductionTask = {
      key: `run:${i}`,
      jobId: `job:${i}`,
      kind: "video",
      mode: "single",
      modelId: "video",
      prompt: `Shot ${i}`,
      inputs: [],
      turnId: "turn",
      createdAt: i + 1,
      status: "IN_PROGRESS",
    };
    p.production.drafts![task.key] = task;
    jobs.push({
      id: task.jobId!,
      status: "IN_PROGRESS",
      requestId: `request:${i}`,
      shot: {
        canvasGeneration: { task, x: i * 300, y: 0 },
      } as ProductionSource,
    });
  }
  const host: Omit<JobTrackerHost, "execute"> = {
    get: () => p,
    change: (fn) => {
      p = fn(p);
    },
    flush: async () => {},
    notify: () => {},
    error: (e) => {
      throw new Error(e);
    },
  };
  return { jobs, host };
}
test("100 real tracker jobs finish around a blocked import and never submit generation", async () => {
  const { jobs, host } = fixture(100);
  let release!: () => void;
  const slow = new Promise<void>((r) => {
    release = r;
  });
  let refreshed = 0;
  const imports = new Map<string, number>();
  const tracker = generatedJobTracker(host.get().id, {
    ...host,
    execute: async <T>(
      command: string,
      args: Record<string, unknown>,
    ): Promise<T> => {
      if (command === "list_jobs") return jobs as T;
      const job = jobs.find((job) => job.id === args.id)!;
      if (command === "refresh_job") {
        refreshed++;
        job.status = "COMPLETED";
        return job as T;
      }
      if (command === "import_job_result") {
        imports.set(job.id, (imports.get(job.id) ?? 0) + 1);
        if (job === jobs[0]) await slow;
        return asset(`asset:${job.id}`, "video") as T;
      }
      throw new Error(`Unexpected command ${command}`);
    },
  });
  await tracker.poll();
  await tick();
  expect(refreshed).toBe(100);
  expect(host.get().assets).toHaveLength(99);
  await tracker.poll();
  release();
  await tick();
  expect(host.get().assets).toHaveLength(100);
  expect([...imports.values()].every((n) => n === 1)).toBe(true);
  tracker.stop();
});
test("desktop-managed jobs attach cached partial outputs without competing for network slots", async () => {
  const { jobs, host } = fixture(2);
  jobs[0] = {
    ...jobs[0],
    backgroundManaged: true,
    status: "COMPLETED",
    outputCount: 2,
    assets: [{ id: "partial" }],
    sync: { stage: "import", paused: true, error: "download failed" },
  };
  jobs[1].backgroundManaged = true;
  const commands: string[] = [];
  const tracker = generatedJobTracker(host.get().id, {
    ...host,
    execute: async <T>(command: string): Promise<T> => {
      commands.push(command);
      if (command === "list_jobs") return jobs as T;
      if (command === "import_job_result")
        return asset("partial", "video") as T;
      throw new Error(`Unexpected network command ${command}`);
    },
  });
  await tracker.poll();
  await tick();
  expect(host.get().assets).toHaveLength(1);
  expect(host.get().production!.drafts!["run:0"].trackingPaused).toBe(true);
  expect(host.get().production!.drafts!["run:0"].status).toBe("RECEIVING");
  await tracker.poll();
  await tick();
  expect(commands.filter((c) => c === "import_job_result")).toHaveLength(1);
  tracker.stop();
});
test("paused tasks reconcile desktop-confirmed cancellation without querying the provider", async () => {
  const { jobs, host } = fixture(1);
  const task = host.get().production!.drafts!["run:0"];
  task.status = "CANCEL_REQUESTED";
  task.error = "Request was cancelled";
  task.trackingPaused = true;
  jobs[0] = { ...jobs[0], backgroundManaged: true, status: "CANCELLED" };
  const commands: string[] = [];
  const tracker = generatedJobTracker(host.get().id, {
    ...host,
    execute: async <T>(command: string): Promise<T> => {
      commands.push(command);
      if (command === "list_jobs") return jobs as T;
      throw new Error(`Unexpected provider query ${command}`);
    },
  });
  await tracker.poll();
  const updated = host.get().production!.drafts!["run:0"];
  expect(updated.status).toBe("CANCELLED");
  expect(updated.error).toBeUndefined();
  expect(updated.trackingPaused).toBe(false);
  expect(commands).toEqual(["list_jobs"]);
  tracker.stop();
});
test("project switch drops pending document mutations, then durable cache reattaches on reopen", async () => {
  const { jobs, host } = fixture(1);
  jobs[0] = {
    ...jobs[0],
    backgroundManaged: true,
    status: "COMPLETED",
    assets: [{ id: "ready" }],
  };
  let release!: () => void;
  const slow = new Promise<void>((r) => {
    release = r;
  });
  const execute: JobTrackerHost["execute"] = async <T>(
    command: string,
  ): Promise<T> => {
    if (command === "list_jobs") return jobs as T;
    await slow;
    return asset("ready", "video") as T;
  };
  const tracker = generatedJobTracker(host.get().id, { ...host, execute });
  await tracker.poll();
  await tick();
  tracker.stop();
  release();
  await tick();
  expect(host.get().assets).toHaveLength(0);
  const restored = generatedJobTracker(host.get().id, { ...host, execute });
  await restored.poll();
  await tick();
  expect(host.get().assets).toHaveLength(1);
  expect(host.get().production!.drafts!["run:0"].status).toBe("COMPLETED");
  restored.stop();
});
