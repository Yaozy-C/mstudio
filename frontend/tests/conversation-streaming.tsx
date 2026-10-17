import { useState } from "react";
import { createRoot } from "react-dom/client";
import {
  AssistantRuntimeProvider,
  ThreadPrimitive,
  useLocalRuntime,
  type ThreadMessageLike,
} from "@assistant-ui/react";
import { MessageContext } from "../src/assistant/AgentMessage";
import { ConversationTimeline } from "../src/assistant/ConversationTimeline";
import { newProject } from "../src/model";
import type { ProductionController } from "../src/production/useProduction";
import type { ProductionTask } from "../src/production/types";
import { settle } from "./virtual-conversation-checks";
import "../src/styles/agent.css";
import "../src/styles/agent-activity.css";

const history: ThreadMessageLike[] = Array.from({ length: 80 }, (_, index) => ({
  id: `history-${index}`,
  role: index % 2 ? "assistant" : "user",
  content:
    `历史 ${index}\n\n` + "已经保存的回答。".repeat((index % 4) * 20 + 5),
  createdAt: new Date(index * 1000),
  metadata: { custom: { turnId: `turn-${Math.floor(index / 2)}` } },
}));
const project = newProject("Virtual history integration");
const tasks: ProductionTask[] = Array.from({ length: 14 }, (_, index) => ({
  key: `task-${index}`,
  kind: "image",
  prompt: `生成任务 ${index}`,
  modelId: "fixture-model",
  inputs: [],
  mode: "multi",
  status: "COMPLETED",
  turnId: index === 13 ? "direct:last" : "turn-39",
  createdAt: 79500 + index,
}));

function Test() {
  const [result, setResult] = useState(
    "准备检查实际聊天历史、任务分页和流式输出。",
  );
  const [opened, setOpened] = useState("");
  const [canvas] = useState(
    () =>
      ({
        runs: tasks,
        showTask: (task: ProductionTask) => setOpened(task.key),
      }) as ProductionController,
  );
  const [adapter] = useState(() => ({
    async *run() {
      const viewport = document.querySelector<HTMLElement>(".agent-messages")!;
      let paragraph: Element | null = null;
      let stable = true;
      let pinned = true;
      let heldTop = 0;
      let respectedScroll = true;
      for (let step = 0; step < 28; step++) {
        yield {
          content: [
            {
              type: "text" as const,
              text:
                "稳定流式段落。\n\n" +
                Array.from(
                  { length: step + 1 },
                  (_, index) => `段落 ${index}：新增输出应当保持已有文本节点。`,
                ).join("\n\n"),
            },
          ],
          metadata: {
            custom: { turnId: "stream-turn", agentName: "测试助手" },
          },
        };
        await settle();
        const current =
          Array.from(viewport.querySelectorAll(".assistant p")).find(
            (element) => element.textContent?.startsWith("稳定流式段落。"),
          ) ?? null;
        if (paragraph) stable &&= current === paragraph;
        else paragraph = current;
        if (step < 18)
          pinned &&=
            viewport.scrollHeight -
              viewport.clientHeight -
              viewport.scrollTop <=
            2;
        if (step === 18) {
          viewport.scrollTop -= 150;
          viewport.dispatchEvent(new Event("scroll"));
          await settle();
          heldTop = viewport.scrollTop;
        }
        if (step > 18)
          respectedScroll &&= Math.abs(viewport.scrollTop - heldTop) <= 2;
      }
      setResult(
        (previous) =>
          previous +
          "\n" +
          [
            stable && paragraph
              ? "PASS 流式文本节点不重挂载"
              : "FAIL 流式文本节点重挂载",
            pinned
              ? "PASS 开始新回答及流式输出跟随底部"
              : "FAIL 流式输出未跟随底部",
            respectedScroll ? "PASS 阅读历史时不抢滚动" : "FAIL 流式输出抢滚动",
          ].join("\n"),
      );
    },
  }));
  const runtime = useLocalRuntime(adapter, { initialMessages: history });
  const start = async () => {
    const checks: string[] = [];
    const check = (condition: unknown, label: string) => {
      if (!condition) throw new Error(label);
      checks.push(`PASS ${label}`);
    };
    try {
      const viewport = document.querySelector<HTMLElement>(".agent-messages")!;
      check(
        viewport.querySelectorAll(".agent-message").length < 12,
        "实际 80 条聊天历史仅挂载可见消息",
      );
      const direct = viewport.querySelector<HTMLButtonElement>(
        '[data-conversation-key="task-13"] .generation-task-entry',
      )!;
      check(direct, "独立生成任务仍按时间混排");
      direct.click();
      await settle();
      const pagination = viewport.querySelector(".generation-pagination")!;
      pagination.querySelectorAll<HTMLButtonElement>("button")[1].click();
      await settle();
      check(pagination.textContent?.includes("2 / 2"), "关联生成任务支持翻页");
      viewport.scrollTop = 0;
      viewport.dispatchEvent(new Event("scroll"));
      await settle();
      viewport.scrollTop = viewport.scrollHeight;
      viewport.dispatchEvent(new Event("scroll"));
      await settle();
      await settle();
      check(
        viewport
          .querySelector(".generation-pagination")
          ?.textContent?.includes("2 / 2"),
        "离屏再返回保留生成任务页码",
      );
      viewport.scrollTop = 0;
      viewport.dispatchEvent(new Event("scroll"));
      await settle();
      check(
        viewport.querySelector('[data-conversation-key="history-0"]'),
        "历史消息可回溯",
      );
      setResult(checks.join("\n"));
      runtime.thread.append({
        role: "user",
        content: [{ type: "text", text: "开始流式测试" }],
      });
    } catch (error) {
      setResult(checks.join("\n") + `\nFAIL ${error}`);
    }
  };
  return (
    <>
      <h1>虚拟历史流式集成测试</h1>
      <button onClick={start}>运行集成回归</button>
      <pre role="status">{result}</pre>
      <p>已打开任务：{opened}</p>
      <div style={{ width: 520, height: 460, border: "1px solid #ccc" }}>
        <MessageContext.Provider
          value={{
            project,
            sent: {},
            targets: {},
            agents: [],
            canvas,
            settings: () => {},
            follow: () => {},
            saveAttachment: () => {},
          }}
        >
          <AssistantRuntimeProvider runtime={runtime}>
            <ThreadPrimitive.Root className="agent-thread">
              <ConversationTimeline canvas={canvas} />
            </ThreadPrimitive.Root>
          </AssistantRuntimeProvider>
        </MessageContext.Provider>
      </div>
    </>
  );
}
createRoot(document.getElementById("root")!).render(<Test />);
