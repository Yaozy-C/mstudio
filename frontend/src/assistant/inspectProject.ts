import { scriptChanged } from "../creative/script";
import type { Project } from "../model";
import { promptStale } from "../creative/prompt";
import { tracksOf } from "../timeline/document";
import { framesOf } from "../production/frames";
import { runsOf } from "../production/requestTask";
const offsetOf = (v: unknown) =>
  typeof v === "number" && Number.isFinite(v) ? Math.max(0, Math.floor(v)) : 0;
export function inspectProject(p: Project, args: Record<string, unknown>) {
  const offset = offsetOf(args.offset),
    textOffset = offsetOf(args.textOffset);
  const ids = Array.isArray(args.nodeIds) ? args.nodeIds.slice(0, 12) : [];
  const explicit = Array.isArray(args.fields);
  const fields = explicit
    ? (args.fields as string[])
    : [
        "plan",
        "text",
        "title",
        "shot",
        "shots",
        "dialogue",
        "assetId",
        "resultAssetId",
      ];
  const wants = (field: string) => fields.includes(field);
  const revision = p.revision ?? 0;
  const detailLimit = ids.length === 1 || args.taskKey ? 4000 : 1000;
  const snippet = (value: string, max = 1000) => ({
    text: value.slice(textOffset, textOffset + max),
    nextTextOffset: value.length > textOffset + max ? textOffset + max : null,
  });
  const assets = p.assets.map(
    ({ id, name, kind, duration, width, height }) => ({
      id,
      name: name.slice(0, 100),
      kind,
      duration,
      width,
      height,
    }),
  );
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
  if (args.section === "generation")
    page = runsOf(p)
      .filter((t) => !args.taskKey || args.taskKey === t.key)
      .map((t) => ({
        id: t.key,
        turnId: t.turnId,
        kind: t.kind,
        ownerId: t.ownerId,
        status: t.status,
        resultAssetId: t.resultAssetId,
        resultAssetIds: t.resultAssetIds,
        modelId: t.modelId,
        prompt: snippet(t.prompt, detailLimit),
        nextPrompt:
          t.nextPrompt === undefined
            ? undefined
            : snippet(t.nextPrompt, detailLimit),
        sourceTaskKey: t.sourceTaskKey,
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
  if (page && Array.isArray(args.fields) && args.section !== "generation") {
    page = page.map((item) => {
      const source = item as Record<string, unknown>;
      const keep = new Set(["id", ...(args.fields as string[])]);
      return {
        ...Object.fromEntries(
          Object.entries(source).filter(([key]) => keep.has(key)),
        ),
        omittedFields: Object.keys(source).filter((key) => !keep.has(key)),
      };
    });
  }
  if (page)
    return {
      revision,
      section: args.section,
      missingIds,
      total: page.length,
      items: page.slice(offset, offset + 5),
      nextOffset: offset + 5 < page.length ? offset + 5 : null,
    };
  return {
    revision,
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
      planId: n.shot?.planId,
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
        omittedFields: explicit
          ? ["title", "text", "shot", "plan", "references"].filter(
              (f) => !wants(f),
            )
          : undefined,
        plan:
          wants("plan") && n.plan
            ? {
                script: n.plan.script?.slice(offset, offset + 10).map((s) => ({
                  id: s.id,
                  duration: s.duration ?? 5,
                  title: s.title,
                  action: snippet(s.action),
                  onScreenText: snippet(s.onScreenText ?? ""),
                  dialogue: snippet(s.dialogue),
                  sound: snippet(s.sound),
                })),
                nextScriptOffset:
                  (n.plan.script?.length ?? 0) > offset + 10
                    ? offset + 10
                    : null,
                story: snippet(n.plan.story),
                sound: snippet(n.plan.sound),
              }
            : undefined,
        shot: n.shot && {
          ...(wants("shot")
            ? {
                planId: n.shot.planId,
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
          wants("shots") && n.kind === "plan"
            ? p.nodes
                .filter((s) => s.shot?.planId === n.id)
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
          n.kind === "plan" &&
          p.nodes.filter((s) => s.shot?.planId === n.id).length > offset + 10
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
