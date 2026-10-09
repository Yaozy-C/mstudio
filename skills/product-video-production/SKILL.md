---
name: product-video-production
description: Produce video in Mstudio: compile a shot design into a final model prompt, choose inputs and control routes, generate and repair clips, and judge the actual result.
---

# Video production

Turn an approved design into a final, usable video prompt and generated media, then judge the result from actual pixels. Use the supplied design; do not redesign the story or quietly widen the task. Loading this Skill grants no extra tools or permissions.

## Read for the task at hand

| The task has to decide | Read |
|---|---|
| How to describe changing screen states, subject/camera motion, focus, light and sound in one final prompt | [Video prompt writing](references/prompt-writing.md) |
| Which input mode and control route can express the planned action | [Control routes](references/control.md) |
| Reference selection, segmentation and recovering a specific defect | [Controlled generation](references/generation.md) |
| **How long each generated segment earns, what to leave out, and how cuts and sound line up in time** | **[Rhythm](references/shot-execution.md)** |
| **How a person's reaction starts and layers on camera** | **[Naturalistic performance](references/shot-execution.md)** |
| Weight, force, contact and material response inside the action | [Animation principles](references/shot-execution.md) |
| What a finished clip must be checked against | [Inspection evidence](references/evidence.md), [review and delivery](references/review.md) |

A request that packs several timed events into a limited duration is itself the signal to read rhythm before writing the prompt, and a request showing real people is the signal to read performance. Do not wait for an explicit complaint.

Inspect generated action and identify usable source candidates for handoff. Final source selection, trimming and timeline assembly belong to editing; pass the observed events, candidate ranges and defects through the existing project records rather than performing timeline work under production permissions.

Read [core](CORE.md) for the short rules.

## Physical causality is not a full operating sequence

An action has to be causally believable: its starting support, path, contact and release must be consistent, and nothing may cross a closed surface. That is a requirement on the action, not an instruction to show every step of it.

Follow the selected design and user requirements when deciding which action phases must be visible. Preserve required continuous proof, anticipation and recognition; omit only nonessential preparation, repeated reaches, redundant handling and idle endings. Keep the supporting geometry consistent in what you do show. When a model cannot render the whole chain reliably, obtain the smallest supported segment that carries the needed event rather than lengthening the prompt to cover every stage.

## Bind every input to a concrete purpose

Each supplied image or video is one of: edit target, first frame, last frame, identity reference, product geometry and material evidence, composition or state reference, or motion reference. Name that use plus the subject it supplies in `references[].purpose` — for example "the lead's face and hair identity", "coworker's face and build identity", "opened main compartment as state reference", "reference clip 2.0-4.0s for hand-over motion" — never a bare label such as "content reference". `role` selects the technical input mode and `purpose` describes the visual use; both must be correct and consistent with the prompt text.

When more than one recurring person or object appears, bind every input to its named subject and keep the binding stable across segments. An unlabelled reference cannot carry a character's identity, and adding more unlabelled references does not fix a missing assignment.

## Model inputs and parameters

Use the selected model's current injected rules and capability schema; query model guidance only when it is missing or has changed. Distinguish ordinary appearance or composition references from actual first or last frame controls. Verify that an input's geometry and starting support can produce the intended event before relying on it; declaring a conflicting middle state "not the first frame" does not remove the conflict. Keep user-locked aspect, resolution and duration in supported parameters, and state an unsupported requirement explicitly instead of silently changing it.

## Result state

Submitted, generated, imported and visually inspected are four different states. A `COMPLETED` job proves the model finished, not that the clip is usable. When a result exists, check it against the original submitted prompt and the intended action before describing or reusing it: read actual frames for geometry, contact, identity and continuity, and state plainly which conclusions rest on frames, which need playback, and which still need listening. Report an evidence gap rather than implying a check happened.

When a result is judged unusable, repair in this order: the smallest useful change to the input or the reference assignment, then the smallest split or cut that preserves the intended expression, then a different control route. Do not answer a diagnosed physical failure with longer negative lists, louder sound or global retiming.
