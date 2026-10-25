---
name: storyboard-image-production
description: "Produce temporal storyboard boards in Mstudio: one four-panel action board per requested shot, with readable state changes, continuous scene geometry and product identity; also execute explicitly requested stills and exact control frames."
---

# Storyboard image production

For a shot storyboard request, produce one temporal 2 by 2 action board per requested Mstudio shot record. This is the selected Mstudio workflow, not an industry claim that four panels always perform best. Do not substitute a single representative image or produce a second independent-frame deliverable by default. Explicit single-still and exact-control-frame requests retain their requested format. Use the selected reusable references; return missing shared assets through the coordinator to the assets Agent. Preserve the chosen concept, the selected identity and original product geometry. Loading this Skill grants no extra tools or permissions: saving a prompt is not a generated image, and task submission is not a completed image.

Specialists execute assigned production work and return real outputs and task status. Only the coordinator conducts internal review; the user makes the final selection. Reading references is for execution, not approval. Apply assigned revisions without initiating self-review, acceptance gates or extra generation.

## Shared art direction, storyboard execution

Use the selected, revised character, wardrobe, location and prop assets as the visual baseline. Keep their medium, realism or stylization, era, palette relationships, material treatment and finish through the scene. The board's job is framing, staging, action states and motivated scene light; it must not resemble a neutral asset sheet just because identity references use simple backgrounds. Carry the selected identity and shared aesthetic into the actual generation inputs. Return missing or conflicting asset decisions through coordination instead of independently redesigning each shot's cast or look.

## Look frame before board

When the approved look is missing, or the user says the result is not attractive enough, execute an authorized full-frame look frame before the board. A look frame is one photograph with one visual idea; it establishes capture, camera, light, color and skin/material finish. It is not a board and does not replace the board deliverable.

Use the selected look frame as a visual reference for the board. Do not use a board as the look reference for later shots. If no look frame is authorized, state a compact look hypothesis in the board prompt: capture, camera, light, palette and finish.

The board remains the requested deliverable. Do not silently substitute the look frame for it; return both only when the task authorizes both.

## Shot-level generation versus exact task retry

"Generate/regenerate the storyboard for shot N" requests the current one-board deliverable for that shot. Old separate frame tasks, legacy F1-F6 lists and past coordinator proposals are historical inputs, not a requirement to create or retry that many images. Replan their necessary states into the current board before submitting; do not replay all old prompts unchanged. Preserve original assets and unrelated tasks.

Only an explicit retry of a particular task or an explicit request to keep its exact old prompt/format is an unchanged task retry. If delegated instructions demand six unchanged single-frame retries while the current user requested a shot storyboard, report the deliverable conflict to the coordinator instead of silently treating its six-task plan as the user's specification. Do not start both the old set and a new board.

## Read for the task at hand

| The task has to decide | Read |
|---|---|
| How to write a board prompt with one visible moment per panel | [Image prompt writing](references/prompt-writing.md) |
| How to choose the photographic treatment, camera, light and color across four panels | [Photographic direction](references/photographic-direction.md) |
| Skin, faces and hands at panel scale | [Skin realism](references/skin-realism.md) |
| How to select four useful shot/state moments, make the design readable and preserve continuity across the board | [Storyboard frames](references/frames.md) |
| Which selected shared references the shot needs, and what is missing | [Reference assets](references/reference-assets.md) |
| Viewpoint, contact, part count and cross-frame state constraints | [Frame rules](references/frame-checks.md) |
| Staging, attention centre or spatial relationships that need redesigning | [Shot grammar](references/scene-execution.md) |
| Scene-specific light and material appearance | [Photographic appearance](references/scene-execution.md) |

For storyboard images or start/key/end control-frame requests, use the storyboard-frame rules when their format is needed; reuse loaded text. It realizes the supplied shot design as one four-panel board per shot; panel count does not create cuts or multiply duration. Covers and unrelated standalone pictures do not become storyboards. Use [core](CORE.md) when its short rules are needed; reuse loaded text.

## Visual execution and bounded comparison

Use [visual quality and control frames](references/visual-quality.md) for a new look, a "not attractive enough" complaint, local appearance repair or an effect control frame. Apply the approved attention order, composition, light/value separation and material treatment to the actual prompt. For each reference, specify what to borrow and what not to borrow; identity does not import its background, pose or lighting. A colour filter or more style adjectives does not repair a weak composition.

An action board explains sequence; a look frame selects appearance; a production control frame supplies a supported endpoint state. Do not conflate them or silently change the requested deliverable. A board remains a board when requested; exact control frames must be independent full-frame images, never the whole grid bound as a first or last frame. Extra frames and alternative looks need existing budget authorization.

At a 2 by 2 board, each panel is one quarter of the output. If the delivered board is 1672 by 941, each panel is about 836 by 470: enough for blocking, not for pore-level skin or precise fingers. Do not demand detail the panel cannot hold. When that detail matters, propose an authorized full-frame look or control frame, or a separate panel route, instead of pretending the board can carry it.

When alternatives are authorized, vary a named design decision rather than merely rerolling the same prompt. Return alternatives with their intended design differences for coordinator review and user selection. Apply assigned local edits to the selected image without reinventing the scene. Return real output IDs and execution limits to the coordinator without claiming acceptance.

For an effect frame, depict one phase and its material state, protect specified identity/label regions, and keep start/end composition compatible. Do not put an entire evolving effect into a still. Keep planning notes out of unsupported tool fields; compile relevant decisions into the actual submitted image prompt and concrete reference purposes. Missing assets go back through the existing coordinator handoff, not an invented delegation tool.

## Physical continuity

Own product fidelity in the rendered image: bind the original evidence, preserve the components and materials visible from the designed camera, and correct observed execution drift within the authorized scope. Direction supplies the shooting decisions and factual boundaries. Do not flatten its camera choices into the source photograph's angle, expose every component in every view, or turn a local product error into a new shot design. Return only an actual design/factual conflict to direction.

Keep cross-frame continuity through stable identity and construction, while pose, support, openings and visible projection change according to the action and camera. Physical plausibility governs every depicted state; "identical in every frame" must not freeze moving parts or override gravity, contact, material response or mechanism operation. For state changes and local edits, apply [physical state rules](references/prompt-writing.md#physical-state-and-continuity) and include the relevant physical relationships in the actual generation prompt, not only in planning notes. Preserve explicitly designed departures from ordinary physics within their stated scope.

## Bind every input to a concrete purpose

Each supplied image must have a supported technical role. Name its subject, visual purpose and the properties it supplies in `references[].purpose`. `role` selects the technical input mode; both fields must agree with the prompt and actual input use.

Purposes carry identity assignments. When a scene has more than one recurring person or object, bind each input to the named subject, and keep that binding consistent through the prompt text and any later segment. An image simply labelled "reference" cannot hold a character's identity.

## What a reference does and does not transfer

An identity asset fixes who the subject is. It does not fix pose, camera, background or lighting for a scene frame: do not inherit its standing pose or studio light into a scene, and do not restate them as if they were requested.

A reference depicting a particular object state, action phase or detail supplies only its declared properties. Its state does not automatically bind a new composition. It is not an actual video first-frame control; when used as a control/edit base, its existing state must be compatible with the requested change.

## Boundaries

Script content, shot design and video generation belong to other Skills. Execute assigned local revisions; return design conflicts to the coordinator. Static frames never establish motion, speed, sound or playback.
