import { array, choices, id, number, object, text } from "./schema";

import {
  parameterSchema,
  imageRatios,
  videoRatios,
  imageResolutions,
  videoResolutions,
  videoMaxResolutions,
} from "../production/parameters";
const imageParameters = parameterSchema({
  ratios: imageRatios,
  resolutions: imageResolutions,
  customSize: true,
  duration: false,
  frameRatio: false,
  supported: true,
});
const videoParameters = parameterSchema({
  ratios: videoRatios,
  resolutions: [...new Set([...videoResolutions, ...videoMaxResolutions])],
  customSize: false,
  duration: true,
  frameRatio: false,
  supported: true,
});
const references = (roles: string[]) =>
  array(
    {
      ...object(
        {
          assetId: { ...id, description: "Existing project media asset ID." },
          purpose: {
            ...text(400),
            description:
              "What to preserve or use from this asset, e.g. product geometry, top view or character identity. Put descriptive text here, never in role.",
          },
          role: {
            ...choices(...roles),
            description: roles.includes("edit")
              ? "Image input mode: reference = visual guidance; edit = source image to modify. Omission defaults to reference. Describe identity, geometry, viewpoint or composition in purpose."
              : "Video input mode: reference = image guidance; first-frame = starting image; last-frame = ending image; video-reference = reference video. Omission defaults to reference for images and video-reference for videos. Select only modes supported by the chosen model; describe visual use in purpose.",
          },
          start: {
            ...number(0),
            description:
              "Reference video trim start in seconds; only meaningful for video-reference. Not output clip duration.",
          },
          end: {
            ...number(0),
            description:
              "Reference video trim end in seconds, greater than start and within the source video duration.",
          },
        },
        ["assetId", "purpose"],
      ),
      description:
        'One existing media input. Example: {"assetId":"existing-image-id","role":"reference","purpose":"Preserve product geometry visible in the top view"}. Replace the example ID with an actual project asset ID.',
    },
    12,
  );
const common = {
  op: {
    type: "string",
    const: "request_generation",
    description:
      "Create a new generation task. Supply the complete prompt in text, output kind in mediaKind, and only that kind’s parameter fields. Omitted settings inherit the referenced task or selected model settings.",
  },
  mediaModelId: {
    ...id,
    description:
      "Media model ID from mstudio_models. Omit to use the user's selected model. If this turn already has a selected model, an explicit ID must match it; this field cannot override that choice.",
  },
  text: {
    ...text(12000),
    minLength: 1,
    description:
      "Complete final prompt sent to the generation model. For request_generation this field is text, not prompt or shot.prompt. The host does not append the shot draft or script automatically.",
  },
};
const target = {
  id: {
    ...id,
    description:
      "Existing target shot ID. Omission inherits the referenced task’s shot when present; otherwise creates a standalone task. For a reusable standalone image use generationPurpose=asset, which forbids id.",
  },
  canvasTaskKey: {
    ...text(300),
    description:
      "Exact canvas task key referenced by this turn; omit when targeting only a shot. Not a shot ID or a new arbitrary task name.",
  },
};
export const generationOperations = [
  object(
    {
      ...common,
      ...target,
      mediaKind: {
        type: "string",
        const: "image",
        description:
          "Generate a still image. This branch has no mode or duration.",
      },
      parameters: imageParameters,
      references: references(["edit", "reference"]),
    },
    ["op", "text", "mediaKind"],
  ),
  object(
    {
      ...common,
      ...target,
      mediaKind: {
        type: "string",
        const: "video",
        description:
          "Generate a video clip; the reference arrangement is specified separately by mode.",
      },
      parameters: videoParameters,
      mode: {
        ...choices("single", "ends", "multi", "mixed"),
        description:
          "Video input arrangement: single = first-frame image; ends = first-frame and last-frame images; multi = reference images; mixed = image/video references. Match references[].role: for ordinary reference images use mode=multi with role=reference; reference is a role, never a mode. For bookend images use mode=ends with first-frame and last-frame roles. Omission inherits the referenced task arrangement; without a source task it defaults to single. When supplying reference images explicitly, also set mode=multi. This is not media kind or output count.",
      },
      references: references([
        "reference",
        "first-frame",
        "last-frame",
        "video-reference",
      ]),
    },
    ["op", "text", "mediaKind"],
  ),
  object(
    {
      ...common,
      mediaKind: {
        type: "string",
        const: "image",
        description:
          "Generate a still image. This branch has no mode or duration.",
      },
      parameters: imageParameters,
      generationPurpose: {
        type: "string",
        const: "asset",
        description:
          "Standalone reusable reference asset, without a shot or canvas task target.",
      },
      references: references(["edit", "reference"]),
    },
    ["op", "text", "mediaKind", "generationPurpose", "references"],
  ),
];
