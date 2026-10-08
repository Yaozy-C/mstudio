---
name: image-production
description: Produce still images in Mstudio - write one-moment image prompts, execute designed storyboard frames, prepare reusable white-background reference assets, and inspect the actual pixels.
---

# Image production

Produce the requested still and the reusable references it depends on, preserving the chosen concept, the selected identities and the verified product geometry. Read [CORE.md](CORE.md) together with this file; every step below assumes its rules. When this text is injected without tools, as in the generation panel, only steps 2 to 4 apply: deliver the final prompt and the reference bindings.

## Workflow

1. **Recover the design and the evidence.** Read the target shot's text, the selected moment, the original product images and the existing assets. A frame executes a chosen instant; it does not redesign the shot. Return a design conflict to direction with evidence; fix execution deviations here.
2. **Decide reuse, edit or new.** Reuse a suitable inspected image before generating. For a same-view state change, edit an inspected base with `role=edit` and "change only X" wording. A new scene is a new generation. For an identity that recurs across distinct shots, prepare a shared asset per [Reference assets](references/reference-assets.md).
3. **Bind every input to a concrete purpose.** Each supplied image is an edit target, an identity or wardrobe reference, product geometry and material evidence, a composition or state reference, or a motion reference. Write that use and the subject it supplies in `references[].purpose`, for example "the lead's face and hair identity" or "the opened main compartment as state reference"; a bare label such as "content reference" is rejected. `role` is the technical input mode (`edit` or `reference`); `purpose` is the visual use; both must be right and agree with the prompt text. With more than one recurring person or object, bind each input to its named subject and keep the binding stable in later segments.
4. **Write the prompt for one visible moment** with [Image prompt writing](references/prompt-writing.md), which includes a worked example, using the selected model's injected rules for syntax and parameters.
5. **Save or generate within the request.** A prompt-only request ends with `mstudio_set_image_prompt(id, framePrompt)`. An explicit image request submits `mstudio_generate_image` for a shot frame or `mstudio_generate_reference_image` for a standalone asset. Submission creates a task; it is not a result.
6. **Inspect the actual pixels.** Read the result with `mstudio_read_image` and check it per [Frame inspection](references/frame-checks.md) before describing, linking or reusing it. Link accepted frames with `mstudio_set_shot_frames(id, frames)` and attach shared assets to consuming shots with `mstudio_upsert_references`.
7. **Report** the affected shot IDs, the real asset IDs, the selected moments, the deviations still open, and which judgements rest on stills only.

## Read when

| The task has to decide | Read |
|---|---|
| How to write one prompt for one visible moment, with reference bindings | [Image prompt writing](references/prompt-writing.md) |
| How to execute a designed shot as a readable frame, and how dependent frames reuse an anchor | [Storyboard frames](references/frames.md) |
| Whether a shared identity reference is missing, and how to design a white-background asset | [Reference assets](references/reference-assets.md) |
| How to inspect pixels for viewpoint, contact, part count and cross-frame state | [Frame inspection](references/frame-checks.md) |
| Staging, attention centre or spatial relationships that need redesign | [Shot grammar](../creative-ad-director/references/cinematography.md) |
| Photographic people, skin, light and material | [Photographic appearance](../creative-ad-director/references/photographic-appearance.md) |
| Realism judgement and visible product components | [Controlled generation](../product-video-production/references/generation.md) |

## What a reference transfers

A white-background identity asset fixes who the subject is. It does not fix pose, camera, background or lighting for a scene frame: never inherit its standing pose or studio light into a scene, and never restate them as if requested. A reference of an opened bag, a mid-action state or a detail close-up supplies structure and state; it is not a first frame and does not license a prompt to repeat an event already complete inside it.

## Failure signatures

| You notice | Do instead |
|---|---|
| A `purpose` reads "content reference", "reference image" or "style" | Name the subject and the visual use: whose identity, which geometry, which state |
| The identity asset's standing pose or studio light appears in the scene frame | Take identity only; write the scene's own pose, background and light |
| The prompt stacks "8K", "ultra realistic", "masterpiece" or a long negative list | Describe the visible surfaces, the light and how the material behaves |
| A generated image is being used as proof of a product part | Only original photos are product evidence; a generated base supplies composition and state |
| A mid-action or end-state image is about to be passed as a first frame | Choose an actual start image, or declare it a state reference and say so in the prompt |
| The report says "generated" as if it meant usable | Read the pixels; report submitted, generated and inspected as separate states |

## Report

Shot IDs and the asset IDs actually linked; for each image, the moment it shows and the checks that passed or failed against the originals; what no still can show (motion, speed, sound).

## Boundaries

Script content, shot design and video generation belong to other Skills. Static frames never establish motion, speed, sound or playback.
