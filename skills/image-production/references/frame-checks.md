# Storyboard previews and frame inspection

Preview frames communicate composition, product relationships, character design and key story states. They are not automatically video starting frames.

## Prepare the requested frames

For a complete visual storyboard, follow the selected shot plan and [frame selection rules](frames.md). Cover its necessary viewpoints and changes without imposing a fixed opening/change/value/result template. Text-only requests, prompt edits and skill maintenance do not trigger generation. Test the hardest static relationship before expanding dependent frames; reuse suitable accepted images and prefer edits for the same viewpoint. Local revisions do not restart the entire sample process.

Read [image prompting](prompt-writing.md) before writing a frame prompt. Select one moment, compatible viewpoint and visible extent, lighting and reference roles. Keep shot IDs, timecodes, arrows and captions outside clean scene images unless the user requests an annotated design.

In Mstudio, request_generation with image creates the task; inspect its actual state and result. Creation is not completion. For reference organization and repairs use [controlled generation](scene-execution.md) only within the authorized image scope.

## Inspect actual pixels

Compare output with original product evidence: structure, color, visible parts, count, state, hand/object attachment, support, perspective, framing restrictions and reflections. For count/boundary-sensitive objects, record relevant invariants such as opening count, divider placement and connections. Do not add structure to accommodate props or use a generated image as product fact.

Check the path and connectivity of parts, not only whether they exist: a closure on the wrong surface or handles attached to the wrong part fail identity even when color and texture match. Compare dependent frames side by side for these invariants and the intended state change. Same-view frames retain landmarks; changed viewpoints retain world relationships with the appropriate new projection, rather than identical screen positions. A new viewpoint can change visibility, not the underlying construction. Record the anchor asset and observed relationships in the existing handoff; no new approval artifact is needed.

After a repair inspect the affected region, the full image and dependent frames. Reject known major defects before expansion. If the character or visual direction has been rejected, fix that sample before propagating it. Hide explanatory labels when judging whether the relationship reads; arrows cannot replace missing contact, a wrong viewpoint or an absent state.

For a sequence board, verify panel order, causal change, recurring identities and readable state in each panel. A caption saying "injured", "escaping" or "fits" is not evidence that the corresponding condition is visible. Check requested labels for errors separately; a polished panel can still omit its decisive action.

Infer viewpoint from visible surfaces, occluding edges and perspective, not the title or prompt. A material repair does not establish an unchecked camera position. A user pointing out one defect does not approve everything else. Retain relevant unknowns without expanding into unrelated review.

For realism requests use [perceptual realism](scene-execution.md). Always apply [visible component preservation](scene-execution.md); similar color and silhouette are insufficient.

## Present and hand off

Arrange selected frames in playback order, with IDs and times matching the shot plan. Add start/end frames only when one moment cannot explain a required change; count the shot duration once. Show readable images, using larger individual views when a contact sheet obscures detail. A download link alone does not replace a requested visible preview.

Keep captions brief; record unobservable motion, cuts and sound in the shot description. Preserve final prompts and reference roles. Distinguish previews, inspected static frames and unverified motion. Choose production frames for the actual control mode; reuse a valid start, but do not pass a middle-state image off as one.
