import type { WorkContext } from "./workContext";

export function workflowAgent(view?: WorkContext["view"]) {
  return view === "concepts" || view === "materials"
    ? "concept"
    : view === "script"
      ? "writer"
      : view === "assets"
        ? "assets"
        : view === "storyboard"
          ? "director"
          : view === "film"
            ? "editor"
            : "coordinator";
}

export function workflowRoles(
  view?: WorkContext["view"],
): string[] | undefined {
  return view === "drafts"
    ? ["coordinator", "assets", "image", "production"]
    : view === "concepts"
      ? ["concept", "coordinator"]
      : view === "materials"
        ? ["concept", "writer"]
        : view === "script"
          ? ["writer", "director"]
          : view === "assets"
            ? ["assets"]
            : view === "storyboard"
              ? ["director", "assets", "image", "production"]
              : view === "film"
                ? ["editor"]
                : undefined;
}
