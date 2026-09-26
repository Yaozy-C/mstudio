export type ModelConnection = {
  id: string;
  name: string;
  connectionId?: string | null;
  endpoint: string;
  model: string;
  contextWindow?: number | null;
  adapter:
    | "openai-compatible"
    | "openai-responses"
    | "gemini-native"
    | "anthropic-native";
  inputs: {
    image: boolean;
    audio: boolean;
    video: boolean;
    document: boolean;
  };
  hasKey: boolean;
};
export type ModelCatalog = {
  profiles: ModelConnection[];
  defaultId: string | null;
  selectedId: string | null;
};
export function localEndpoint(endpoint: string) {
  try {
    return ["localhost", "127.0.0.1", "[::1]"].includes(
      new URL(endpoint).hostname,
    );
  } catch {
    return false;
  }
}
export const readyModel = (model: ModelConnection) =>
  model.hasKey || localEndpoint(model.endpoint);
export const selectedModel = (catalog: ModelCatalog) =>
  catalog.profiles.find(
    (p) => p.id === (catalog.selectedId || catalog.defaultId),
  );
export const emptyCatalog: ModelCatalog = {
  profiles: [],
  defaultId: null,
  selectedId: null,
};
export function newConnection(): ModelConnection {
  return {
    id: crypto.randomUUID(),
    name: "",
    endpoint: "",
    model: "",
    contextWindow: 32768,
    adapter: "openai-compatible",
    inputs: { image: false, audio: false, video: false, document: false },
    hasKey: false,
  };
}
