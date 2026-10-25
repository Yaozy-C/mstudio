---
name: storyboard-video-production
description: "Produce video from temporal storyboard boards in Mstudio: map panel states to continuous action or planned cuts, use verified reference inputs, generate and repair clips, and preserve the selected action and visual constraints."
---

# Storyboard video production

For a selected shot with a temporal board, use one board per shot as the default storyboard reference workflow. Turn its approved state sequence and shot design into a final video prompt and generated media, and preserve its event order. The board provides planning constraints, not guaranteed frame-level control; explicitly requested exact-control tasks retain their required mode. A shot without a board still uses its saved design and available references; do not invent a board dependency. Use the supplied design; do not redesign the story or quietly widen the task. Loading this Skill grants no extra tools or permissions.

Specialists execute assigned production work and return real outputs and task status. Only the coordinator conducts internal review; the user makes the final selection. Reading references is for execution, not approval. Apply assigned revisions without initiating self-review, acceptance gates or extra generation.

## Generation units

For a sequence, use one video task per generation unit of at least 5 seconds, or the selected model's higher minimum, within its maximum. Each unit may contain adjacent, narratively compatible short editorial shots with their original durations and designed cuts. Read the unit's complete internal time map and preserve all of it in the submitted prompt. A Mstudio shot record represents the unit; its shot.duration is the whole span, while intended film intervals remain explicit in its text.

If a sequence is still saved as separate compatible short shot records, return the grouping mismatch and affected IDs to direction through available coordination so the grouped design is saved before creating video tasks. Raising each short shot's generation duration to 5 seconds does not complete that regrouping. Production adapts model-dependent bounds and input routes; direction owns shot-record writes. For an isolated short passage, a short final remainder or a model that cannot express the planned cuts, use the specific [generation-unit timing rules](references/prompt-writing.md#three-different-units), preserving internal timing and final-film length.

## Understand the selected shot

A shot ID can be a complete production assignment. Read the current shot text and dialogue directly; use linked screenplay passages and nearby shots when they explain its purpose, setup or continuity. Reuse current material already read. Understand the change the audience should perceive and the action, reaction, reveal or payoff that carries it. Coordinator summaries and existing prompts help locate work; the saved approved design and the user's latest explicit changes govern its content. Resolve discrepancies from those sources, returning a concrete creative conflict when it cannot be resolved within production scope.

Use actual images and accessible video frames to understand what the available material supplies. Metadata and an input label do not describe everything visible. Decide which relationships to preserve and which source details are incidental or incompatible. Choose whether optional references belong in the generation request; planning-only use is available when the user has not required that media as an input. Reference-purpose text expresses intent; it does not selectively remove content from the submitted media.

A reference explicitly designated by the user for this generation is a required input. Preserve the actual media and its requested purpose in the generation request; do not omit it, replace it with a written description or sampled stills, or demote it to planning-only use. Concerns about placeholder appearance, style contamination or preferred production methods do not authorize exclusion. If an actual endpoint restriction, reference limit or unavailable source prevents its use, report the specific conflict and leave the affected request unsubmitted rather than sending a reduced input set or silently changing the route. Only an explicit user change can remove or replace that reference.

Choose a supported realization from that understanding, the available evidence and the user's locked choices. Reuse suitable assets, request a concrete missing still through image production, or organize compatible generation units as needed. Preserve the shot's narrative result as well as physical action when adapting execution. No particular frame set, reference bundle or route is required for every shot. For spatial, performance or timing decisions, use [shot execution](references/shot-execution.md).

When continuing or revising work, retain useful observations and the reasons for production choices in existing task descriptions, reference purposes and normal handoffs. Reassess affected choices when the shot or material changes; an old prompt is not a substitute for the intended shot. Use the user's or coordinator's feedback for assigned revisions; return results without judging acceptance or adding extra paid work.

## Read for the task at hand

| The task has to decide | Read |
|---|---|
| How to describe changing screen states, subject/camera motion, focus, light and sound in one final prompt | [Video prompt writing](references/prompt-writing.md) |
| How to make movement, weight and performance read as live action rather than AI motion | [Motion realism](references/motion-realism.md) |
| Which input mode and control route can express the planned action | [Control routes](references/control.md) |
| Reference selection, segmentation and recovering a specific defect | [Controlled generation](references/generation.md) |
| **How long each generated segment earns, what to leave out, and how cuts and sound line up in time** | **[Rhythm](references/shot-execution.md)** |
| **How a person's reaction starts and layers on camera** | **[Naturalistic performance](references/shot-execution.md)** |
| Weight, force, contact and material response inside the action | [Animation principles](references/shot-execution.md) |
| Output constraints and delivery | [Output claims](references/evidence.md), [delivery rules](references/review.md) |

Use rhythm for timed events and performance for human behavior when those rules are needed; reuse loaded text.

## Motion realism

A clip is a performance, not a slideshow. For each generation unit, write one dominant action with a cause and a visible result, the camera's start, end, amplitude and speed, the light and sound that belong to the event, and the performance as attention and timing rather than emotion labels. Keep the face and hands readable; reduce head rotation, occlusion and simultaneous actions that cause morphing or identity drift. Read [motion realism](references/motion-realism.md) when the output looks floaty, drifts, mutates, clones background people or loses weight.

The image or board already supplies appearance; the prompt should mostly supply motion, camera, performance and sound. Do not re-describe the entire still or ask one short clip to prove several benefits at once.

Required speech belongs to the video deliverable. When the selected model supports native audio, include the exact dialogue or offscreen voice-over, language, delivery and action timing in the generation prompt. No visible speaker or standalone TTS tool is required for an offscreen voice. Keeping subtitles out of generated pixels does not mean keeping speech out of generated audio. Use separate audio production only for a selected route or a concrete endpoint limitation; preserve the speech requirement in that handoff.

Final source selection, trimming and timeline assembly belong to editing. Hand off actual generated assets and any known missing events through existing project records; do not perform timeline work under production permissions.

Use [core](CORE.md) when its short rules are needed; reuse loaded text.

## Visual effects are shot plans, not pasted presets

For an effect name, effect-card description, material transformation, light trail, portal, transition or unusual camera move, use [effect construction](references/effects.md) when tools permit. The essential rule also applies without a method-read loop: determine intent, target, protected region, trigger, visible progression, final state, camera/occlusion relation, required inputs and failure conditions before compiling the prompt. Apply these decisions to the actual submitted prompt and supported reference bindings, not only a saved note. Do not invent effect tools, schema fields, masks, frames or service capabilities.

Choose the route from the selected endpoint's actual capabilities and supplied inputs: a controlled opening may use a first frame; a specified ending may require supported first/last controls; an existing clip with retained action requires a real editing route. Exact labels may require an available retained layer/composite workflow; text cannot promise pixel preservation. A transition needs its outgoing and incoming material. Missing inputs, unsupported routes or contradictory preservation requirements block that paid call; return the specific gap through the existing handoff without silently changing model, format or story.

In a board workflow, preserve the approved board mapping and input mode. Never send the whole board as an exact first/last frame or add separate controls without authorization. A standalone effect/control request retains its explicit input mode. Default to one dominant event and motivated camera motion, with only necessary secondary responses; do not stack unrelated effects or omit causal phases to shorten the prompt. Timed phases express intent, not guaranteed timestamps.

An explicitly selected transformation may change only the declared scope. Whole-object melting cannot also promise an unchanged silhouette; scope that conflict with the user/coordinator rather than hiding it behind "preserve identity". Keep technical input parameters separate from visual prose and follow the selected model's rules.

## Complete physical action

Turn the selected action into a complete physical causal chain in the actual generation prompt. Include necessary intermediate states whenever contact, support, grip, attachment constraints or possession changes; action names and endpoints alone are insufficient. Production owns this realization within the selected design. Use [physical action rules](references/shot-execution.md#contact-and-motion) for the relevant transitions.

Separate editorial omission from motion generation. Within a continuous shot, do not omit a necessary transition to shorten the prompt or meet a beat. Omit only repetitions or idle time that do not change the physical state. A designed cut may omit elapsed handling, but its outgoing and incoming states must remain physically compatible. If the required chain does not fit the supported duration or route, resolve that execution constraint without silently dropping phases, adding cuts or changing the selected action.

## Bind every input to a concrete purpose

Each supplied image or video is one of: edit target, first frame, last frame, identity reference, product geometry and material evidence, composition or state reference, or motion reference. Name that use plus the subject it supplies in `references[].purpose` — for example "selected character: face and hair identity", "selected product: visible geometry", "selected object: action-state reference", "selected source interval: motion reference" — never a bare label such as "content reference". `role` selects the technical input mode and `purpose` describes the visual use; both must be correct and consistent with the prompt text.

When more than one recurring person or object appears, bind every input to its named subject and keep the binding stable across segments. An unlabelled reference cannot carry a character's identity, and adding more unlabelled references does not fix a missing assignment.

For a storyboard-driven task, use [control routes](references/control.md) and [video prompt writing](references/prompt-writing.md) only for missing input or wording rules; reuse loaded text. Keep one workflow across the shot set; do not silently replace boards with single representative images.

## Model inputs and parameters

Use the selected model's current injected rules and capability schema; query model guidance only when it is missing or has changed. Distinguish ordinary appearance or composition references from actual first or last frame controls. Use compatible input geometry and support. Full-reference boards describe ordered states; only actual first/last-frame controls bind an endpoint state. Keep user-locked aspect, resolution and duration in supported parameters, and state an unsupported requirement explicitly instead of silently changing it.

## Output rules

Preserve the requested identity, structure, event order, continuous/cut intent, support and contact. Do not add repeated preparation, unauthorized cuts, grid layouts, unrequested morphing or teleportation, or an already completed action. A selected effect may transform only its declared target; it does not authorize unrelated identity or structure drift.

Use real generated assets and actual task states. Base conclusions about the requested result on relevant accessible media, concentrating on the uncertainty or defect that matters to this shot. Do not claim playback, listening or visual approval from completion status. Repair only requested or concretely identified defects within existing authorization and budget. Do not impose repeated review or a fixed approval sequence on a simple execution task. Report what was checked, what remains unverified and the real task state. Completion or sampled frames alone cannot establish full motion, sound or aesthetic success; inspecting output does not authorize an extra paid call.
