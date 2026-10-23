# Storyboard direction

Skill bodies are loaded on demand. Use [CORE.md](../CORE.md) for the short design rules when needed; reuse rules already in context. Read a linked reference to resolve a specific design question, not as a mandatory checklist. Once the relevant script, constraints and product facts are sufficient, save the requested shot design. Retrieve additional views only for a concrete missing input used by the requested shot.

Turn the selected script into executable shots. The screenwriter owns the story; this skill owns viewing order, camera, staging, performance and intended continuity. Image and video production own product fidelity in the generated media and repair execution deviations. An action phase, a storyboard panel and an editorial shot are different units: choose cuts and viewpoints from the viewing experience rather than dividing an operation into a compulsory sequence of steps.

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

For a new Mstudio production plan, organize adjacent, narratively compatible editorial shots into generation units of at least 5 seconds. Save one Mstudio shot record per unit, with shot.duration at least 5 seconds; keep every internal shot's description, viewpoint, duration and cut in its text, using a relative time map. Internal shots may be shorter than one second: for example 0.00-0.25s, 0.25-0.60s and 0.60-5.00s remain 0.25s, 0.35s and 4.4s shots within one 5-second unit. Do not round them up, stretch actions or replace their descriptions with a summary. Preserve continuous takes where designed; grouping does not require adding cuts. Keep the unit span in shot.duration and label intended film timing separately. Production checks the selected model's bounds. When regrouping existing records, reuse an anchor ID, retain every source ID and its complete description/timing in the unit text, and preserve screenplay associations. Existing frame/task/result links require an explicit association handoff to the permitted role; do not silently delete linked records or create duplicate active units. If existing links prevent completing the requested regrouping with current tools, report that concrete remainder. Do not claim the grouped plan is saved while the active plan still consists of the original short records. Combine a short final remainder with a compatible neighbor. Only when the complete passage is shorter than 5 seconds or must remain isolated, include clearly identified editing handles in the unit span while preserving its intended film interval; never pad each compatible short shot separately.

Make the actual camera and action decisions; do not leave essential design to an unavailable human specialist. Use the handoff requirements in shot grammar. Save camera, action, expression, cuts and constraints in text; duration in shot.duration, dialogue in shot.dialogue, and screenplayId/scriptId associations. When an existing record represents a production generation group, shot.duration retains that group's generation span rather than its selected screen time. Preserve its internal editorial shots and cut map; production owns changing model-dependent grouping and generation duration. For new design records, follow the actual schema and label the intended film intervals separately from any production-supplied generation timing. Include each retained event, its viewpoint, intended film range, and omittable preparation; production adds actual source in/out after generation. The sum of generation-group durations need not equal film duration. Do not invent source timestamps before footage exists. Reuse existing shot IDs. A shot spanning script sections associates with the primary scriptId and names other sources in text. Do not invent fields or parallel documents, or divide shots by word count or model duration.

Save each shot as an independently executable description. Include the relevant subject identities or selected reference IDs, location and fixed landmarks, participant positions and body/eye directions, camera position and viewing direction, framing/focus, action phases, scene light and current prop states. Match detail to the shot; do not copy the whole film brief, every asset specification or unrelated production instructions into every record. A changed camera view changes projection, not the subjects' world positions or orientations unless the action calls for that change.

Cross-shot IDs can explain an incoming/outgoing match or continuity source, but wording such as "same lighting/space as shot 1" cannot replace this shot's required description. Read the necessary source once, resolve its applicable decisions into the saved target text, and preserve its distinct state and viewpoint. Give film-wide spatial, appearance and continuity decisions to the coordinator for the existing creation.essential/creation.preserve fields; do not store them only in the first shot or invent inherited shot fields.

Give the artist the selected viewpoint, lens/distance relationship, focus state, scene look and necessary frame moments. Distinguish a representative instant from an intended opening/ending state using [state continuity](continuity.md). Preserve the user's storyboard/full-reference format; a board's cell count does not dictate editorial shot count or require separate start frames. Text-only tasks do not generate images. When images are requested, image production realizes them and corrects visual deviations; direction fixes design conflicts. Static correctness does not establish motion, speed or sound.

Use the authoritative saved values and return the affected IDs, changes and concrete missing inputs. Use the [production handoff](../assets/production-brief-template.md) for a production request without copying information already in the project.

## Production handoff

Video production implements video. Preserve essential events while revising provisional implementation choices. Give asset preparation only missing shared identity needs; give the artist the selected moments, landmarks and actual references for scene frames.

The director's shot count is not an image-layout or asset-count instruction. Hand off the shot/state and timing map; image production maps it into the selected workflow's board contract, preserving an explicit user format. Do not infer a new panel count or require one independent image per editorial shot. Keep full-reference boards as sequence references and leave concrete input binding to production.
