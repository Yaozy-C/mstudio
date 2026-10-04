import {
  array,
  boolean,
  choices,
  described,
  id,
  integer,
  number,
  object,
  text,
  type Schema,
} from "./schema";
import { design, transitionKind, visual } from "./visualContract";
import { generationOperations } from "./generationContract";
const reference = object(
  { assetId: id, purpose: text(400), start: number(0), end: number(0) },
  ["assetId", "purpose"],
);
const references = array(reference, 12);
const screenplay = object({
  scriptMode: choices("merge", "replace"),
  removeParagraphIds: array(id, 200),
  paragraphOrder: array(id, 200),
  script: array(
    object(
      {
        id,
        title: text(),
        action: text(),
        onScreenText: text(),
        dialogue: text(),
        sound: text(),
        duration: number(0.01, 3600),
      },
      ["id"],
    ),
    200,
  ),
});
const shot = object({
  screenplayId: described(
    id,
    "Existing screenplay node ID that owns this shot.",
  ),
  scriptId: described(
    id,
    "Existing paragraph ID inside shot.screenplayId; not a screenplay or shot ID.",
  ),
  order: described(
    integer(1, 999),
    "Shot sequence number within its screenplay, starting at 1.",
  ),
  duration: described(
    number(0.01, 3600),
    "Editorial shot length in seconds. This is distinct from request_generation.parameters.duration, which is the generated source clip length and must satisfy the selected model.",
  ),
  dialogue: described(text(), "Spoken dialogue for this shot."),
  frames: array(
    object({ assetId: id, title: text(), prompt: text(12000) }, [
      "assetId",
      "title",
    ]),
    50,
  ),
  framePrompt: described(
    text(12000),
    'Image prompt draft saved on the shot. Write inside shot: {"shot":{"framePrompt":"Complete still-image prompt"}}. Saving does not generate an image.',
  ),
  prompt: described(
    text(12000),
    'Video prompt draft saved on the shot. Write inside shot: {"shot":{"prompt":"Complete video prompt"}}. Saving does not generate a video. For request_generation or update_generation, the task prompt field is text.',
  ),
});
const node = {
  title: text(300),
  text: described(
    text(),
    "Node body; for a shot, action/staging notes. Video/image prompt drafts belong in shot.prompt/shot.framePrompt, not here.",
  ),
  assetId: id,
  resultAssetId: id,
  references,
  screenplay: described(
    screenplay,
    "Screenplay changes, including script paragraphs; not shot fields.",
  ),
  shot: described(
    shot,
    "Nested shot fields; use only the properties listed here for this role. Prompt drafts belong inside this object, not at the operation top level. Omitted fields are preserved.",
  ),
};
const clip = {
  trimIn: number(0),
  trimOut: number(0),
  speed: number(0.25, 4),
  volume: number(0),
  start: number(0),
  trackId: id,
  x: number(),
  y: number(),
  scale: number(),
  opacity: number(0, 1),
  fadeIn: number(0),
  fadeOut: number(0),
  visual,
};
const op = (
  name: string,
  properties: Record<string, Schema>,
  required: string[] = [],
) =>
  object(
    {
      op: {
        type: "string",
        const: name,
        description:
          name === "update_node"
            ? "Update an existing node. Fields are siblings of op/id except shot-specific fields, which must be nested in shot. Saves content only."
            : name === "update_generation"
              ? "Replace an existing generation task prompt using taskKey and text. Does not submit generation."
              : `Operation discriminator; must be exactly ${name}.`,
      },
      ...properties,
    },
    ["op", ...required],
  );
export const operationContract = {
  oneOf: [
    op(
      "add_node",
      {
        id,
        kind: choices("text", "shot", "note", "asset", "screenplay"),
        ...node,
        x: number(),
        y: number(),
      },
      ["id", "kind", "title"],
    ),
    op("update_node", { id, ...node }, ["id"]),
    op("remove_node", { id }, ["id"]),
    op("set_brief", { text: text() }, ["text"]),
    op("choose_take", { id, assetId: id }, ["id", "assetId"]),
    op("assemble_screenplay", { id }, ["id"]),
    ...generationOperations,
    op(
      "update_generation",
      {
        taskKey: described(
          text(300),
          "Existing generation task key from a task receipt or inspection; not a shot ID.",
        ),
        text: described(
          { ...text(12000), minLength: 1 },
          "Complete replacement prompt for this generation task. This operation accepts text, not prompt or shot. Saves only; does not regenerate or modify the shot draft.",
        ),
      },
      ["taskKey", "text"],
    ),
    op("regenerate_generation", { taskKey: text(300) }, ["taskKey"]),
    op("set_creation", {
      intent: text(6000),
      essential: text(6000),
      preserve: text(6000),
      stage: choices("planning", "production", "editing"),
    }),
    op(
      "set_references",
      { id, referenceMode: choices("upsert", "replace"), references },
      ["id", "referenceMode", "references"],
    ),
    op(
      "set_references",
      { id, referenceMode: choices("remove"), assetIds: array(id, 12) },
      ["id", "referenceMode", "assetIds"],
    ),
    op("append_clip", { assetId: id, ...clip }, ["assetId"]),
    op("update_clip", { id, ...clip }, ["id"]),
    op(
      "move_clip",
      { id, start: number(0), trackId: id, allowOverlap: boolean },
      ["id", "start"],
    ),
    op(
      "retime_clip",
      { id, speed: number(0.25, 4), ripple: boolean, allowOverlap: boolean },
      ["id", "speed"],
    ),
    op("slip_clip", { id, sourceOffset: number() }, ["id", "sourceOffset"]),
    op("remove_clip", { id }, ["id"]),
    op(
      "set_transition",
      {
        id,
        fromClipId: id,
        kind: transitionKind,
        duration: number(0.05, 3),
        design,
      },
      ["id", "fromClipId", "kind"],
    ),
    op(
      "add_track",
      { id, title: text(100), trackKind: choices("video", "audio") },
      ["title", "trackKind"],
    ),
    op(
      "update_track",
      { id, title: text(100), muted: boolean, hidden: boolean },
      ["id"],
    ),
    op(
      "add_caption",
      { id, text: text(1000), start: number(0), end: number(0) },
      ["text", "start", "end"],
    ),
    op(
      "update_caption",
      { id, text: text(1000), start: number(0), end: number(0) },
      ["id"],
    ),
    op("remove_caption", { id }, ["id"]),
  ],
};
