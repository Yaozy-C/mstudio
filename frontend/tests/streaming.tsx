// Manual browser regression: open /tests/streaming.html on the Vite dev server.
import { useState } from "react";
import { createRoot } from "react-dom/client";
import {
  AssistantRuntimeProvider,
  ThreadPrimitive,
  useLocalRuntime,
} from "@assistant-ui/react";
import { AgentMessage, MessageContext } from "../src/assistant/AgentMessage";
import type { Project } from "../src/model";
import "../src/styles/agent.css";
import "../src/styles/agent-mentions.css";
const settle = () =>
  new Promise<void>((r) =>
    requestAnimationFrame(() => requestAnimationFrame(() => r())),
  );
function Test() {
  const [result, setResult] = useState("点击开始，模拟 24 次流式更新。");
  const [adapter] = useState(() => ({
    async *run() {
      let paragraph: Element | null = null;
      let details: HTMLDetailsElement | null = null;
      let lastTop = 0;
      let stable = true,
        monotonic = true,
        keptOpen = true,
        respectedScroll = true;
      let heldTop = 0;
      for (let i = 0; i < 24; i++) {
        yield {
          content: [
            {
              type: "text" as const,
              text:
                "这段文字应该保持同一个 DOM 节点。\n\n" +
                Array.from(
                  { length: i + 1 },
                  (_, n) =>
                    `段落 ${n + 1}：持续输出新的内容，已有段落不应卸载或闪动。`,
                ).join("\n\n"),
            },
          ],
          metadata: { custom: { turnId: "fixture", agentName: "测试助手" } },
        };
        await new Promise((r) => setTimeout(r, 80));
        await settle();
        const current =
          Array.from(document.querySelectorAll(".assistant p")).find((p) =>
            p.textContent?.startsWith("这段文字"),
          ) ?? null;
        const disclosure =
          document.querySelector<HTMLDetailsElement>(".assistant details")!;
        const viewport =
          document.querySelector<HTMLElement>(".agent-messages")!;
        if (paragraph) stable &&= current === paragraph;
        else paragraph = current;
        if (!details) {
          details = disclosure;
          details.open = true;
        } else keptOpen &&= details === disclosure && disclosure.open;
        if (i < 16) monotonic &&= viewport.scrollTop >= lastTop - 1;
        if (i === 16) {
          viewport.scrollTop -= 100;
          viewport.dispatchEvent(new Event("scroll"));
          heldTop = viewport.scrollTop;
        }
        if (i > 16)
          respectedScroll &&= Math.abs(viewport.scrollTop - heldTop) <= 1;
        lastTop = viewport.scrollTop;
      }
      setResult(
        [
          stable ? "PASS 文本节点未重挂载" : "FAIL 文本节点重挂载",
          monotonic ? "PASS 跟随滚动没有反向跳动" : "FAIL 跟随滚动反向跳动",
          keptOpen ? "PASS 执行记录保持展开" : "FAIL 执行记录重置",
          respectedScroll ? "PASS 向上阅读时不抢滚动" : "FAIL 抢滚动",
        ].join("\n"),
      );
    },
  }));
  const runtime = useLocalRuntime(adapter);
  return (
    <>
      <h1>助手流式回归测试</h1>
      <button
        onClick={() =>
          runtime.thread.append({
            role: "user",
            content: [{ type: "text", text: "测试流式回答" }],
          })
        }
      >
        开始测试
      </button>
      <pre role="status">{result}</pre>
      <div style={{ width: 340, height: 400, border: "1px solid #ccc" }}>
        <MessageContext.Provider
          value={{
            project: { id: "fixture", nodes: [] } as unknown as Project,
            sent: {},
            targets: {},
            settings: () => {},
            follow: () => {},
          }}
        >
          <AssistantRuntimeProvider runtime={runtime}>
            <ThreadPrimitive.Root className="agent-thread">
              <ThreadPrimitive.Viewport className="agent-messages">
                <ThreadPrimitive.Messages
                  components={{
                    UserMessage: AgentMessage,
                    AssistantMessage: AgentMessage,
                  }}
                />
              </ThreadPrimitive.Viewport>
            </ThreadPrimitive.Root>
          </AssistantRuntimeProvider>
        </MessageContext.Provider>
      </div>
    </>
  );
}
createRoot(document.getElementById("root")!).render(<Test />);
