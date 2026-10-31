---
name: product-video-production
description: "Produce video in Mstudio: compile a shot design into a final model prompt, choose inputs and control routes, generate and repair clips, and deliver the requested media."
---

# Video production

Turn an approved design into a final, usable video prompt and generated media, and deliver the requested media. Use the supplied design; do not redesign the story or quietly widen the task. Loading this Skill grants no extra tools or permissions.

Specialists execute assigned production work and return real outputs and task status. Only the coordinator conducts internal review; the user makes the final selection. Reading references is for execution, not approval. Apply assigned revisions without initiating self-review, acceptance gates or extra generation.

## One shot, one task

Generate **one video task per single continuous shot**. A shot is one camera setup and one continuous action; its `shot.duration` is the intended screen duration.

- Do not group adjacent editorial shots into one task. A two-person reaction, a cutaway to a clock, a detail insert and a wide shot are separate shots, not one generation unit.
- If a shot is shorter than the model minimum, generate at the model minimum and trim in editing. Do not pad, slow, stretch or merge it with a neighbor.
- If a shot record contains multiple editorial cuts or an internal time map, return it to direction to be split before generating. Do not submit a montage as one prompt.
- If a model offers multi-shot generation, do not use it for narrative clarity; use it only when the user explicitly asks for a single-generation montage and accepts the model inventing the cuts.
- The generation prompt describes **one action and one camera behavior**. Do not write internal cut points, a scene beat sheet or a montage into a single-shot prompt.

This replaces the former five-second generation-unit rule. Grouping short shots into one task makes the model invent cuts, performances and product actions; editing owns the cut.

## Understand the selected shot

A shot ID can be a complete production assignment. Read the current shot text and dialogue directly; use linked screenplay passages and nearby shots when they explain its purpose, setup or continuity. Reuse current material already read. Understand the change the audience should perceive and the action, reaction, reveal or payoff that carries it. Coordinator summaries and existing prompts help locate work; the saved approved design and the user's latest explicit changes govern its content. Resolve discrepancies from those sources, returning a concrete creative conflict when it cannot be resolved within production scope.

Use actual images and accessible video frames to understand what the available material supplies. Metadata and an input label do not describe everything visible. Decide which relationships to preserve, which source details are incidental or incompatible, and whether a source belongs in the generation request at all. A useful planning reference may guide your reasoning without supplying finished appearance. Reference-purpose text expresses intent; it does not selectively remove content from the submitted media.

Choose a supported realization from that understanding, the available evidence and the user's locked choices. Reuse suitable assets, request a concrete missing still through image production, or split the scene into single shots as needed. Preserve the shot's narrative result as well as physical action when adapting execution. No particular frame set, reference bundle or route is required for every shot. For spatial, performance or timing decisions, use [shot execution](references/shot-execution.md).

When continuing or revising work, retain useful observations and the reasons for production choices in existing task descriptions, reference purposes and normal handoffs. Reassess affected choices when the shot or material changes; an old prompt is not a substitute for the intended shot. Use the user's or coordinator's feedback for assigned revisions; return results without judging acceptance or adding extra paid work.

## Read for the task at hand

| The task has to decide | Read |
|---|---|
| How to describe changing screen states, subject/camera motion, focus, light and sound in one final prompt | [Video prompt writing](references/prompt-writing.md) |
| How to make a product, hand interaction and camera move read as live action rather than AI motion | [Product motion realism](references/motion-realism.md) |
| Which input mode and control route can express the planned action | [Control routes](references/control.md) |
| Reference selection, segmentation and recovering a specific defect | [Controlled generation](references/generation.md) |
| **How long each generated segment earns, what to leave out, and how cuts and sound line up in time** | **[Rhythm](references/shot-execution.md)** |
| **How a person's reaction starts and layers on camera** | **[Naturalistic performance](references/shot-execution.md)** |
| Weight, force, contact and material response inside the action | [Animation principles](references/shot-execution.md) |
| Output constraints and delivery | [Output claims](references/evidence.md), [delivery rules](references/review.md) |

Use rhythm for timed events and performance for human behavior when those rules are needed; reuse loaded text.

## Product motion realism

Realize the selected creative event through a clear action sequence. Preserve its physical causes, decisive change, intended result, performance intensity and timing, coordinating camera and subject motion. Keep rigid parts rigid, let soft goods deform under load, show support and contact, and keep the label plane readable when it matters. Read [product motion realism](references/motion-realism.md) when the product drifts, a hand mutates, the object floats or the clip reads as a synthetic turntable.

The supplied product and scene references constrain geometry and appearance; the prompt should mostly supply action, camera, material response and sound. Do not re-describe the whole product sheet or ask one clip to prove every feature.

Required speech belongs to the video deliverable. When the selected model supports native audio, include the exact dialogue or offscreen voice-over, language, delivery and action timing in the generation prompt. No visible speaker or standalone TTS tool is required for an offscreen voice. Keeping subtitles out of generated pixels does not mean keeping speech out of generated audio. Use separate audio production only for a selected route or a concrete endpoint limitation; preserve the speech requirement in that handoff.

Final source selection, trimming and timeline assembly belong to editing. Hand off actual generated assets and any known missing events through existing project records; do not perform timeline work under production permissions.

Use [core](CORE.md) when its short rules are needed; reuse loaded text.

## Visual effects are shot plans, not pasted presets

For an effect name, effect-card description, material transformation, light trail, portal, transition or unusual camera move, use [effect construction](references/effects.md) when tools permit. The essential rule also applies without a method-read loop: determine intent, target, protected region, trigger, visible progression, final state, camera/occlusion relation, required inputs and failure conditions before compiling the prompt. Apply these decisions to the actual submitted prompt and supported reference bindings, not only a saved note. Do not invent effect tools, schema fields, masks, frames or service capabilities.

Choose the route from the selected endpoint's actual capabilities and supplied inputs: a controlled opening may use a first frame; a specified ending may require supported first/last controls; an existing clip with retained action requires a real editing route. Exact labels may require an available retained layer/composite workflow; text cannot promise pixel preservation. A transition needs its outgoing and incoming material. Missing inputs, unsupported routes or contradictory preservation requirements block that paid call; return the specific gap through the existing handoff without silently changing model, format or story.

In a board workflow, preserve the approved board mapping and input mode. Never send the whole board as an exact first/last frame or add separate controls without authorization. A standalone effect/control request retains its explicit input mode. Preserve the selected event hierarchy, camera behavior and secondary responses; keep each change tied to its creative purpose and physical cause. Timed phases express intent, not guaranteed timestamps.

An explicitly selected transformation may change only its declared scope. Resolve conflicts between transformation and preservation requirements through the current workflow before submission. Keep technical input parameters separate from visual prose and follow the selected model's rules.

## Complete physical action

Turn the selected action into a complete physical causal chain in the actual generation prompt. Include necessary intermediate states whenever contact, support, grip, attachment constraints or possession changes; action names and endpoints alone are insufficient. Production owns this realization within the selected design. Use [physical action rules](references/shot-execution.md#contact-and-motion) for the relevant transitions.

Separate editorial omission from motion generation. Within a continuous shot, do not omit a necessary transition to shorten the prompt or meet a beat. Omit only repetitions or idle time that do not change the physical state. A designed cut may omit elapsed handling, but its outgoing and incoming states must remain physically compatible. If the required chain does not fit the supported duration or route, resolve that execution constraint without silently dropping phases, adding cuts or changing the selected action.

## Bind every input to a concrete purpose

Each supplied image or video must have a supported technical role. Name its subject, visual purpose and the properties it supplies in `references[].purpose`. `role` selects the technical input mode; both fields must agree with the prompt and actual input use.

When more than one recurring person or object appears, bind every input to its named subject and keep the binding stable across segments. An unlabelled reference cannot carry a character's identity, and adding more unlabelled references does not fix a missing assignment.

## Model inputs and parameters

Use the selected model's current injected rules and capability schema; query model guidance only when it is missing or has changed. Distinguish ordinary appearance or composition references from actual first or last frame controls. Use compatible input geometry and support. Full-reference boards describe ordered states; only actual first/last-frame controls bind an endpoint state. Keep user-locked aspect, resolution and duration in supported parameters, and state an unsupported requirement explicitly instead of silently changing it.

## Output rules

Preserve the requested identity, structure, event order, continuous/cut intent, support and contact. Do not add repeated preparation, unauthorized cuts, grid layouts, unrequested morphing or teleportation, or an already completed action. A selected effect may transform only its declared target; it does not authorize unrelated identity or structure drift.

Use real generated assets and actual task states. Base conclusions about the requested result on relevant accessible media, concentrating on the uncertainty or defect that matters to this shot. Do not claim playback, listening or visual approval from completion status. Repair only requested or concretely identified defects within existing authorization and budget. Do not impose repeated review or a fixed approval sequence on a simple execution task. Report what was checked, what remains unverified and the real task state. Completion or sampled frames alone cannot establish full motion, sound or aesthetic success; inspecting output does not authorize an extra paid call.
