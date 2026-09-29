---
name: product-storyboard
description: Turn approved scripts into shot design, action beats and storyboard frames in Mstudio; maintain continuity and verify design against actual frames.
---

# Storyboard direction

The short [CORE.md](CORE.md) is automatically loaded from the database. Read this detailed document only when the current task needs its methods.

Turn the selected script into executable shots. [Scriptwriting](../ad-script/SKILL.md) owns the story; this skill owns viewing order, camera, staging and continuity; the [storyboard artist](../storyboard-art/SKILL.md) executes frames.

## User intent and factual basis

Separate explicit requests, product facts and exploratory assumptions using the current message, project and original attachments. Do not invent consumer needs or performance. Preserve the selected viewing experience and essential events. Assistant-proposed distances, complete operating sequences and timing remain revisable design decisions, not user locks. Audience, language, faces and style follow the current project.

## Scope and reading routes

For a full film, read the complete current script and existing shots. For a local revision, start with the target shot, its script section and relevant original evidence; read neighboring shots only for a concrete continuity dependency. Use [project state](references/project-state.md) to recover decisions and [facts](references/brief-and-facts.md) for factual gaps. Reuse complete, unchanged tool results. Use returned IDs and pagination offsets; a project revision refreshes affected objects, not the entire project. Historical tasks do not extend the current request.

| Decision | Read |
|---|---|
| Shot purpose, viewpoint, staging and cuts | [Shot grammar and handoff](../creative-ad-director/references/cinematography.md), the main design reference |
| Weight, force and performance | [Animation principles](../creative-ad-director/references/animation-principles.md) |
| Timing, pauses, cuts and sound | [Rhythm](../creative-ad-director/references/rhythm.md) |
| State continuity and asset changes | [States and dependencies](references/shots-and-continuity.md) |
| Full-film information and product value | [Viewing structure](references/strategy-and-hook.md); local edits check only affected relationships |
| Explicitly redesigning the opening concept | [Creative criteria](references/creative-bar.md) and [opening development](references/director-hook.md); do not reselect an approved concept |

## Save and hand off

Make the actual camera and action decisions; do not leave essential design to an unavailable human specialist. Use the handoff requirements in shot grammar. Save camera, action, expression, cuts and constraints in text; duration in shot.duration, dialogue in shot.dialogue, and screenplayId/scriptId associations. Reuse existing shot IDs. A shot spanning script sections associates with the primary scriptId and names other sources in text. Do not invent fields or parallel documents, or divide shots by word count or model duration.

Give the artist the necessary frame moments and selected design. Text-only tasks do not generate images. When images are requested, apply [frame inspection](references/preview-images.md). The director fixes design conflicts; the artist fixes execution deviations. Static correctness does not establish motion, speed or sound.

Verify saved changes and return the affected IDs, changes and relevant unknowns. Use the [production handoff](assets/production-brief-template.md) for a production request without copying information already in the project.

## Video generation authorization

Follow the user's current authorization and Mstudio execution mode; do not add another approval workflow. A request for a plan, storyboard or prompt does not authorize video generation. An explicit generation request authorizes tasks for the specified objects, inputs and model. Respect any explicit agreement to review a particular version first; do not ask again for authorization already covering the same scope. Distinguish creation, submission, completion and visual inspection of a task.

[Production](../product-video-production/SKILL.md) executes video. Change the implementation route when necessary without silently deleting core events. Edit only fields allowed by the current role; route work outside that role through the coordinator.

Detailed role scope and execution methods: [role methods](references/role-methods.md). Read only when relevant.
