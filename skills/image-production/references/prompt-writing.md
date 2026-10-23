# Write visible states for the intended image deliverable

Translate the selected design into a visible instant. Keep the requested art direction; a photograph, rough storyboard and graphic layout need different treatment. A scene panel cannot contain incompatible before/after states; a sequence layout assigns them to separate panels.

## Select the deliverable and moment

Distinguish a new scene, local edit, reusable identity asset, clean storyboard/control frame and finished cover/graphic. This determines whether background, annotations and visible copy belong in the output; neither white background nor text-free output is universal.

Select the requested start, key or end state before describing it. A full shot script is context, not an instruction to put its whole sequence into one still. Covers and representative frames may show the result; actual preparation frames must precede the required change. When the choice materially changes the composition or reveal and intent remains unresolved, identify that decision instead of guessing. Ordinary secondary details do not require clarification.

Describe motion as a visible pose, support/contact, fabric tension or deliberately requested motion blur. Do not carry camera travel, playback timing or sound instructions into a still. A completely dark image can be an intentional control frame when explicitly requested; do not replace it with a visible product reveal. For a whole-sequence review, a multi-panel board is a valid route: state reading order and one event/state per panel. Individually requested images and production controls stay separate. Do not confuse a subject reference sheet with a story sequence.

For an assigned standalone still, depict only its selected instant. Movement, camera trajectories, dialogue and sound remain handoff context. Reference assets establish identity, construction or layout rather than a shot sequence.

## Visible decisions

- Moment and pose: locate the body/object, its support, weight and decisive contact. Distinguish anticipation from result; do not reveal a planned surprise early. For a difficult pose, consult [animation principles](scene-execution.md).
- Camera and composition: realize the selected view direction, height, lens/distance relationship, subject scale and crop. Translate optical intent into the required perspective, relative foreground/background scale and visible surfaces; a focal-length label alone is insufficient. Do not inherit the product reference camera or change the director's view to expose more components. Consult [shot grammar](scene-execution.md) for an execution question; return essential redesign to direction.
- Light and focus: carry the selected scene lighting and depict the selected sharp subject/plane, depth of field and background legibility. A still shows one focus state; a timed focus transfer remains video handoff context. Preserve common material colors and light relationships across the sequence while allowing justified changes with viewpoint.
- Output: distinguish a clean scene frame, annotated storyboard and graphic design. Put presentation labels outside clean video-input frames; when text is requested, specify exact copy and placement.

Script, dialogue, sound and camera trajectory are context. Include only what is visible at this moment, with a sound source only when needed. Add details to resolve ambiguity, not new events, people, props or slogans. Surreal effects change the chosen relationships while retaining a compatible source and state.

## Write a photograph, not a production checklist

One call should do one job: an identity plate, a location plate, a look frame, a board, or a local edit. Do not ask one prompt to establish identity, geography, four states and the final look at once.

Use five short blocks as a drafting aid, then merge them into coherent prose:

1. **Capture and camera.** Photographic fact or medium, lens, distance, height and the visible consequence.
2. **Instant and blocking.** Who or what is where, the action or held state, gaze, hands, support and contact.
3. **Light and color.** Source, size, direction, ratio, bounce, palette relationship and what stays neutral.
4. **Material and skin at scale.** Two or three specific responses that matter at this framing.
5. **Reference roles.** One line per input: what it supplies and what it must not transfer.

Keep continuity rules, part counts, negative constraints and acceptance checks in the task checklist or `references[].purpose`. Repeat a constraint in the prompt only if the model has already violated it. The prompt should read as a photograph; five unrelated clauses will be averaged into a safe catalogue image.

Example of the same scene written as a photograph:

> A photorealistic 35mm film still, camera at seated eye level on wet sand, 50mm at T2.8. A woman singer stands on a low timber platform at dusk, hand just released from a microphone, looking down-left with a small dawning smile. In the lower left foreground an older woman sits cross-legged, hands meeting in a clap. The sun is below the horizon: a thin apricot band remains, the sky above is dusty blue-grey, the sea is dark slate. Warm sand bounce lights the faces; a cool ambient fill keeps the shadow side readable. Rust-red linen, cream trousers, grey curls, weathered planks and matte black metal each catch the light differently. Fine grain, soft highlight roll-off, open shadows, muted natural color.

Not: a paragraph of "no text, no logo, no extra people, no orange filter, do not copy the reference pose, keep the same identity, 1920x1080" before the scene has been described.

## Spatial coherence before prose

Fix the camera location relative to the subject, its viewing direction, visible surfaces and occluded regions before adding style. Describe the selected instant: aperture open/closed, object support and contact, foreground/background and light source/path must agree. An interior view through an opening and an exterior product view expose different surfaces; do not combine them to display every product feature. A reference of the outside can establish identity without becoming the requested composition.

If the design requires mutually incompatible states or viewpoints, identify the exact conflict before generation rather than inventing a compromise. Ordinary missing details may be resolved within the selected design; do not reopen decisions already made by the user. Keep this reasoning out of the final prompt: write the resulting visible image in natural prose, with only supported model-specific reference syntax added afterward.

## Physical state and continuity

Separate stable properties from changing state. Identity, part count, attachment topology and actual dimensions persist unless the design explicitly changes them. Pose, tension, deformation, opening extent, movable-part positions, occlusion and projected shape follow the selected action, material and viewpoint. Do not lock these together in a blanket "identical in every frame" clause.

For each relevant state change, establish the physical cause and resulting visible state: what applies force or releases support, what remains attached, where contact occurs, and how gravity, inertia, material stiffness and mechanism travel affect the result. Across panels, these relationships must permit the transition; each panel still depicts one instant. A static or edited image must also have plausible support and contact without requiring a motion sequence.

Carry the task-relevant relationships into the final prompt as a short physical-behavior paragraph or alongside the affected panel. Describe observable consequences, not merely "obey physics": a released flexible strap becomes slack while its roots and length remain unchanged; a hinged panel rotates without shrinking its surface; a closure's moving parts correspond to its opening state. These are conditional relationships, not a requirement that all handles droop or all objects deform. Use the evidenced material and mechanism; do not invent hidden anchors, measurements or hardware to explain motion.

When a consistency instruction conflicts with the requested action, retain the evidenced structure and rewrite the state constraint to allow the physically required change. Do not silently change the action or redesign the product. Explicitly surreal or stylized behavior follows the selected world rules; unrelated support, contact and identity relationships still hold. Keep unsupported structure unknown.

## Match prompt detail to actual inputs

A strong supplied image can carry appearance and layout; a simple cleanup, new view or local edit may need only the requested transformation plus essential invariants. New scenes and designed multi-view assets need the missing composition, relationships and visible details. Do not impose one prompt length or field template on both.

For a sequence, keep the accepted identity, location, time of day and art direction as the shared baseline; change only the per-panel event, state and selected camera. Independent calls must receive that baseline in their text or actual references: an earlier description is not automatically visible. Use actual input order and supported syntax rather than copying another platform's asset aliases.

## Product structure and references

Use original image evidence for openings, closure paths, part count, connections and visible surfaces. A parts list or 'same product' does not resolve contradictory geometry. Preserve construction without demanding that hidden parts become visible; count only features that the chosen view should reveal. A proposed staging action cannot redefine the product. Return an evidenced design conflict to direction; hidden geometry remains unknown. Texture similarity alone cannot establish structural fidelity.

Identify supplied images in actual order and describe each visual use: identity/geometry or composition/light. In tool arguments, write that description in purpose; role selects the technical input mode, reference for guidance or edit for the source image being modified. Product photos constrain the product, not the entire original arrangement. Preserve actual markings while excluding irrelevant dimension labels or interface chrome.

An independent request cannot see 'the previous frame' without that image. Supply the selected base for same-view continuity alongside original product evidence. A generated base supplies composition/state, not proof of unseen mechanisms. Use 'change only X' language for an actual image edit; a new scene remains a new generation. State a model limitation if it cannot accept the required references.

## Edits and reference scope

For an edit, identify the supplied base, target region, desired replacement and surrounding properties to retain. Do not rewrite the entire scene when only a hand or closure needs repair. For a new scene, describe its composition explicitly instead of saying 'change only' against an identity reference.

Reference purposes are selective: geometry, identity, state, composition and lighting are separate constraints. An open or closed product photo binds that state only when requested or used as an actual composition/control/edit input. Outside views can support product identity without defining an interior composition or proving unseen interior details. Resolve conflicting actual control inputs before generation.

## Final prompt and image check

Write the visible output, selected state, composition, light and necessary reference constraints in direct paragraphs. Specifications belong in supported parameters. Keep planning notes, review history and quality slogans out of the final text. Describe material behavior rather than adding universal negative lists.

Compare actual output with the intended state, viewpoint, support/contact, identity, components and requested text. For a sequence, also check that changing support, tension, openings and movable parts follow the depicted action rather than repeating a frozen reference state. Repair the observed mismatch. Fine detail does not by itself establish realism.

For photographic work, also check the camera relationship, light direction and ratio, color relationships and skin/material response. If the result reads as synthetic, use the diagnosis in [visual quality](visual-quality.md) and repair the cause rather than adding more detail.

For photographic people, skin appearance, environmental lighting or a restrained realist style, read [photographic direction](photographic-direction.md) and [skin realism](skin-realism.md), plus [photographic appearance](scene-execution.md) for scene-specific light. These examples do not set the style for other projects.
