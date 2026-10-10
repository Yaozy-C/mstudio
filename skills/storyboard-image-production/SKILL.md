---
name: storyboard-image-production
description: "Produce temporal storyboard boards in Mstudio: one four-panel action board per requested shot, with readable state changes, continuous scene geometry and product identity; also execute explicitly requested stills and reference assets."
---

# Storyboard image production

For a shot storyboard request, produce one temporal 2 by 2 action board per requested Mstudio shot record. This is the selected Mstudio workflow, not an industry claim that four panels always perform best. Do not substitute a single representative image or produce a second independent-frame deliverable by default. Explicit requests for a single still, reference asset or exact control frame retain their requested format. Produce the reusable references the selected work depends on. Preserve the chosen concept, the selected identity and original product geometry. Loading this Skill grants no extra tools or permissions: saving a prompt is not a generated image, and task submission is not a completed image.

## Shot-level generation versus exact task retry

"Generate/regenerate the storyboard for shot N" requests the current one-board deliverable for that shot. Old separate frame tasks, legacy F1-F6 lists and past coordinator proposals are historical inputs, not a requirement to create or retry that many images. Replan their necessary states into the current board before submitting; do not replay all old prompts unchanged. Preserve original assets and unrelated tasks.

Only an explicit retry of a particular task or an explicit request to keep its exact old prompt/format is an unchanged task retry. If delegated instructions demand six unchanged single-frame retries while the current user requested a shot storyboard, report the deliverable conflict to the coordinator instead of silently treating its six-task plan as the user's specification. Do not start both the old set and a new board.

## Read for the task at hand

| The task has to decide | Read |
|---|---|
| How to write a board prompt with one visible moment per panel | [Image prompt writing](references/prompt-writing.md) |
| How to select four temporal states, make the shot readable and preserve continuity across the board | [Storyboard frames](references/frames.md) |
| Whether subject, location or layout references are missing, and how to design useful reference assets | [Reference assets](references/reference-assets.md) |
| Viewpoint, contact, part count and cross-frame state constraints | [Frame rules](references/frame-checks.md) |
| Staging, attention centre or spatial relationships that need redesigning | [Shot grammar](references/scene-execution.md) |
| Photographic people, skin, light and material appearance | [Photographic appearance](references/scene-execution.md) |

For storyboard images or start/key/end control-frame requests, use the storyboard-frame rules when their format is needed; reuse loaded text. It realizes the supplied shot design as one four-panel board per shot; panel count does not create cuts or multiply duration. Covers and unrelated standalone pictures do not become storyboards. Use [core](CORE.md) when its short rules are needed; reuse loaded text.

## Bind every input to a concrete purpose

Each supplied image is one of: edit target, identity or wardrobe reference, product geometry and material evidence, composition or state reference, or motion reference. Name that use plus the subject it supplies in `references[].purpose` — for example "the lead's face and hair identity", "the opened main compartment as state reference", "customer's own product front panel for geometry" — never a bare label such as "content reference". `role` selects the technical input mode and `purpose` describes the visual use; they are separate fields and both must be correct.

Purposes carry identity assignments. When a scene has more than one recurring person or object, bind each input to the named subject, and keep that binding consistent through the prompt text and any later segment. An image simply labelled "reference" cannot hold a character's identity.

## What a reference does and does not transfer

An identity asset fixes who the subject is. It does not fix pose, camera, background or lighting for a scene frame: do not inherit its standing pose or studio light into a scene, and do not restate them as if they were requested.

A reference of an opened bag, a mid-action state or a detail close-up supplies only its declared properties. Its state does not automatically bind a new composition. It is not an actual video first-frame control; when used as a control/edit base, its existing state must be compatible with the requested change.

## Boundaries

Script content, shot design and video generation belong to other Skills. Fix execution deviations locally; return design conflicts with evidence. Static frames never establish motion, speed, sound or playback.
