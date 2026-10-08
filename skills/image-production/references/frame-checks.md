# Storyboard previews and frame inspection

Preview frames communicate composition, product relationships, character design and key story states. They are not automatically video start frames.

## Prepare the requested frames

For a complete visual storyboard, select representative frames: the opening, the key change, the value evidence and the result. Text-only requests, prompt edits and Skill maintenance do not trigger generation. Test the hardest static relationship before expanding dependent frames; reuse accepted images and prefer an edit for the same viewpoint. A local revision does not restart the whole sample process.

Read [image prompting](prompt-writing.md) before writing a frame prompt: one moment, a compatible viewpoint and visible extent, lighting, and reference roles. Keep shot IDs, timecodes, arrows and captions out of clean scene images unless the user asked for an annotated design.

`mstudio_generate_image` creates the task; inspect its actual state and result. For reference organisation and repairs use [controlled generation](../../product-video-production/references/generation.md) within the authorised image scope.

## Inspect the actual pixels

Compare the output with the original product evidence: structure, colour, visible parts, count, state, hand and object attachment, support, perspective, framing restrictions and reflections. For count and boundary-sensitive objects, record the invariants: opening count, divider placement, connections. Do not add structure to fit props, and do not use a generated image as product fact.

Check the path and connectivity of parts, not only their presence: a closure on the wrong surface or handles on the wrong part fail identity even with matching colour and texture. Compare dependent frames side by side for these invariants, landmarks and the intended state change. A new viewpoint changes visibility, not construction. Record the anchor asset and the observed relationships in the existing handoff.

After a repair, inspect the affected region, the whole image and the dependent frames. Reject known major defects before expanding. If the character or visual direction was rejected, fix that sample before propagating it. Judge whether a relationship reads with explanatory labels hidden; arrows cannot replace missing contact, a wrong viewpoint or an absent state.

Infer the viewpoint from visible surfaces, occluding edges and perspective, not from the title or prompt. A material repair does not establish an unchecked camera position. A user pointing out one defect does not approve everything else. Keep the relevant unknowns without expanding into unrelated review.

For realism requests apply [perceptual realism](../../product-video-production/references/generation.md#perceptual-realism). Always apply [visible product components](../../product-video-production/references/generation.md#visible-product-components); similar colour and silhouette are not enough.

## Present and hand off

Arrange the selected frames in playback order with IDs and times matching the shot plan. Add start and end frames only when one moment cannot explain a required change; count the shot duration once. Show readable images, enlarging a frame when a contact sheet hides detail; a download link does not replace a requested preview.

Keep captions brief; motion, cuts and sound belong in the shot description. Preserve the final prompts and reference roles. Distinguish previews, inspected frames and unverified motion. Choose production frames for the actual control mode: reuse a valid start, never pass a middle-state image as one.
