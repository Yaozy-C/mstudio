---
name: creative-ad-director
description: Shared creative, cinematography, animation and prompt-conversion rules for Mstudio advertising roles.
---

# Creative and shot-design rules

Prompt writing belongs to agents assigned the image-prompt or video-prompt Skill. Direction defines the intended staging and performance; the assigned prompt author converts that design.


This is a professional reference library, not a second production workflow. Read the rules required by the current role and task. Loading this skill does not expand scope or permissions. Separate product facts, user objectives and creative hypotheses; invented events must not masquerade as performance evidence.

## Read by responsibility

| Responsibility | Main reference | Conditional reference |
|---|---|---|
| Writer: new concept or event revision | [Concepts](references/concepts.md) | [Opening criteria](../product-storyboard/references/creative-bar.md), [development practice](../product-storyboard/references/director-hook.md) |
| Director: shot design or revision | [Shot grammar](references/cinematography.md) | [Animation principles](references/animation-principles.md), [rhythm](references/rhythm.md), [state dependencies](../product-storyboard/references/shots-and-continuity.md) |
| Artist: frames and image prompts | [Image prompts](references/image-prompt-writing.md) | Staging and handoff in shot grammar; [frame inspection](../product-storyboard/references/preview-images.md) |
| Media producer: implementation | [Control routes](references/shot-control.md), [video prompt conversion](references/video-prompt-writing.md) | Use the selected model’s injected guidance |
| Editor: pacing changes | [Rhythm](references/rhythm.md) | [Editing execution](../product-video-production/references/motion-and-editing.md) |

## Execute in Mstudio

[Scriptwriting](../ad-script/SKILL.md) owns script fields, [storyboard direction](../product-storyboard/SKILL.md) owns shot fields and design handoff, [the artist](../storyboard-art/SKILL.md) owns images, and [production](../product-video-production/SKILL.md) owns model tasks and footage selection.

Use mstudio_models for available model capabilities. Do not invent parameters or switch services by default. Distinguish written design, actual frames, motion and sound checks; submission success does not establish output quality. Generation follows [current authorization](../product-storyboard/SKILL.md#video-generation-authorization).
