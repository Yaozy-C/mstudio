---
name: creative-ad-director
description: Realize a selected one-scene screenplay as shots, one continuous shot per generation record. Design viewpoint, staging, performance, photographic appearance and planned timing without turning the scene into a montage.
---

# Creative direction and shot design

Realize the selected **scene** as shots. Determine how camera, staging and performance make its events and product meaning perceptible. This is a professional reference library, not a second production workflow. Loading it grants no extra tools or permissions. Separate user requirements, product facts and creative hypotheses; an invented event is never product evidence.

Own how the scene is photographed: viewpoint, framing, lens/distance relationship, focus, staging, performance, camera movement and cuts. Image and video production own reference binding, visual product fidelity and correction of generated deviations. Give them original asset IDs and the necessary factual boundaries; do not repeat a component inventory in every shot or restrict all views to the product photograph's angle.

For an action shot, distinguish the event from the selected storyboard instant. Specify only contact, support, possession or mechanism states needed to realize the intended event and connect the cuts. Do not invent a product function or mechanism that the event depends on. Production realizes the detailed physical transitions and preserves the actual product through them; return an essential factual conflict without replacing the chosen viewing experience with a safer display shot.

## One shot, one record

The scene is shot as **single continuous shots**, not as a multi-cut generation unit.

- Save **one Mstudio shot record per single continuous shot**: one camera setup, one continuous action, one viewpoint. Do not group adjacent editorial shots into one record.
- If the scene needs a cut, save **separate shot records**; the editor assembles them. A two-person reaction, a cutaway to a clock, a detail insert and a wide shot are separate shots, not one "generation unit".
- If a shot is shorter than the selected model's minimum, generate at the model minimum and trim in editing. Do not merge it with a neighbor to reach five seconds. Do not pad, slow, stretch or add events.
- If an existing record contains multiple editorial shots or an internal cut map, return it to be split before production; do not submit it as one generation.
- A shot record's `duration` is its intended screen duration. The generation duration may be longer only when the model requires a minimum; the extra source is trimming handle, not added story.

This is a deliberate change from generation-unit grouping: grouping short shots into one video task makes the model invent the cuts, the product action and the performance. One shot per task gives the model one action and one camera behavior, and gives editing the cut.

## Preserve the scene contract

The selected card's scene contract is fixed:

- who / where / want / obstacle / product action / turn / cost / last line / first image / last image.

The director may choose viewpoint, staging, performance and cuts that realize it, but may not replace the scene with a world, a montage, a second location or a new protagonist. If the script is a world or a montage, or the product action is missing, return the conflict instead of designing around it.

Each shot must be independently understandable for execution. State its relevant subjects, spatial relationships, camera, action, light and current object states in its own text; a reference to another shot cannot replace these decisions. Give film-wide constraints to the coordinator for the existing project creation fields rather than making the first shot their only home.

## Read for the task at hand

Within shot design, choose a reference for the concrete decision at hand. Reuse text already in context and read a linked document only to resolve a concrete question.

| The task has to decide | Read |
|---|---|
| How to make the selected opening and payoff readable without changing the scene | [Visual realization](references/concepts.md) |
| Shot purpose, viewpoint, staging, cuts and the handoff to production | [Shot grammar](references/cinematography.md) |
| Lens/distance, perspective, depth of field and focus as expressive choices | [Lens, distance and focus](references/cinematography.md#lens-distance-and-focus) |
| Turning a chosen scene into shots, camera, action beats and continuity | [Shot design](references/shot-design.md) |
| Whether an earlier design decision, product fact or input is still valid | [Constraints and facts](references/brief-and-facts.md), [project state](references/project-state.md) |
| Which physical instant a frame depicts, and how shot states connect | [State continuity](references/continuity.md) |
| **How a person actually behaves on camera** | **[Naturalistic performance](references/naturalistic-performance.md)** |
| Weight, force, contact and material response | [Animation principles](references/animation-principles.md) |
| **Planned shot durations, visible action phases and intended cut/sound relationships** | **[Rhythm](references/rhythm.md)** |
| **Skin, faces, fabric and light that read as a photograph rather than a polished render** | **[Photographic appearance](references/photographic-appearance.md)** |
| **Discovering a photographic look** | **[Photographic exploration](references/photographic-exploration.md)** |
| Reusing the relationship of a real reference shot for a different product | [Transferring a reference](references/shot-references.md) |
| Handing a shot to a generation task | [Production handoff](assets/production-brief-template.md) |

Use performance for human behavior, rhythm for timed beats and photographic appearance for skin, light and material when those rules are needed; reuse loaded text.

## Visual decisions before prompt wording

For a new photographic look or an appearance complaint, read [photographic exploration](references/photographic-exploration.md). Discover a concrete visual relationship before settling on mood labels or a preferred composition formula. State the selected viewpoint, scale, moment, light/material and color relationships in the handoff. Reuse an approved look; do not reopen it for every shot. Realism alone is not the criterion, and "cinematic", "premium" or a resolution label is not a lighting or composition decision.

For an effect, state why it belongs in this shot, its target and protected regions, trigger, visible progression, ending state, camera/occlusion relationship and intended editorial exit. A preset name or pasted effect description is an idea to instantiate, not an executable recipe.

Carry these decisions in existing permitted shot text or ordinary handoff text. They are not new database fields, tool arguments or extra agent permissions. Include which reference properties to borrow and which not to borrow. Production chooses a supported route and compiles the decisions into the actual submitted prompt and input bindings; notes alone do not reach the generator.

## Boundaries

Concept and scene selection belong to creative concepts. Scene words, action and timing belong to ad-script. Executing frames and writing image prompts belong to image production. Converting one shot into a final video prompt, choosing inputs and generating belong to production. Cutting, grading and transitions belong to editing. This Skill decides staging, camera, performance intent and continuity within the selected scene; it does not reopen scene selection during shot design.

Distinguish written design, actual frames, motion and sound. A saved design or a submitted task is not a verified result. Model syntax, limits and input modes come from the selected model's injected guidance; never invent parameters or switch services by default.
