/** URL parsing is a host service in QuickJS, and a Web API in the UI. */
type ParsedUrl = Pick<
  URL,
  "protocol" | "username" | "password" | "hostname" | "hash"
>;
declare global {
  var domainParseUrl: ((value: string) => string) | undefined;
}
export function parseUrl(value: string): ParsedUrl {
  if (typeof globalThis.domainParseUrl === "function") {
    const parsed = JSON.parse(
      globalThis.domainParseUrl(value),
    ) as ParsedUrl | null;
    if (!parsed) throw new Error("Invalid URL");
    return parsed;
  }
  return new URL(value);
}
