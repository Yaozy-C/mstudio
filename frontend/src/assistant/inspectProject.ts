import { batchSummary } from "../production/batch";
import { inspectScript } from "./inspectScript";
import { scriptChanged } from "../creative/script";
import type { Project } from "../model";
import { isSelectableAsset } from "../workspace/assetLibrary";
import { promptStale } from "../creative/prompt";
import { tracksOf } from "../timeline/document";
import { framesOf } from "../production/frames";
import { runsOf } from "../production/requestTask";
import { taskOutcome } from "../production/taskOutcome";
const offsetOf = (v: unknown) =>
  typeof v === "number" && Number.isFinite(v) ? Math.max(0, Math.floor(v)) : 0;
export function inspectProject(p: Project, args: Record<string, unknown>) {
  const offset = offsetOf(args.offset),
    textOffset = offsetOf(args.textOffset);
  const ids = Array.isArray(args.nodeIds) ? args.nodeIds.slice(0, 12) : [];
  const explicit = Array.isArray(args.fields);
  const fields = explicit
    ? (args.fields as string[])
    : ["title", "shot", "script", "assetId", "resultAssetId"];
  const wants = (field: string) => fields.includes(field);
  const revision = p.revision ?? 0;
  const detailLimit = ids.length === 1 || args.taskKey ? 4000 : 1000;
  const snippet = (value: string, max = 1000) => ({
    text: value.slice(textOffset, textOffset + max),
    nextTextOffset: value.length > textOffset + max ? textOffset + max : null,
  });
  const assets = p.assets
    .filter(isSelectableAsset)
    .map(({ id, name, kind, duration, width, height }) => ({
      id,
      name: name.slice(0, 100),
      kind,
      duration,
      width,
      height,
    }));
  const creation = p.creation;
  const captions = (p.captions ?? []).map((c) => ({
    ...c,
    ...snippet(c.text),
  }));
  let page: unknown[] | undefined;
  if (args.section === "assets") page = assets;
  if (args.section === "clips") page = p.clips;
  if (args.section === "tracks") page = tracksOf(p);
  if (args.section === "captions") page = captions;
  const generation =
    args.section === "generation"
      ? runsOf(p).filter((t) => !args.turnId || t.turnId === args.turnId)
      : [];
  if (args.section === "generation")
    page = generation
      .filter(
        (t) =>
          (!args.taskKey || args.taskKey === t.key) &&
          (!args.status || t.status === args.status),
      )
      .map((t) => ({
        continuation: taskOutcome(t).continuation,
        id: t.key,
        turnId: t.turnId,
        kind: t.kind,
        generationPurpose: t.generationPurpose,
        ownerId: t.ownerId,
        targetNodeId: t.targetNodeId,
        trackingPaused: t.trackingPaused,
        status: t.status,
        resultAssetId: t.resultAssetId,
        resultAssetIds: t.resultAssetIds,
        modelId: t.modelId,
        prompt: snippet(t.prompt, detailLimit),
        hiddenFromList: !!t.hiddenFromList,
        inputs: t.inputs,
        parameters: t.parameters,
        error: t.error,
      }));
  if (args.section === "creation" && !ids.length)
    return {
      revision,
      creation: {
        brief: snippet(p.brief, 800),
        intent: snippet(creation?.intent ?? p.brief, 800),
        essential: snippet(creation?.essential ?? "", 800),
        preserve: snippet(creation?.preserve ?? "", 800),
        stage: creation?.stage,
      },
      section: "creation",
    };
  const objectIds = Array.isArray(args.ids) ? args.ids.slice(0, 12) : [];
  const missingIds = objectIds.filter(
    (id) => !page?.some((item) => (item as { id?: string }).id === id),
  );
  if (page && objectIds.length)
    page = page.filter((item) =>
      objectIds.includes((item as { id?: string }).id),
    );
  if (page && Array.isArray(args.fields)) {
    page = page.map((item) => {
      const source = item as Record<string, unknown>;
      const keep = new Set([
        "id",
        ...(args.fields as string[]),
        ...(args.section === "generation" ? ["status", "continuation"] : []),
      ]);
      return {
        ...Object.fromEntries(
          Object.entries(source).filter(([key]) => keep.has(key)),
        ),
        omittedFields:
          args.section === "generation"
            ? undefined
            : Object.keys(source).filter((key) => !keep.has(key)),
      };
    });
  }
  if (page) {
    // Small editing groups fit one read; large records still have a byte budget.
    const limit = args.section === "generation" ? (explicit ? 30 : 5) : 12;
    const items: unknown[] = [];
    let size = 0;
    for (const item of page.slice(offset, offset + limit)) {
      const bytes = new TextEncoder().encode(JSON.stringify(item)).length;
      if (items.length && size + bytes > 12000) break;
      items.push(item);
      size += bytes;
    }
    return {
      revision,
      section: args.section,
      batch:
        args.section === "generation" ? batchSummary(generation) : undefined,
      missingIds,
      total: page.length,
      items,
      nextOffset:
        offset + items.length < page.length ? offset + items.length : null,
    };
  }
  return {
    revision,
    readDetails: ids.length
      ? "Use fields for full text; script with paragraphIds/scriptFields narrows paragraphs; screenplay returns the full paginated document."
      : undefined,
    missingNodeIds: ids.filter((id) => !p.nodes.some((n) => n.id === id)),
    name: p.name.slice(0, 200),
    brief: ids.length ? undefined : p.brief.slice(0, 500),
    format: { width: p.width, height: p.height, fps: p.fps },
    nodeCount: p.nodes.length,
    nextOffset:
      !ids.length && offset + 10 < p.nodes.length ? offset + 10 : null,
    nodes: (ids.length ? [] : p.nodes.slice(offset, offset + 10)).map((n) => ({
      id: n.id,
      kind: n.kind,
      title: n.title.slice(0, 80),
      assetId: n.assetId,
      resultAssetId: n.resultAssetId,
      screenplayId: n.shot?.screenplayId,
      order: n.shot?.order,
    })),
    details: p.nodes
      .filter((n) => ids.includes(n.id))
      .map((n) => ({
        id: n.id,
        kind: n.kind,
        title: wants("title") ? n.title.slice(0, 80) : undefined,
        ...(wants("text") ? snippet(n.text, detailLimit) : {}),
        assetId: wants("assetId") ? n.assetId : undefined,
        resultAssetId: wants("resultAssetId") ? n.resultAssetId : undefined,
        omittedFields: [
          "title",
          "text",
          "shot",
          "screenplay",
          "references",
        ].filter((f) => !wants(f)),
        screenplay:
          n.screenplay && (wants("screenplay") || wants("script"))
            ? {
                ...inspectScript(
                  n.screenplay.script ?? [],
                  args,
                  wants("screenplay"),
                ),
              }
            : undefined,
        shot: n.shot && {
          ...(wants("shot")
            ? {
                screenplayId: n.shot.screenplayId,
                scriptId: n.shot.scriptId,
                scriptChanged: scriptChanged(p, n),
              }
            : {}),
          order:
            wants("shot") || wants("shot.order") ? n.shot.order : undefined,
          duration:
            wants("shot") || wants("shot.duration") || wants("duration")
              ? n.shot.duration
              : undefined,
          frames: wants("frames")
            ? framesOf(n)
                .slice(offset, offset + 5)
                .map((f) => ({
                  assetId: f.assetId,
                  title: f.title,
                  prompt: snippet(f.prompt ?? ""),
                }))
            : undefined,
          framePrompt: wants("framePrompt")
            ? snippet(n.shot.framePrompt ?? "", detailLimit)
            : undefined,
          visualChanged: wants("shot") ? n.shot.visualChanged : undefined,
          dialogue: wants("dialogue") ? snippet(n.shot.dialogue) : undefined,
          prompt: wants("prompt")
            ? snippet(n.shot.prompt ?? "", detailLimit)
            : undefined,
          promptStale:
            wants("shot") || wants("prompt") ? promptStale(p, n) : undefined,
          takes: wants("takes")
            ? n.shot.takes
                ?.slice(offset, offset + 5)
                .map(({ basis: _basis, ...take }) => take)
            : undefined,
        },
        shots:
          wants("shots") && n.kind === "screenplay"
            ? p.nodes
                .filter((s) => s.shot?.screenplayId === n.id)
                .sort((a, b) => a.shot!.order - b.shot!.order)
                .slice(offset, offset + 10)
                .map((s) => ({
                  id: s.id,
                  title: s.title,
                  order: s.shot!.order,
                  duration: s.shot!.duration,
                }))
            : undefined,
        nextShotOffset:
          wants("shots") &&
          n.kind === "screenplay" &&
          p.nodes.filter((s) => s.shot?.screenplayId === n.id).length >
            offset + 10
            ? offset + 10
            : null,
        references: wants("references")
          ? n.references
              ?.slice(offset, offset + 4)
              .map((r) => ({ ...r, purpose: r.purpose.slice(0, 400) }))
          : undefined,
        nextReferenceOffset:
          wants("references") && (n.references?.length ?? 0) > offset + 4
            ? offset + 4
            : null,
      })),
    assets: ids.length ? [] : assets.slice(offset, offset + 10),
    assetCount: p.assets.length,
    clips: ids.length ? [] : p.clips.slice(offset, offset + 10),
    clipCount: p.clips.length,
    tracks: ids.length ? [] : tracksOf(p).slice(0, 10),
    captionCount: p.captions?.length ?? 0,
    creation:
      (ids.length && args.section !== "creation") || !creation
        ? undefined
        : {
            intent: creation.intent.slice(0, 300),
            essential: creation.essential.slice(0, 300),
            preserve: creation.preserve.slice(0, 300),
            stage: creation.stage,
            truncated:
              creation.intent.length > 300 ||
              creation.essential.length > 300 ||
              creation.preserve.length > 300,
          },
  };
}
