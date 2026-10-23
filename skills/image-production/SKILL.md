---
name: image-production
description: Produce standalone stills and reusable character, wardrobe, prop or location references in Mstudio while preserving the selected project art direction.
---

# Image production

Produce the requested still image and the reusable references it depends on. Preserve the chosen concept, the selected identity and original product geometry. Loading this Skill grants no extra tools or permissions: saving a prompt is not a generated image, and task submission is not a completed image.

Specialists execute assigned production work and return real outputs and task status. Only the coordinator conducts internal review; the user makes the final selection. Reading references is for execution, not approval. Apply assigned revisions without initiating self-review, acceptance gates or extra generation.

## Reference assets and scene images

For a reusable character, wardrobe, prop or location asset, start with [reference assets](references/reference-assets.md). Make identity, silhouette, proportions, outfit, construction and material easy to inspect at the needed scale. Choose a single view or a useful consistent view sheet; do not apply storyboard panel counts, action beats, internal cuts or shot timing to an asset request. Simplified backgrounds are optional aids; for a photographic character, a real room or textured surface with directional light is usually stronger than a seamless wall. Location references retain the needed environment and layout.

Select the subject method before writing an asset prompt: [character assets](references/character-assets.md) for people, including age-appropriate anatomy, skin and wardrobe; [object assets](references/object-assets.md) for props and products, including construction and material response; [scene assets](references/scene-assets.md) for environments, spatial anchors and light states. Read only the relevant branch. For a person using an object or entering an environment, combine the settled references with explicit subject purposes and check their contact and scale.

Assets and storyboard images follow the same selected project art direction: medium, realism or stylization, era, palette relationships, material treatment and finish. Reference clarity does not authorize a generic catalogue look, a different character design or a lower standard of craft. Asset lighting and pose can differ from the scene; they do not override its narrative lighting, camera or action. Apply the common [visual quality method](references/visual-quality.md) with asset-specific checks.

For live-action or photorealistic work, read [photographic direction](references/photographic-direction.md) before writing the prompt. Decide the capture, lens and distance, camera height, light source and ratio, color relationship and surface finish first. A character asset is an identity plate, not a catalogue cutout: head-and-shoulders or waist-up, unposed, in a real room or against a textured surface with directional light, with the selected wardrobe and skin tone readable at the intended scale. A full-body neutral-backdrop view is justified only when a consuming shot needs body scale or the complete outfit.

## Read for the task at hand

| The task has to decide | Read |
|---|---|
| How to write one prompt for one visible moment | [Image prompt writing](references/prompt-writing.md) |
| How to choose the photographic treatment, camera, light and color without the AI look | [Photographic direction](references/photographic-direction.md) |
| Skin, age, hands and synthetic-skin repair | [Skin realism](references/skin-realism.md) |
| Whether subject, location or layout references are missing, and how to design useful reference assets | [Reference assets](references/reference-assets.md) |
| Character identity, age, anatomy, skin, hair and wardrobe | [Character assets](references/character-assets.md) |
| Writing a photographic person prompt or diagnosing synthetic skin | [Photographic people prompts](references/photographic-people.md) |
| Object geometry, components, connections and material fidelity | [Object assets](references/object-assets.md) |
| Environment baseline, layout, camera geography and motivated light states | [Scene assets](references/scene-assets.md) |
| Viewpoint, contact, part count and cross-frame state constraints | [Frame rules](references/frame-checks.md) |
| Staging, attention centre or spatial relationships that need redesigning | [Shot grammar](references/scene-execution.md) |
| Scene-specific light and material appearance | [Photographic appearance](references/scene-execution.md) |

Shot storyboards and video control frames belong to image production through the coordinator. This method produces the assigned standalone still or reusable asset. Use [core](CORE.md) when its short rules are needed; reuse loaded text.

## Visual execution and bounded comparison

Use [visual quality and control frames](references/visual-quality.md) for a new look, a "not attractive enough" complaint, local appearance repair or an effect control frame. Apply the approved attention order, composition, light/value separation and material treatment to the actual prompt. For each reference, specify what to borrow and what not to borrow; identity does not import its background, pose or lighting. A colour filter or more style adjectives does not repair a weak composition.

Do not ask one call to establish identity, geography, four states and the final look at once. A look frame, an identity plate, a location plate and a storyboard board are different deliverables; produce the authorized one and return a missing dependency through coordination. When the task is a look frame, one frame and one visual idea are complete work, not an incomplete storyboard.

When alternatives are authorized, vary a named design decision rather than merely rerolling the same prompt. Return alternatives with their intended design differences for coordinator review and user selection. Apply assigned local edits to the selected image without reinventing the scene. Return real output IDs and execution limits to the coordinator without claiming acceptance.

For an effect frame, depict one phase and its material state, protect specified identity/label regions, and keep start/end composition compatible. Do not put an entire evolving effect into a still. Keep planning notes out of unsupported tool fields; compile relevant decisions into the actual submitted image prompt and concrete reference purposes. Missing assets go back through the existing coordinator handoff, not an invented delegation tool.

## Physical continuity

Own product fidelity in the rendered image: bind the original evidence, preserve the components and materials visible from the designed camera, and correct observed execution drift within the authorized scope. Direction supplies the shooting decisions and factual boundaries. Do not flatten its camera choices into the source photograph's angle, expose every component in every view, or turn a local product error into a new shot design. Return only an actual design/factual conflict to direction.

Keep cross-frame continuity through stable identity and construction, while pose, support, openings and visible projection change according to the action and camera. Physical plausibility governs every depicted state; "identical in every frame" must not freeze moving parts or override gravity, contact, material response or mechanism operation. For state changes and local edits, apply [physical state rules](references/prompt-writing.md#physical-state-and-continuity) and include the relevant physical relationships in the actual generation prompt, not only in planning notes. Preserve explicitly designed departures from ordinary physics within their stated scope.

## Bind every input to a concrete purpose

Each supplied image is one of: edit target, identity or wardrobe reference, product geometry and material evidence, composition or state reference, or motion reference. Name that use plus the subject it supplies in `references[].purpose` — for example "the lead's face and hair identity", "the opened main compartment as state reference", "customer's own product front panel for geometry" — never a bare label such as "content reference". `role` selects the technical input mode and `purpose` describes the visual use; they are separate fields and both must be correct.

Purposes carry identity assignments. When a scene has more than one recurring person or object, bind each input to the named subject, and keep that binding consistent through the prompt text and any later segment. An image simply labelled "reference" cannot hold a character's identity.

## What a reference does and does not transfer

An identity asset fixes who the subject is. It does not fix pose, camera, background or lighting for a scene frame: do not inherit its standing pose or studio light into a scene, and do not restate them as if they were requested.

A reference of an opened bag, a mid-action state or a detail close-up supplies only its declared properties. Its state does not automatically bind a new composition. It is not an actual video first-frame control; when used as a control/edit base, its existing state must be compatible with the requested change.

## Boundaries

Script content, shot design and video generation belong to other Skills. Execute assigned local revisions; return design conflicts to the coordinator. Static frames never establish motion, speed, sound or playback.
