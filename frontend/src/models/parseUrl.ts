/** Frontend URL parsing uses the browser's standard URL API. */
type ParsedUrl = Pick<
  URL,
  "protocol" | "username" | "password" | "hostname" | "hash"
>;
export function parseUrl(value: string): ParsedUrl {
  return new URL(value);
}
