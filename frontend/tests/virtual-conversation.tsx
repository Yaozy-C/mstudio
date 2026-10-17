import { useState } from "react";
import { createRoot } from "react-dom/client";
import { VirtualConversation } from "../src/assistant/VirtualConversation";
import { AgentActivityFeed } from "../src/assistant/AgentActivityFeed";
import { checkConversation, type Entry } from "./virtual-conversation-checks";
import "../src/styles/agent.css";
import "../src/styles/agent-activity.css";

const initial: Entry[] = Array.from({ length: 200 }, (_, index) => ({
  key: `message-${index}`,
  text: `历史消息 ${index}：` + "核对参考画面与镜头。".repeat((index % 4) + 1),
}));
function Test() {
  const [viewport, setViewport] = useState<HTMLDivElement | null>(null);
  const [entries, setEntries] = useState(initial);
  const [result, setResult] = useState("准备检查 200 条历史消息。");
  const [busy, setBusy] = useState(false);
  return (
    <>
      <h1>历史虚拟化回归测试</h1>
      <button
        disabled={busy}
        onClick={async () => {
          setBusy(true);
          setResult("运行中…");
          try {
            const checks = await checkConversation(viewport!, setEntries);
            setResult(checks.join("\n"));
          } catch (error) {
            setResult(`FAIL ${error}`);
          } finally {
            setBusy(false);
          }
        }}
      >
        运行回归
      </button>
      <pre role="status">{result}</pre>
      <div
        ref={setViewport}
        className="agent-messages"
        style={{ width: 520, height: 460, border: "1px solid #ccc" }}
      >
        <VirtualConversation
          entries={entries}
          viewport={viewport}
          scrollRequest={0}
        >
          {(entry) => (
            <article className="agent-message">
              <h3>{entry.key}</h3>
              <p>{entry.text}</p>
              <AgentActivityFeed
                seconds={20}
                running={false}
                events={Array.from({ length: 10 }, (_, index) => [
                  {
                    seq: index * 2 + 1,
                    created: index,
                    kind: "tool/call",
                    payload: {
                      callId: `read-${index}`,
                      name: "mstudio_read_image",
                      arguments: { assetId: `asset-${entry.key}-${index}` },
                    },
                  },
                  {
                    seq: index * 2 + 2,
                    created: index,
                    kind: "tool/result",
                    payload: { callId: `read-${index}`, result: { ok: true } },
                  },
                ]).flat()}
                children={[
                  {
                    projectId: "fixture",
                    parentTurn: entry.key,
                    turnId: `child-${entry.key}`,
                    childId: "child",
                    callId: "read-0",
                    agentId: "image",
                    state: "completed",
                    phase: {},
                    note: "",
                    started: 1,
                    updated: 2,
                    events: [
                      {
                        seq: 101,
                        created: 1,
                        kind: "assistant/reasoning",
                        payload: { text: "子任务完整思考" },
                      },
                    ],
                  },
                ]}
              />
            </article>
          )}
        </VirtualConversation>
      </div>
    </>
  );
}
createRoot(document.getElementById("root")!).render(<Test />);
