# Storyboard direction

Skill bodies are loaded on demand. Use [CORE.md](../CORE.md) for the short design rules when needed; reuse rules already in context. Read a linked reference to resolve a specific design question, not as a mandatory checklist. Once the relevant script, constraints and product facts are sufficient, save the requested shot design. Retrieve additional views only for a concrete missing input used by the requested shot.

Turn the selected script into executable shots. The screenwriter owns the story; this skill owns viewing order, camera, staging and continuity; image production executes frames.

## User intent and factual basis

Separate explicit requests, product facts and exploratory assumptions using the current message, project and original attachments. Do not invent consumer needs or performance. Preserve the selected viewing experience and essential events. Assistant-proposed distances, complete operating sequences and timing remain revisable design decisions, not user locks. Audience, language, faces and style follow the current project.

## Scope and reading routes

For a full film, read the complete current script and existing shots. For a local revision, start with the target shot, its script section and relevant original evidence; read neighboring shots only for a concrete continuity dependency. Use [project state](project-state.md) to recover decisions and [facts](brief-and-facts.md) for factual gaps.

| Decision | Read |
|---|---|
| Shot purpose, viewpoint, staging and cuts | [Shot grammar and handoff](cinematography.md), the main design reference |
| Weight, force and performance | [Animation principles](animation-principles.md) |
| Timing, pauses, cuts and sound | [Rhythm](rhythm.md) |
| State continuity and asset changes | [States and dependencies](continuity.md) |
| The selected opening, development or product relationship is unreadable in shots | [Realize the viewing proposition](concepts.md); preserve the premise and return concept-level conflicts to creative planning |

## Save and hand off

Make the actual camera and action decisions; do not leave essential design to an unavailable human specialist. Use the handoff requirements in shot grammar. Save camera, action, expression, cuts and constraints in text; duration in shot.duration, dialogue in shot.dialogue, and screenplayId/scriptId associations. For a generation group, shot.duration is its generation span, not its final selected screen time; retain internal editorial shots and explicit cuts in text as defined in [CORE.md](../CORE.md). Label generation-local timing and final-film timing separately. Include each retained event, its viewpoint, intended film range, and omittable preparation; production adds actual source in/out after generation. The sum of generation-group durations need not equal film duration. Do not invent source timestamps before footage exists. Reuse existing shot IDs. A shot spanning script sections associates with the primary scriptId and names other sources in text. Do not invent fields or parallel documents, or divide shots by word count or model duration.

Give the artist the necessary frame moments and selected design. Text-only tasks do not generate images. When images are requested, hand off the selected moments to image production for generation. The director fixes design conflicts; the artist fixes execution deviations. Static correctness does not establish motion, speed or sound.

Use the authoritative saved values and return the affected IDs, changes and concrete missing inputs. Use the [production handoff](../assets/production-brief-template.md) for a production request without copying information already in the project.

## Production handoff

Video production implements video. Preserve essential events while revising provisional implementation choices. Give asset preparation only missing shared identity needs; give the artist the selected moments, landmarks and actual references for scene frames.
