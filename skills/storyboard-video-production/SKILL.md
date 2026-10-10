---
name: storyboard-video-production
description: "Produce video from temporal storyboard boards in Mstudio: map panel states to continuous action or planned cuts, use verified reference inputs, generate and repair clips, and preserve the selected action and visual constraints."
---

# Storyboard video production

Use one temporal board per selected shot as the default storyboard reference workflow. Turn its approved state sequence and shot design into a final video prompt and generated media, and preserve its event order. The board provides planning constraints, not guaranteed frame-level control; explicitly requested exact-control tasks retain their required mode. Use the supplied design; do not redesign the story or quietly widen the task. Loading this Skill grants no extra tools or permissions.

## Read for the task at hand

| The task has to decide | Read |
|---|---|
| How to describe changing screen states, subject/camera motion, focus, light and sound in one final prompt | [Video prompt writing](references/prompt-writing.md) |
| Which input mode and control route can express the planned action | [Control routes](references/control.md) |
| Reference selection, segmentation and recovering a specific defect | [Controlled generation](references/generation.md) |
| **How long each generated segment earns, what to leave out, and how cuts and sound line up in time** | **[Rhythm](references/shot-execution.md)** |
| **How a person's reaction starts and layers on camera** | **[Naturalistic performance](references/shot-execution.md)** |
| Weight, force, contact and material response inside the action | [Animation principles](references/shot-execution.md) |
| Output constraints and delivery | [Output claims](references/evidence.md), [delivery rules](references/review.md) |

Use rhythm for timed events and performance for human behavior when those rules are needed; reuse loaded text.

Final source selection, trimming and timeline assembly belong to editing. Hand off actual generated assets and any known missing events through existing project records; do not perform timeline work under production permissions.

Use [core](CORE.md) when its short rules are needed; reuse loaded text.

## Physical causality is not a full operating sequence

An action has to be causally believable: its starting support, path, contact and release must be consistent, and nothing may cross a closed surface. That is a requirement on the action, not an instruction to show every step of it.

Follow the selected design and user requirements when deciding which action phases must be visible. Preserve required continuous proof, anticipation and recognition; omit only nonessential preparation, repeated reaches, redundant handling and idle endings. Keep the supporting geometry consistent in what you do show. When a model cannot render the whole chain reliably, obtain the smallest supported segment that carries the needed event rather than lengthening the prompt to cover every stage.

## Bind every input to a concrete purpose

Each supplied image or video is one of: edit target, first frame, last frame, identity reference, product geometry and material evidence, composition or state reference, or motion reference. Name that use plus the subject it supplies in `references[].purpose` — for example "the lead's face and hair identity", "coworker's face and build identity", "opened main compartment as state reference", "reference clip 2.0-4.0s for hand-over motion" — never a bare label such as "content reference". `role` selects the technical input mode and `purpose` describes the visual use; both must be correct and consistent with the prompt text.

When more than one recurring person or object appears, bind every input to its named subject and keep the binding stable across segments. An unlabelled reference cannot carry a character's identity, and adding more unlabelled references does not fix a missing assignment.

For a storyboard-driven task, use [control routes](references/control.md) and [video prompt writing](references/prompt-writing.md) only for missing input or wording rules; reuse loaded text. Keep one workflow across the shot set; do not silently replace boards with single representative images.

## Model inputs and parameters

Use the selected model's current injected rules and capability schema; query model guidance only when it is missing or has changed. Distinguish ordinary appearance or composition references from actual first or last frame controls. Use compatible input geometry and support. Full-reference boards describe ordered states; only actual first/last-frame controls bind an endpoint state. Keep user-locked aspect, resolution and duration in supported parameters, and state an unsupported requirement explicitly instead of silently changing it.

## Output rules

Preserve the requested identity, structure, event order, continuous/cut intent, support and contact. Do not add repeated preparation, unauthorized cuts, grid layouts, morphing, teleportation or an already completed action.

Use real generated assets and actual task states. Do not claim playback, listening or visual approval from completion status. Repair only requested or concretely identified defects within existing authorization. This Skill does not require a review pass, evidence ledger or repeated material inspection.
