# Shot design: from script to shot records

Turn the selected script into executable shots. [Scriptwriting](../../ad-script/SKILL.md) owns the story; this Skill owns viewing order, camera, staging and continuity; [image production](../../image-production/SKILL.md) executes frames. Read a linked reference to settle a concrete question, not as a checklist.

## Scope and reading route

For a full film, read the complete current script and existing shots. For a local revision, start from the target shot, its script paragraph and the relevant original evidence; read neighbouring shots only for a concrete continuity dependency. Inspect additional product views only for a feature a shot actually uses.

Separate explicit requests, product facts and exploratory assumptions. Preserve the selected viewing experience and essential events. Distances, complete operating sequences and timings you proposed remain revisable design decisions, not user locks. Audience, language, face policy and style follow the current project.

| Decision | Read |
|---|---|
| Shot purpose, viewpoint, staging, cuts | [Shot grammar](cinematography.md) |
| Weight, force, performance beats | [Animation principles](animation-principles.md) |
| Timing, pauses, cuts and sound | [Rhythm](rhythm.md) |
| State continuity and asset changes | [Continuity](continuity.md) |
| A weak opening, continuation or product connection | [Concept development](concepts.md) |

## What one shot record holds

An editorial shot is one continuous viewpoint; a Mstudio shot record may group several editorial shots into one generation group. Write the record so production can read it without you:

1. Purpose and evidence: what changes for the viewer and the visible cause.
2. Camera and staging: opening and closing composition, viewpoint, attention centre, positions of subject, target and obstacle, camera path with its trigger, speed and focus.
3. Action beats: necessary anticipation, main action, contact, outcome and reaction; order of primary and secondary movement; permissible omissions.
4. Cut relationship: why to cut or hold, the outgoing and incoming states, direction and where attention sits.
5. Locks and freedom: what must survive (continuity, camera relationships, event order, required evidence, identity and parts) and what production may vary. Mark which locks are the user's.
6. Frames: the frame moments the shot needs, the relationships each must show, and what a still cannot verify.

Simple shots can use short sentences. Do not create fields the tools do not have, and do not split shots by word count or model duration.

### Worked example: one shot record

Shot 2 of the lunch-bag piece (script paragraph p3), written as the shot's text. It is an illustration of the record format, not a template for other films.

```text
Purpose: the viewer learns that the bag opens wide enough to lift a full container straight out. Visible cause: zipper travel, lid lifting on the rear seam, the container rising through the open top with clearance.
Camera and staging: close three-quarter view from the lead's side, desk edge at the bottom of frame; the bag's front pocket and both zipper pulls visible at the start. Attention on the slider, then on the opening. Static camera; no move needed because the change happens inside the frame.
Beats: (1) hand already on the slider, one short pull across the full zipper; (2) lid lifts and settles back on its seam; CUT; (3) overhead-leaning view, container lifted vertically with clearance, lowered onto the clear desk beside the bag, hand stays until the base lands. Omitted: finding the slider, re-grips, the second hand repositioning.
Cut: a motivated cut between (2) and (3) because the viewer needs a new angle to read clearance; outgoing state is the open lid at rest, incoming state is the hand already gripping the container. Attention stays on the opening.
Locks: bag geometry per asset a-product-front, two silver pulls, lid hinges at the rear seam, container fits fully inside (user lock: the product must not be modified). Free: exact finger placement, micro-timing, which hand steadies.
Frames: f1 just after the lid settles (shows opening and container inside), f2 container clear of the top edge. Neither still can show the zipper speed or the hand release; playback must.
```

`shot.duration` for this record is the generation span the model needs for beats 1 to 3, not the 3 seconds the film will keep; the intended film range is written in the text.

## Save

- Staging, camera, beats, cuts and locks go into the shot's text with `mstudio_update_node(id, text)`.
- `shot.duration`, `shot.dialogue`, `order`, `screenplayId` and `scriptId` go through `mstudio_update_shot`; use `mstudio_update_shots(items)` for atomic reorders; create missing shots with `mstudio_add_shot`.
- For a generation group, `shot.duration` is the group's generation span, not its final screen time. Keep the internal editorial shots, their intended film ranges and explicit cuts in the text; production adds actual source in and out points after inspection. Never invent source timestamps before footage exists.
- Reuse existing shot IDs. A shot spanning several paragraphs links to the primary `scriptId` and names the others in text.
- Attach references with `mstudio_upsert_references(id, references=[{assetId, purpose}])`, naming the subject and the visual use.

Verify the receipt, then return the affected IDs, the changes and the open unknowns.

## Hand off

Give the artist the selected moment, viewpoint, attention centre and spatial or pose constraints for each frame; text-only tasks do not generate images. Give production the action, camera, timing and hard constraints; essential events are locked, provisional implementation choices are not. Use the [production handoff](../assets/production-brief-template.md) without copying what is already saved. The director fixes design conflicts; the artist fixes execution deviations. Static correctness does not establish motion, speed or sound.
