---
name: image-production
description: Produce still images in Mstudio: write one-moment image prompts, execute designed frames, prepare reusable subject and location references and preserve the selected visual constraints.
---

# Image production

Produce the requested still image and the reusable references it depends on. Preserve the chosen concept, the selected identity and original product geometry. Loading this Skill grants no extra tools or permissions: saving a prompt is not a generated image, and task submission is not a completed image.

## Read for the task at hand

| The task has to decide | Read |
|---|---|
| How to write one prompt for one visible moment | [Image prompt writing](references/prompt-writing.md) |
| How to choose representative/start/key/end frames, make the shot readable and preserve continuity across a storyboard | [Storyboard frames](references/frames.md) |
| Whether subject, location or layout references are missing, and how to design useful reference assets | [Reference assets](references/reference-assets.md) |
| Viewpoint, contact, part count and cross-frame state constraints | [Frame rules](references/frame-checks.md) |
| Staging, attention centre or spatial relationships that need redesigning | [Shot grammar](references/scene-execution.md) |
| Photographic people, skin, light and material appearance | [Photographic appearance](references/scene-execution.md) |

For storyboard images or start/key/end control-frame requests, use the storyboard-frame rules when their format is needed; reuse loaded text. It applies the supplied shot design without requiring a fixed number of images. Covers and unrelated standalone pictures do not become storyboards. Use [core](CORE.md) when its short rules are needed; reuse loaded text.

## Visual execution and bounded comparison

Method revision: visual-vfx-2026-10-16.1.

Use [visual quality and control frames](references/visual-quality.md) for a new look, a "not attractive enough" complaint, local appearance repair or an effect control frame. Apply the approved attention order, composition, light/value separation and material treatment to the actual prompt. For each reference, specify what to borrow and what not to borrow; identity does not import its background, pose or lighting. A colour filter or more style adjectives does not repair a weak composition.

An action board explains sequence; a look frame selects appearance; a production control frame supplies a supported endpoint state. Do not conflate them or silently change the requested deliverable. A board remains a board when requested; exact control frames must be independent full-frame images, never the whole grid bound as a first or last frame. Extra frames and alternative looks need existing budget authorization.

When alternatives are authorized, vary a named design decision rather than merely rerolling the same prompt. Compare candidates against the chosen reference direction and hard constraints, not a made-up beauty score. Preserve the selected image through supported local edits when possible; one local defect is not a reason to reinvent the whole scene. Inspect available actual pixels and identify unverified properties. Without image access, return an unverified result, not visual approval.

For an effect frame, depict one phase and its material state, protect specified identity/label regions, and keep start/end composition compatible. Do not put an entire evolving effect into a still. Keep planning notes out of unsupported tool fields; compile relevant decisions into the actual submitted image prompt and concrete reference purposes. Missing assets go back through the existing coordinator handoff, not an invented delegation tool.

## Physical continuity

Keep cross-frame continuity through stable identity and construction, while pose, support, openings and visible projection change according to the action and camera. Physical plausibility governs every depicted state; "identical in every frame" must not freeze moving parts or override gravity, contact, material response or mechanism operation. For state changes and local edits, apply [physical state rules](references/prompt-writing.md#physical-state-and-continuity) and include the relevant physical relationships in the actual generation prompt, not only in planning notes. Preserve explicitly designed departures from ordinary physics within their stated scope.

## Bind every input to a concrete purpose

Each supplied image is one of: edit target, identity or wardrobe reference, product geometry and material evidence, composition or state reference, or motion reference. Name that use plus the subject it supplies in `references[].purpose` — for example "the lead's face and hair identity", "the opened main compartment as state reference", "customer's own product front panel for geometry" — never a bare label such as "content reference". `role` selects the technical input mode and `purpose` describes the visual use; they are separate fields and both must be correct.

Purposes carry identity assignments. When a scene has more than one recurring person or object, bind each input to the named subject, and keep that binding consistent through the prompt text and any later segment. An image simply labelled "reference" cannot hold a character's identity.

## What a reference does and does not transfer

An identity asset fixes who the subject is. It does not fix pose, camera, background or lighting for a scene frame: do not inherit its standing pose or studio light into a scene, and do not restate them as if they were requested.

A reference of an opened bag, a mid-action state or a detail close-up supplies only its declared properties. Its state does not automatically bind a new composition. It is not an actual video first-frame control; when used as a control/edit base, its existing state must be compatible with the requested change.

## Boundaries

Script content, shot design and video generation belong to other Skills. Fix execution deviations locally; return design conflicts with evidence. Static frames never establish motion, speed, sound or playback.
