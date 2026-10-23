# Visual quality and production control frames

Apply these methods within the selected image deliverable and existing authorization.

## One art direction, different deliverables

Read the selected project look and relevant approved images before choosing appearance. Carry the same medium, degree of realism or stylization, period, palette relationships, material language and finish from reference assets into storyboard imagery. Use the existing project notes, asset records and reference purposes to retain those decisions; do not create a separate style registry or reopen a settled look for each shot.

Adapt presentation to purpose. An asset image should expose useful identity and construction with readable pose, scale and light; a storyboard panel should express the designed attention order, camera, action state and motivated scene light. A neutral background does not require generic commercial photography, glamour retouching or a different rendering style. Shared aesthetics do not require identical exposure, background, pose or lighting: for example, a clear daytime character reference can support a blue-hour scene while preserving the same face, clothing, material character and photographic texture. Do not erase intentional story-driven light or state changes to match the reference's presentation.

## Resolve appearance before execution

Use the approved look. If it is missing, make a compact, clearly stated execution hypothesis rather than automatically generating alternatives: attention order, viewpoint and scale, light/value separation, material response, palette relationship and exclusions. Return concept or shot-design conflicts to the coordinator. Do not reopen an approved direction just because a sample has a local defect.

Replace generic adjectives with visible relationships. Metal can be described by the shape and roll-off of its reflections, transparent material by edge thickness and transmission, and cloth by fold scale and directional response. Do not give all surfaces the same glossy treatment. A cleaner background cannot compensate for a silhouette that merges into it.

## Photographic direction and the AI look

For live-action or photorealistic work, use [photographic direction](photographic-direction.md) and, for people, [skin realism](skin-realism.md). A look is not a list of style adjectives; it is a set of visible decisions about capture, lens, camera height, composition, motivated light, color and surface response. Decide those before adding subject detail.

When a result looks synthetic, diagnose the cause before adding more detail:

| Symptom | Likely prompt cause | Positive repair |
|---|---|---|
| Waxy or uniform face | "smooth", "flawless", "beauty", "retouched" | Regional color, scale-appropriate pores, fine lines, a light-driven sheen, shadow-side detail |
| Catalogue pose | Full-body, frontal, neutral backdrop | An unposed moment, weight shift, occupied hands, a real room with depth |
| Flat lighting | "soft even light", "studio light" | A named source, direction, ratio and shadow-side bounce |
| Plastic fabric | "clean", "premium", "crisp" | Folds from the pose, seams, weave, weight and slight wear |
| Orange or teal wash | "cinematic", "warm sunset" without color relationships | Separate sky, sea, skin and neutral values; one warm source, not a global grade |
| Over-sharpened detail | "8K", "HDR", "ultra detailed", "sharp focus" everywhere | Natural optical sharpness, appropriate depth of field, grain and highlight roll-off |
| Generic face | "beautiful", "symmetrical", "perfect features" | Specific bone structure, slight asymmetry, a distinctive feature, an unposed expression |

Do not answer "AI look" with more slogans or a longer negative list. Replace the cause with a visible relationship. If the face is still synthetic after the prompt has a specific structure, light and scale, repair the selected image rather than rerolling the whole scene.

## Bind reference transfer explicitly

For each actual input, name the subject and the exact property to borrow in its purpose. Also state conflicting properties not to inherit in the prompt when needed. A face reference supplies identity, not its pose and lighting. A product reference supplies construction, not every reflection baked into its photograph. Preserve user-locked appearance rather than stripping it away as a generic cleanup.

Resolve reference conflicts before generating. Keep the smallest relevant set and use the endpoint's real technical roles. Never invent references or label a rough sketch as exact product evidence.

## Use the right image artifact

| Artifact | What it establishes | What it does not establish |
|---|---|---|
| Reference asset | Selected identity, wardrobe, construction, material or location baseline | Scene staging, action timing or mandatory scene lighting |
| Temporal board | Ordered scene states and continuity | Full-frame detail or exact video endpoints |
| Look frame | A selected appearance and composition | Motion or elapsed time |
| Start/end control image | An actual endpoint state for a supported route | Guaranteed interpolation, identity or timing |
| Edit target | The image whose selected region/appearance changes | Pixel-perfect preservation by prompt alone |

Preserve the requested board format and cell map. Improve its hierarchy and coherent lighting within that format. Do not silently add a second set of look/control images. A separately authorized hero still or control frame stays full-frame, without a contact sheet or grid border.

A still describes one moment. For an evolving effect, select a meaningful phase: onset, the hardest transition state or the completed appearance. Different states are not interchangeable evidence. Do not ask one unsegmented still to contain an entire motion sequence.

## Compose the actual prompt

Resolve the shot into a photograph, not a production checklist. Write it in this order: capture and camera; the visible instant and blocking; motivated light and color relationship; material and skin at the actual scale; reference roles and only the properties that must be preserved. This is a working order, not a fixed vendor syntax.

Keep continuity rules, part counts, negative constraints and acceptance checks in the task checklist or `references[].purpose`, not in the prompt. A final prompt should read as one coherent image; if it reads as a contract with five unrelated clauses, the model will average them.

Put technical aspect, resolution and supported controls in the actual parameter fields. Do not paste planning labels or fictional API keys into a prompt. A final prompt must be self-contained for the current call; the service will not automatically read the project's shot notes. Do not promise a resolution or control the selected endpoint does not expose.

## Explore only within the authorized budget

A single-generation task stays a single-generation task. Alternatives and repairs require explicit existing authorization.

When exploration is authorized, use distinct design hypotheses with comparable scope. State the intended design difference and return the candidates for coordinator review and user selection; do not score or select them. Do not hide extra sampling cost when comparing old and new methods.

## Return generated work

Return real image IDs, task status and concrete execution limits to the coordinator. Internal review belongs to the coordinator and final selection to the user; the generating specialist does not issue pass/fail judgments, rank candidates or add an acceptance gate.

## Execute assigned local revisions

Use the selected image and the specific change assigned by the user or coordinator. Prefer a supported edit that preserves the specified composition, identity and unaffected styling. An edit instruction does not guarantee pixel preservation; a text-to-image reroll is not a retained-pixel edit. Return the new output without starting another revision yourself.

Keep each change within its assigned scope and budget; do not add a new background, camera move or particle field to change a highlight.

## Effect control example: partial frost on a can

The chosen shot is a locked three-quarter product view. The opening image has sparse condensation. The ending image has thin frost on the lower body and side edges, stopping before the front label. Keep the can opening, silhouette, viewpoint and light arrangement compatible across both images. Build the ending through a supported edit of the selected opening when available, then return both real images to the coordinator.

The completed frame is not evidence that frost grew continuously. If the selected video endpoint lacks last-frame control, do not label the ending reference as a bound endpoint. Preserve exact label pixels only with an available retained-layer/compositing route; text requests do not provide that guarantee.

## Local reference

Use the existing [image prompt method](prompt-writing.md) for physical state and supported prompt syntax. This document adds visual decisions and bounded iteration; it does not override product evidence, requested formats or endpoint limitations.
