/** Host saves the direct reference atomically; no media import or local asset insertion. */
export async function attachPrevisVideo({
  flush,
  attach,
  refresh,
}: {
  flush: () => Promise<unknown>;
  attach: () => Promise<unknown>;
  refresh: () => Promise<void>;
}) {
  await flush();
  await attach();
  await refresh();
}
