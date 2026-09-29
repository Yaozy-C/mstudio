# Write one image at one visible moment

Required before every prompt: read [animation principles](animation-principles.md) in full. For images, translate relevant principles into the selected visible pose, staging, support, weight and anticipation/result state; for videos, into observable motion and performance. Apply the selected model’s injected syntax and constraints separately; this guide remains model-independent.

Use for storyboard frames, production frames, product scenes and local image edits. Translate selected shot intent into a visible instant. Time evolution belongs in video prompting. Retain the current tool, model and authorization.

## Output and context

Distinguish a clean scene frame, annotated storyboard layout and graphic design with onscreen copy. Clean video-input frames have no shot number, caption, subtitle or layout by default; put presentation labels outside the image. If the user requests text or panels, specify exact content and placement instead of banning all text.

Script, dialogue, sound and camera trajectory are reasoning inputs, not fields to concatenate into the prompt. Select the state actually visible now: opening, contact, support, occlusion and position. Do not show future events early. Convert sound to a visible source only when needed. Do not append conversation history or review notes.

## Resolve visible ambiguity

Preserve user composition, style and concept. Add only detail needed for this image; do not invent extra people, props, slogans or effects. Resolve conflicts using established intent; ask only when a material locked conflict cannot be resolved.

- Moment: before, during or after the action, without incompatible simultaneous states.
- Camera/composition: aspect, subject scale, visible extent and crop; compatible camera height, direction and visible surfaces.
- Light/focus: coherent time, source direction, palette and focus; required background cues must survive depth of field or bloom.
- Effect/material: express mood through visible mechanisms, distinguishing refraction, particles, liquid, vapor and emission. Surreal effects change only selected relationships. Use targeted constraints for actual ambiguity rather than universal negative lists.

## Reference roles

For a recurring real product, inspect original images before asserting geometry. Extract only the structural relationships relevant to this view: opening and closure path, part count and placement, attachment points, and which surfaces become visible when it opens or turns. A parts list or "same product" is insufficient. Keep these invariants in the existing frame description and carry them into each self-contained request. Distinguish verified structure from occlusion or unknown geometry; a new angle does not authorize inventing a seam, opening or connection.

Compare the proposed shot and prompt against those relationships before submission. Original product evidence governs identity; a director's proposed action or composition cannot redefine it. If they conflict, return the specific mismatch to direction and preserve the intended reveal while the shot is corrected. Do not submit contradictory geometry alongside "strictly preserve identity". Effects need a compatible physical source and state; an empty container cannot stand in for visible hot contents merely by adding vapor.

Identify inputs in actual order as product identity, composition/light reference or edit target. Product photos constrain visible geometry, connections and material, not the entire original arrangement. Exclude irrelevant dimension annotations, packaging text or interface chrome while preserving required actual product markings.

A new scene with references is still a new generation. Use only-change-X/preserve-Y editing language for an actual existing-image edit. Reuse an inspected base frame for same-view continuity without treating it as factual product evidence.

An independent request cannot see "the previous frame" unless that image is actually supplied. For dependent frames, use the inspected base for composition/state continuity alongside original product evidence, with distinct reference roles. If the selected model cannot accept the needed references, report that limitation rather than promising an identical frame through text alone.

## Submission and inspection

Keep the actual output, visible state, composition, light, reference roles and necessary constraints. Short paragraphs are sufficient. Set specifications through supported parameters; text mentioning dimensions is not proof that an API parameter was set.

Check final text, reference order and parameters against the task preview. No hidden append of metadata, scripts or templates; reference-role text added by a platform is part of the inspectable request. Save the actual request and identify inaccessible provider rewriting as unknown.

Inspect actual state, composition, physical appearance, text and expected components. Texture detail is not perceptual realism. Repair visible deviations rather than adding realistic or high-resolution. Follow existing retry and authorization limits; finishing a prompt does not authorize generation.
