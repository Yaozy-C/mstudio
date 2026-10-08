---
name: creative-ad-director
description: Direction methods for Mstudio - the viewing proposition, shot design, staging, on-camera performance, photographic appearance, rhythm, continuity, and transferring a real reference shot to a product.
---

# Creative direction and shot design

Direct the viewing experience: what the audience notices, understands, expects and remembers, and how the product takes part. This Skill decides the concept's soundness, viewing order, camera, staging, performance intent, timing and continuity. Read [CORE.md](CORE.md) together with this file; every step below assumes its rules.

## Workflow

1. **Recover the state.** Read the current creation, screenplay, shots, assets and timeline with `mstudio_inspect` only where the supplied snapshot is missing something. Decide whether this is a new commission, the continuation of a named version, or a local edit; the same product does not imply continuation. Separate user locks, product facts and your own earlier proposals. Details in [Project state](references/project-state.md) and [Constraints and facts](references/brief-and-facts.md).
2. **Check the concept before staging it.** The script or brief must name a real opening, what makes the next moment worth watching, the payoff and the product's contribution. If it does not, repair the premise with [Concept development](references/concepts.md) before designing shots; staging cannot rescue a weak promise.
3. **Design the shots.** Follow [Shot design](references/shot-design.md) for scope, the record format with its worked example, and saving. Decide each shot's purpose and viewpoint with [Shot grammar](references/cinematography.md), the beats and material response with [Animation principles](references/animation-principles.md), the time each beat earns with [Rhythm](references/rhythm.md), and the states a cut must preserve with [Continuity](references/continuity.md).
4. **Save and verify.** Write the design into the existing shot records, then verify the receipt. Return the affected IDs, what changed, and the unresolved unknowns.
5. **Hand off.** Give production the selected moments, references, locks and acceptance points through the [production handoff](assets/production-brief-template.md). Make the camera and action decisions yourself; there is no human specialist downstream to finish them.

## Read when

The task itself is the trigger. Do not wait for a complaint about the output.

| The task has to decide | Read |
|---|---|
| Whether a concept, opening or payoff is worth making, or how to repair a flat one | [Concept development](references/concepts.md) |
| Turning a chosen script into shot records, and how to save them | [Shot design](references/shot-design.md) |
| Shot purpose, viewpoint, staging, cuts and the handoff format | [Shot grammar](references/cinematography.md) |
| **Real people on camera: what triggers a reaction, when it starts, how gaze, breath, head and body layer** | **[Naturalistic performance](references/naturalistic-performance.md)** |
| **Several timed events inside a limited duration: what each beat earns, what to omit, how cuts and sound shape pace** | **[Rhythm](references/rhythm.md)** |
| **Believable skin, fabric and light that read as a photograph** | **[Photographic appearance](references/photographic-appearance.md)** |
| Weight, force, contact and material response | [Animation principles](references/animation-principles.md) |
| How shots, frames and generated clips depend on each other's state | [Continuity](references/continuity.md) |
| Whether an earlier decision, product fact or input is still valid | [Constraints and facts](references/brief-and-facts.md), [Project state](references/project-state.md) |
| Reusing the mechanism of a real reference shot for a different product | [Transferring a reference](references/shot-references.md) |
| Handing an approved design to generation | [Production handoff](assets/production-brief-template.md) |

## Failure signatures

| You notice | Do instead |
|---|---|
| A shot's purpose reads "cinematic", "premium" or "dynamic" | Replace it with what the viewer learns or feels and the visible cause |
| The action needs the product to open, bend or hold in a way the original photos do not show | Inspect the originals; restage around verified edges, openings and attachments |
| You divided the runtime into equal slots first | List the distinct information first, then allocate reading time |
| The model's minimum clip length makes you stretch an action or slow it down | Group compatible shots into one generation with explicit cuts; keep the beat short and select the range in the edit |
| A person is written as "looks and smiles" | Write the trigger, then gaze or breath, then head and body, and what the action is for |
| You want to create a registry, a previs document or a parallel plan | Put it in the shot text and `references[].purpose`; the project has no other place |
| You are about to call the design verified | Say what stills verified; motion, speed and sound stay unchecked until played and heard |

## Report

The shot IDs touched and what changed in each; the locks that production must keep and the choices it may vary; the frame moments requested; the unknowns that need original evidence or playback.

## Boundaries

Script content and dialogue wording belong to [ad-script](../ad-script/SKILL.md). Executing frames and image prompts belong to [image-production](../image-production/SKILL.md). Compiling a video prompt, choosing inputs and generating belong to [product-video-production](../product-video-production/SKILL.md). Cutting, grading and transitions belong to [video-editing](../video-editing/SKILL.md). Model syntax, limits and input modes come from the selected model's injected guidance; never invent parameters or switch services.
