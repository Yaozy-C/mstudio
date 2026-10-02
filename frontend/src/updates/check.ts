import { bridge, native } from "../bridge";
import { version } from "../../package.json";
export const RELEASE_PAGE = "https://github.com/Yaozy-C/mstudio/releases";
export type UpdateResult =
  | { status: "unpublished" }
  | { status: "available" | "current"; version: string };
function parseVersion(value: string) {
  const match =
    /^v?(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.exec(
      value,
    );
  if (!match) throw new Error("INVALID_RELEASE_VERSION");
  const parts = match.slice(1, 4).map(Number);
  if (!parts.every(Number.isSafeInteger))
    throw new Error("INVALID_RELEASE_VERSION");
  return { parts, prerelease: !!match[4] };
}
export function compareRelease(
  current: string,
  release: unknown,
): UpdateResult {
  if (release === null) return { status: "unpublished" };
  if (
    !release ||
    typeof release !== "object" ||
    !("tag_name" in release) ||
    typeof release.tag_name !== "string" ||
    !("draft" in release) ||
    release.draft !== false ||
    !("prerelease" in release) ||
    release.prerelease !== false
  )
    throw new Error("INVALID_RELEASE");
  const latest = parseVersion(release.tag_name);
  const installed = parseVersion(current);
  if (latest.prerelease) throw new Error("INVALID_RELEASE");
  const different = latest.parts.findIndex(
    (part, index) => part !== installed.parts[index],
  );
  const newer =
    different === -1
      ? installed.prerelease
      : latest.parts[different] > installed.parts[different];
  return {
    status: newer ? "available" : "current",
    version: release.tag_name.replace(/^v/, ""),
  };
}
export async function checkForUpdate(): Promise<UpdateResult> {
  if (native) {
    const result = await bridge<{ currentVersion: string; release: unknown }>(
      "check_app_update",
    );
    return compareRelease(result.currentVersion, result.release);
  }
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 15000);
  try {
    const response = await fetch(
      "https://api.github.com/repos/Yaozy-C/mstudio/releases/latest",
      {
        headers: { Accept: "application/vnd.github+json" },
        signal: controller.signal,
        cache: "no-store",
      },
    );
    if (response.status === 404) return { status: "unpublished" };
    if (!response.ok) throw new Error("UPDATE_CHECK_FAILED");
    return compareRelease(version, await response.json());
  } finally {
    clearTimeout(timeout);
  }
}
