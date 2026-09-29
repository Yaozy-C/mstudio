---
name: product-video-production
description: Produce media tasks, select usable footage and edit timelines in Mstudio while checking action origin, continuity, rhythm and sound.
---

# Video production, editing and inspection

The short [CORE.md](CORE.md) is automatically loaded from the database. For every image/video prompt, the animation principles and applicable prompt guide are mandatory; other detailed methods are read when needed.

Read the current target and relevant project state. Continue from the [production handoff](../product-storyboard/assets/production-brief-template.md), preserving the chosen concept/model. Do not treat new work as continuation or delete core events for technical convenience. Audience, style, exclusions and authorization come from the current project.

## Local prompt changes

A referenced card identifies the target through referencedTask.key. Use inspect(section=generation, taskKey=theKey), not the full task list. Read associated nodes/fields only for missing evidence and neighboring shots only for a real dependency.

Use update_generation(taskKey,text) for the task; a submitted task stores the revised text for its next generation. Do not also overwrite shot.prompt/framePrompt unless the user requests synchronization. Use update_node for an explicitly requested shot draft. Prompt-only changes do not generate media or prove a visual issue resolved. History does not introduce unrelated tasks; return only actual changes and relevant gaps.

## Generate within scope

Follow [authorization](../product-storyboard/SKILL.md#video-generation-authorization) and the selected execution mode. Use request_generation for an authorized generation, not an external API or automatic purchase.

1. Query mstudio_models and read the selected model's rules. Do not inspect/output secrets or turn one service failure into a claim that all routes fail. Production owns mode, input roles, parameters and final text. Return spatial/design conflicts to the director with evidence; art owns stills and editing owns final audiovisual assembly.
2. Apply [controlled generation](references/generation.md). Distinguish product facts, scene references, state references and genuine frame-control inputs. Calling a middle-state image not the first frame does not make it a compatible start.
3. Read [image](../creative-ad-director/references/image-prompt-writing.md) or [video](../creative-ad-director/references/video-prompt-writing.md) conversion and the full [animation principles](../creative-ad-director/references/animation-principles.md) before writing any image/video prompt. Apply the selected model’s injected guidance separately. Save draft fields within role permissions or update only the selected task. Submitted text is the complete final model prompt. Verify reference order/roles and actual parameters for aspect, resolution and duration. Text descriptions are not parameter settings. Unsupported requirements need a specific limitation and feasible option, not silent defaults.
4. Inspect generation state and actual result assetId. Reuse existing tasks; completion is not visual acceptance. Inspect from source/support through path/contact to destination, not just the successful ending.
5. Mark usable ranges and minimum repair scope using [motion and editing](references/motion-and-editing.md). After two reasoned attempts without progress, change route; stop extra calls at user budget/attempt limits.

## Timeline and speed

Use real footage and existing timeline operations. trimIn/trimOut are source times, start is film time, and used duration is (trimOut-trimIn)/speed. Remove purposeless waits/repetition without hiding required causality.

Read current revision. Inspect action start, decisive event and end using mstudio_read_image(assetId,clipId,time): time is clip-relative seconds with trim/speed applied; without clipId it is source time.

- move_clip(id,start,trackId?): place on the film timeline at project-frame-aligned seconds, preserving source range and speed.
- retime_clip(id,speed,ripple?): absolute speed 0.25–4, preserving source range and start. ripple:true shifts only same-track clips beginning at/after the old end; other audio/subtitles require explicit synchronization.
- slip_clip(id,sourceOffset): offset both source boundaries in source seconds without changing film position or duration.

Moving/retiming rejects same-track overlap by default; allowOverlap:true is only for intentional overlap. Insufficient source handles fail rather than silently clamping or fabricating footage. Reread state/handles on failure. Edit/retime before transitions and grading rechecks.

Use [rhythm](../creative-ad-director/references/rhythm.md) to distinguish maxima, locked totals, local speed and whole-film multipliers. Absolute speed differs from multiplying current speed. Whole-film retiming updates relevant track starts/speeds, subtitle timing and fades; source trim ranges do not scale with film time. Respect actual tool limits.

Inspect saved state. Do not claim unavailable export/playback/audio operations. Preserve accepted material and unrelated tracks. A new version does not inherit old inspection results.

## Inspect and deliver

Use [output review](references/review-and-delivery.md) and [evidence](references/evidence-contract.md). Inspect original parts, essential causality, action source/destination and extra repetition. Motion and audio require actual viewing/listening; metadata and extracted stills cannot establish them.

Return real assets or the saved timeline, distinguishing complete, pending and unchecked. Core motion errors are not merely low-resolution limitations. Use project fields/messages for records when no shell/file tool exists; do not pretend to run external scripts.

## Further reading

- [Source 1](https://www.shotcut.org/howtos/keyboard-shortcuts/)
- [Source 2](https://www.blackmagicdesign.com/products/davinciresolve/training)

Detailed role scope and execution methods: [role methods](references/role-methods.md). Read only when relevant.
