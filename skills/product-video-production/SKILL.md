---
name: product-video-production
description: Produce video in Mstudio - compile a shot design into the final model prompt, choose inputs and a control route, generate and repair clips, and judge the actual result from pixels.
---

# Video production

Turn an approved design into a final, usable video prompt and generated media, then judge the result from actual pixels. Use the supplied design; do not redesign the story or widen the task. Read [CORE.md](CORE.md) together with this file; every step below assumes its rules. When this text is injected without tools, as in the generation panel, only steps 2 to 4 apply: deliver the final prompt, the mode and the reference bindings.

## Workflow

1. **Recover the design and the locks.** Read the shot text, duration, dialogue, references, the original product evidence and the user's locked aspect, resolution, duration and language. Essential events are locked; provisional implementation choices are yours to change.
2. **Fix the units.** A script section describes a dramatic change; an editorial shot is one viewpoint; a generation group is one model call and may occupy one Mstudio shot record. Group compatible short shots to meet the model's minimum clip length, keeping every internal cut explicit, and never stretch an action to fill the minimum. When several timed events share a limited duration, read [Rhythm](../creative-ad-director/references/rhythm.md) before writing.
3. **Choose the route and the inputs.** Pick a control route from [Control routes](references/control.md). Set the video `mode` and each input's `role` from the selected model's injected guidance, and give a mode only its own roles: `single` takes one `first-frame`; `ends` takes a `first-frame` and a `last-frame`; `multi` takes `reference` images, and any explicit reference images mean `multi`; `mixed` takes image and `video-reference` inputs. Verify that an input's geometry and starting support can produce the intended event; calling a conflicting middle state "not the first frame" does not remove the conflict.
4. **Compile the prompt** with [Video prompt writing](references/prompt-writing.md), which includes a worked example, and the selected model's rules. Real people on camera mean reading [Naturalistic performance](../creative-ad-director/references/naturalistic-performance.md) first; believable skin and light mean [Photographic appearance](../creative-ad-director/references/photographic-appearance.md).
5. **Save or generate within the request.** A prompt-only request ends with `mstudio_set_video_prompt(id, prompt)`. An explicit generation request submits `mstudio_generate_video` with the complete prompt, the actual asset references and the model's supported parameters; an unsupported requirement is reported, not silently changed.
6. **Track the result state.** Submitted, generated, imported and visually inspected are four different states; a `COMPLETED` job proves the model finished, not that the clip is usable. Wait with `mstudio_await_generation` only when the next authorised step depends on the result, then read frames with `mstudio_read_image` and judge per [Review and delivery](references/review.md), recording evidence per [Inspection evidence](references/evidence.md).
7. **Repair in order.** When a result is unusable: the smallest change to an input or reference assignment; then the smallest split or cut that preserves the expression; then a different route, per [Controlled generation](references/generation.md). Never answer a diagnosed physical failure with longer negative lists, louder sound or global retiming.
8. **Hand off.** Give editing the shot or group ID, the actual asset IDs, the usable source ranges with observed events, and the acceptance points still open.

## Read when

| The task has to decide | Read |
|---|---|
| How to compile the current segment into one final prompt, with a worked example | [Video prompt writing](references/prompt-writing.md) |
| Which input mode and control route can express the planned action | [Control routes](references/control.md) |
| Reference selection, segmentation and recovering a specific defect | [Controlled generation](references/generation.md) |
| **Several timed events in a limited duration: what each segment earns and how cuts and sound line up** | **[Rhythm](../creative-ad-director/references/rhythm.md)** |
| **How a person's reaction starts and layers on camera** | **[Naturalistic performance](../creative-ad-director/references/naturalistic-performance.md)** |
| Weight, force, contact and material response inside the action | [Animation principles](../creative-ad-director/references/animation-principles.md) |
| What a finished clip is checked against and how to deliver | [Inspection evidence](references/evidence.md), [Review and delivery](references/review.md) |
| Selecting usable source intervals and assembling the timeline | [Editing](../video-editing/references/editing.md) |

## Physical causality is not a full operating sequence

An action must be causally believable: starting support, path, contact and release stay consistent, and nothing crosses a closed surface. That is a requirement on the action, not an instruction to show every step. Decide which part the audience needs, normally the decisive contact or change and its result, and omit preparation, repeated reaches, redundant handling and idle endings while keeping the supporting geometry consistent in what is shown. When the model cannot render the whole chain reliably, obtain the smallest supported segment that carries the needed event instead of lengthening the prompt.

## Bind every input to a concrete purpose

Each supplied image or video is an edit target, a first frame, a last frame, an identity reference, product geometry and material evidence, a composition or state reference, or a motion reference. Name that use and the subject it supplies in `references[].purpose`, for example "the lead's face and hair identity", "opened main compartment as state reference" or "reference clip 2.0-4.0s for the hand-over motion"; a bare label such as "content reference" is rejected. `role` is the technical input mode and `purpose` the visual use; both must be correct and agree with the prompt text. With more than one recurring person or object, bind every input to its named subject and keep the binding stable across segments; adding unlabelled references never fixes a missing identity assignment.

## Failure signatures

| You notice | Do instead |
|---|---|
| The prompt narrates every step of an operation, with hand positions and re-grips | Keep the decisive contact or change and its result; omit preparation and handling |
| One prompt asks for a continuous take and also for hard cuts | Choose one, or split the events into separately supported groups |
| Five shots, two people and four lines of dialogue are packed into one call | One generation group per compatible set of events; the edit assembles the film |
| A negative list grows to fix a failed result | Diagnose: a different input, a different reference binding, a split, or a different route |
| A reference is labelled "content", "style" or "mood" | Bind it to a named subject and a visual use, and match the prompt text |
| `COMPLETED` is being reported as usable | Read the frames, then list what still needs playback and listening |
| The minimum clip length is being filled with slow motion or a lingering hold | Group compatible shots with explicit cuts; select the shorter range in the edit |
| A locked aspect, resolution or duration is unsupported by the model | Report the unsupported requirement; do not change it silently |

## Report

Submitted, generated and inspected states with task and asset IDs; what the frames at which times showed against the intended action; what still needs playback or listening; the locked events not yet achieved.

## Boundaries

Story, shot design and continuity belong to [creative direction](../creative-ad-director/SKILL.md); still frames and image prompts to [image production](../image-production/SKILL.md); cutting, grading and transitions to [video editing](../video-editing/SKILL.md). Model syntax, limits and input modes come from the selected model's injected guidance; never invent parameters or switch services.
