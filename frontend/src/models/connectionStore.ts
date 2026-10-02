import { useEffect, useState } from "react";
import { bridge, native } from "../bridge";
import { modelsChanged } from "./useModels";
export type ServiceKind =
  | "openai-compatible"
  | "openai-responses"
  | "gemini-native"
  | "anthropic-native"
  | "fal"
  | "http-json"
  | "codex";
export type ServiceConnection = {
  id: string;
  name: string;
  kind: ServiceKind;
  endpoint: string;
  hasKey: boolean;
  modelCount: number;
};
export const textServiceKinds: ServiceKind[] = [
  "codex",
  "openai-compatible",
  "openai-responses",
  "gemini-native",
  "anthropic-native",
];
export const imageServiceKinds: ServiceKind[] = [
  "gemini-native",
  "fal",
  "http-json",
];
export const videoServiceKinds: ServiceKind[] = ["fal", "http-json"];
export const serviceLabels: Record<ServiceKind, string> = {
  "openai-compatible": "Chat Completions",
  "openai-responses": "Responses",
  "gemini-native": "Gemini 原生",
  "anthropic-native": "Claude 原生",
  fal: "fal",
  "http-json": "自定义 HTTP",
  codex: "Codex",
};
export const servicePresets = [
  { name: "Codex", kind: "codex", endpoint: "codex://local" },
  {
    name: "OpenAI",
    kind: "openai-responses",
    endpoint: "https://api.openai.com/v1",
  },
  {
    name: "Google",
    kind: "gemini-native",
    endpoint: "https://generativelanguage.googleapis.com",
  },
  {
    name: "Anthropic",
    kind: "anthropic-native",
    endpoint: "https://api.anthropic.com/v1",
  },
  {
    name: "DeepSeek",
    kind: "openai-compatible",
    endpoint: "https://api.deepseek.com",
  },
  { name: "fal", kind: "fal", endpoint: "https://queue.fal.run" },
  { name: "其他对话服务", kind: "openai-compatible", endpoint: "" },
  { name: "自定义 HTTP", kind: "http-json", endpoint: "" },
] satisfies Array<{ name: string; kind: ServiceKind; endpoint: string }>;
export function serviceChanged() {
  window.dispatchEvent(new Event("service-connections-changed"));
  window.dispatchEvent(new Event("media-models-changed"));
  modelsChanged();
}
export function useServiceConnections() {
  const [connections, setConnections] = useState<ServiceConnection[]>([]);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);
  useEffect(() => {
    let live = true;
    let revision = 0;
    const refresh = () => {
      const current = ++revision;
      if (!native) {
        setLoading(false);
        return;
      }
      setLoading(true);
      setError("");
      void bridge<ServiceConnection[]>("service_connections")
        .then((rows) => {
          if (live && current === revision) {
            setConnections(rows);
            setError("");
          }
        })
        .catch((e) => {
          if (live && current === revision) setError(String(e));
        })
        .finally(() => {
          if (live && current === revision) setLoading(false);
        });
    };
    refresh();
    window.addEventListener("service-connections-changed", refresh);
    return () => {
      live = false;
      window.removeEventListener("service-connections-changed", refresh);
    };
  }, []);
  return {
    connections,
    error,
    loading,
    refresh: () =>
      window.dispatchEvent(new Event("service-connections-changed")),
  };
}
