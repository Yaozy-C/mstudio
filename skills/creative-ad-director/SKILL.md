---
name: creative-ad-director
description: Realize a selected screenplay through shot design, camera, staging, performance, photographic appearance and planned timing. Use for storyboards and shot revisions, not concept selection or final editing.
---

# Creative direction and shot design

Realize the selected screenplay as shots: determine how camera, staging and performance make its events and product meaning perceptible. This is a professional reference library, not a second production workflow. Loading it grants no extra tools or permissions. Separate user requirements, product facts and creative hypotheses; an invented event is never product evidence.

Own how the passage is photographed: viewpoint, framing, lens/distance relationship, focus, staging, performance, camera movement and cuts. Image and video production own reference binding, visual product fidelity and correction of generated deviations. Give them original asset IDs and the necessary factual boundaries; do not repeat a component inventory in every shot or restrict all views to the product photograph's angle. A changed projection or an occluded part is not a change of product identity.

For an action shot, distinguish the event from the selected storyboard instant. Specify only contact, support, possession or mechanism states needed to realize the intended event and connect the cuts. Do not invent a product function or mechanism that the event depends on. Production realizes the detailed physical transitions and preserves the actual product through them; return an essential factual conflict without replacing the chosen viewing experience with a safer display shot.

## Mstudio shot records and timing

When creating a production shot plan or regrouping one, save one Mstudio shot record per generation unit of at least 5 seconds. Group adjacent, narratively compatible editorial shots inside that record; do not create one short record for each script paragraph and leave grouping to video production. Set `shot.duration` to the whole unit span. In its title identify the unit and its span; in its text preserve each internal shot's complete design, exact relative time range, viewpoint and intended cut, plus its intended film interval. Internal shots can be shorter than one second. Their timing and the final-film runtime stay unchanged.

Each saved shot must be independently understandable for execution. State its relevant subjects, spatial relationships, camera, action, light and current object states in its own text; a reference to another shot cannot replace these decisions. Give film-wide constraints to the coordinator for the existing project creation fields rather than making the first shot their only home. See [shot-record saving](references/shot-design.md#save-and-hand-off) for scope and continuity references.

For example, compatible 2-second and 3-second shots become one 5-second record containing 0–2s and 2–5s internal shots, with the designed cut at 2s. Giving each original record a separate 5-second video task does not implement this grouping. Combine a short final remainder with a compatible neighbor; if the entire passage is shorter than 5 seconds or must remain isolated, distinguish at least 5 seconds of source including editing handles from the unchanged intended film interval. See [shot-record saving](references/shot-design.md#save-and-hand-off) for existing IDs and associations. Production checks model-dependent limits; the director owns saving the grouped design.

## Read for the task at hand

Within shot design, choose a reference for the concrete decision at hand. Reuse text already in context and read a linked document only to resolve a concrete question.

| The task has to decide | Read |
|---|---|
| How to make the selected opening and payoff readable without changing the premise | [Visual realization of the premise](references/concepts.md) |
| Shot purpose, viewpoint, staging, cuts and the handoff to production | [Shot grammar](references/cinematography.md) |
| Lens/distance, perspective, depth of field and focus as expressive choices | [Lens, distance and focus](references/cinematography.md#lens-distance-and-focus) |
| Turning a chosen script into shots, camera, action beats and continuity | [Shot design](references/shot-design.md) |
| Whether an earlier design decision, product fact or input is still valid | [Constraints and facts](references/brief-and-facts.md), [project state](references/project-state.md) |
| Which physical instant a frame depicts, and how shot states connect | [State continuity](references/continuity.md) |
| **How a person actually behaves on camera: what triggers a reaction, when it starts, and how gaze, breath, head and body layer** | **[Naturalistic performance](references/naturalistic-performance.md)** |
| Weight, force, contact and material response | [Animation principles](references/animation-principles.md) |
| **Planned shot durations, visible action phases and intended cut/sound relationships** | **[Rhythm](references/rhythm.md)** |
| **Skin, faces, fabric and light that read as a photograph rather than a polished render** | **[Photographic appearance](references/photographic-appearance.md)** |
| **Discovering a photographic look across people, everyday scenes and spaces; choosing viewpoint, moment and color relationships** | **[Photographic exploration](references/photographic-exploration.md)** |
| Reusing the relationship of a real reference shot for a different product | [Transferring a reference](references/shot-references.md) |
| Handing an approved design to a generation task | [Production handoff](assets/production-brief-template.md) |

Use performance for human behavior, rhythm for timed beats and photographic appearance for skin, light and material when those rules are needed; reuse loaded text.

## Visual decisions before prompt wording

For a new photographic look or an appearance complaint, read [photographic exploration](references/photographic-exploration.md). Discover a concrete visual relationship before settling on mood labels or a preferred composition formula. A single focal priority and several interacting points of interest are both available. State the selected viewpoint, scale, moment, light/material and color relationships in the handoff. Reuse an approved look; do not reopen it for every shot. Realism alone is not the criterion, and "cinematic", "premium" or a resolution label is not a lighting or composition decision.

Use [visual design and effect intent](references/visual-design.md) for the concrete decision. Separate the action board, a full-frame look reference and actual production controls. Preserve a requested board format; do not silently replace it or add paid look frames. Propose an independent keyframe only when it serves a stated control need and is authorized.

For an effect, state why it belongs in this shot, its target and protected regions, trigger, visible progression, ending state, camera/occlusion relationship and intended editorial exit. A preset name or pasted effect description is an idea to instantiate, not an executable recipe. Propose an effect mechanism proactively only when it serves the selected concept; do not invent catalog tools or claim a demonstrated preset exists. Resolve whole-object transformation versus unchanged-silhouette conflicts before production.

Carry these decisions in existing permitted shot text or ordinary handoff text. They are not new database fields, tool arguments or extra agent permissions. Include which reference properties to borrow and which not to borrow. Production chooses a supported route and compiles the decisions into the actual submitted prompt and input bindings; notes alone do not reach the generator.

## Boundaries

Concept development and direction selection belong to creative concepts. Script content and dialogue wording belong to ad-script. Executing frames and writing image prompts belong to image-production. Converting a design into a final video prompt, choosing inputs and generating belong to product-video-production. Cutting, grading and transitions belong to video-editing. This Skill decides staging, camera, performance intent, timing and continuity within the selected viewing proposition; it does not reopen concept selection during shot design.

Distinguish written design, actual frames, motion and sound. A saved design or a submitted task is not a verified result. Model syntax, limits and input modes come from the selected model's injected guidance; never invent parameters or switch services by default.
