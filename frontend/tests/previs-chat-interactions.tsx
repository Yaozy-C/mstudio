import { useState } from "react";
import { createRoot } from "react-dom/client";
import { Theme } from "@radix-ui/themes";
import { PrevisChat } from "../src/previs/PrevisChat";
import { newModel } from "../src/previs/types";
import { newConnection } from "../src/models/types";
import type { PrevisEditorState } from "../src/previs/usePrevisEditor";
import "@radix-ui/themes/styles.css";
import "../src/styles/color-tokens.css";
import "../src/styles/motion.css";
import "../src/styles/base.css";
import "../src/styles/frame-theme.css";
import "../src/styles/agent.css";
import "../src/previs/previs.css";

const model = {
  ...newConnection(),
  id: "demo",
  name: "Demo",
  adapter: "codex" as const,
};
const paragraph =
  "预演文件操作 · Read the current scene, adjust the camera, and save the file.\n\n";
const frame = () =>
  new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
let submitted = 0;

function App() {
  const [prompt, setPrompt] = useState("");
  const [partial, setPartial] = useState(paragraph.repeat(40));
  const [busy, setBusy] = useState<"chat" | null>("chat");
  const [results, setResults] = useState<string[]>([]);
  const state = {
    file: newModel("Demo"),
    messages: [],
    busy,
    phase: busy ? "generating" : null,
    pendingPrompt: "",
    agentPartial: partial,
    agentEvents: [],
    seconds: 1,
    stopping: false,
    error: "",
    prompt,
    setPrompt,
    model,
    setModelId: () => {},
    catalog: { profiles: [model] },
    frames: { done: 0, total: 0 },
    lastKind: "chat",
    send: async () => {
      submitted++;
      setPrompt("");
    },
    stop: async () => setBusy(null),
  } as unknown as PrevisEditorState;
  async function check() {
    const checks: string[] = [];
    const assert = (ok: boolean, name: string) => {
      checks.push(`${ok ? "PASS" : "FAIL"}: ${name}`);
      setResults([...checks]);
      if (!ok) throw Error(name);
    };
    try {
      const viewport =
        document.querySelector<HTMLDivElement>(".previs-messages")!;
      const composer = document.querySelector(".previs-composer")!;
      const top = () => composer.getBoundingClientRect().top;
      assert(
        viewport.scrollHeight - viewport.clientHeight - viewport.scrollTop <= 2,
        "initial bottom",
      );
      viewport.scrollTop = 300;
      viewport.dispatchEvent(new Event("scroll"));
      const position = viewport.scrollTop;
      const composerTop = top();
      const outerTop = document.documentElement.scrollTop;
      for (let i = 0; i < 5; i++) {
        setPartial((text) => text + paragraph);
        await frame();
      }
      assert(
        Math.abs(viewport.scrollTop - position) <= 1,
        "streaming preserves upward scroll",
      );
      assert(
        top() === composerTop &&
          document.documentElement.scrollTop === outerTop,
        "composer and outer page stay fixed",
      );
      viewport.scrollTop = viewport.scrollHeight;
      viewport.dispatchEvent(new Event("scroll"));
      setPartial((text) => text + paragraph);
      await frame();
      assert(
        viewport.scrollHeight - viewport.clientHeight - viewport.scrollTop <= 2,
        "returning to bottom resumes following",
      );
      setBusy(null);
      setPrompt("中文输入 test");
      await frame();
      const input = document.querySelector("textarea")!;
      const enter = (options: KeyboardEventInit = {}) =>
        input.dispatchEvent(
          new KeyboardEvent("keydown", {
            bubbles: true,
            cancelable: true,
            key: "Enter",
            keyCode: 13,
            ...options,
          }),
        );
      input.focus();
      input.dispatchEvent(
        new CompositionEvent("compositionstart", { bubbles: true }),
      );
      enter();
      await frame();
      assert(
        submitted === 0 && !!input.value,
        "Chinese composing Enter does not send or clear text",
      );
      input.dispatchEvent(
        new CompositionEvent("compositionend", { bubbles: true }),
      );
      enter({ keyCode: 229 });
      enter({ isComposing: true });
      await frame();
      assert(submitted === 0, "WebKit IME confirmation does not send");
      assert(
        enter({ shiftKey: true }) && submitted === 0,
        "Shift Enter preserves native newline",
      );
      enter();
      await frame();
      assert(
        submitted === 1 && input.value === "",
        "separate Enter sends once",
      );
      enter();
      await frame();
      assert(submitted === 1, "empty Enter does not submit");
    } catch (error) {
      setResults([...checks, String(error)]);
    }
  }
  return (
    <Theme>
      <main style={{ padding: 24 }}>
        <button onClick={() => void check()}>Run checks</button>
        <output
          style={{
            position: "absolute",
            top: 24,
            left: 540,
            whiteSpace: "pre-wrap",
          }}
        >
          {results.join("\n")}
        </output>
        <div style={{ display: "flex", height: 650, width: 480 }}>
          <PrevisChat state={state} />
        </div>
      </main>
    </Theme>
  );
}
createRoot(document.getElementById("root")!).render(<App />);
