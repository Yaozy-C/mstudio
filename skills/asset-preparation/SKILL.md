---
name: asset-preparation
description: Read storyboards to identify repeated cross-shot elements, audit existing references and prepare only missing white-background shared assets in Mstudio.
---

# Shared asset preparation

The short [CORE.md](CORE.md) is automatically loaded from the database. Read this detailed document only when the current task needs its methods.

Read the actual storyboard before deciding that assets are needed. The default trigger is an element repeated across distinct shots whose identity or appearance must remain consistent. Multiple A/B frames of one shot do not establish cross-shot reuse. Single-shot people, props and incidental background details stay with the storyboard artist; do not create a separate asset for every visible thing. Handle an explicit user request for a particular asset without inventing extra consuming shots.

## Storyboard and inventory audit

Read the shot directory for the requested scope, then actual shot text, references and existing frames using inspect with nodeIds and fields. Read the relevant script only to resolve an identity or state ambiguity. Use nextOffset, nextShotOffset, nextReferenceOffset and nextTextOffset when returned; titles alone are not evidence of what a shot contains. For a local request, read related shots only to resolve a concrete reuse dependency; do not expand the edit scope to the whole film.

Group occurrences of the same element by actual shot IDs and required identity, outfit, product structure or set-piece invariants. Different people or merely similar category objects are not automatically one shared asset. A planned outfit/state change is a deliberate variant, not an inconsistency to erase.

Inventory project images through inspect(section=assets) and relevant tasks through inspect(section=generation). Images and kind=asset media cards are ordinary project materials. Inspect promising image pixels with mstudio_read_image. Match each repeated element to its consumers, needed view/attributes, available asset IDs and concrete gap. Classify the outcome as reusable, inspect-existing, pending, repair-needed or missing. Keep the concise reuse/gap map in the handoff; do not create placeholder asset nodes for every candidate or duplicate the project in memory.

Reuse suitable originals and accepted references directly. Inspect unchecked candidates before deciding to generate replacements. Reuse/wait for relevant pending jobs rather than submitting duplicates. If existing references cover the needs, report no new assets needed and hand off their real IDs. When a gap remains and generation is authorized, prepare only the minimum missing or unsuitable references and necessary views, not a full asset pack by default. Reassess affected consumers after storyboard changes without rebuilding unrelated assets.

Original product photos remain structural evidence. A generated cleanup or new view must not replace them as proof of an unseen mechanism. Existing suitable images need not be regenerated just to obtain a white background.

## White-background outputs

Every image you generate or edit as an asset must have a solid pure-white background. Include this explicitly in the saved generation prompt. Show the subject clearly, with sufficient margins, neutral readable lighting and no environmental backdrop, floor texture, gradient, transparency, captions, panels or watermark. Preserve actual product markings. A small natural contact shadow may support the subject without turning the background gray. Keep a character's full outfit, footwear and required accessories visible and consistent across useful views. Generate only the views actually needed; derive additional views from the inspected identity reference, not a new independent description.

For setting needs, prepare isolated set pieces or architectural elements on white; location composition, environmental lighting and finished scene frames belong to the storyboard artist. Existing source photos may retain their original background. The white-background requirement applies to every new asset image you produce, not to all downstream storyboard frames.

Read [image prompting](../creative-ad-director/references/image-prompt-writing.md) for relevant product geometry and reference roles. Use actual image inputs, not 'same as before' without the referenced image. Identity and costume may be chosen during asset preparation; after selection they remain fixed for dependent shots unless the design explicitly changes them.

## Save, inspect and hand off

Use request_generation with mediaKind=image, generationPurpose=asset, the complete prompt and explicit references (an empty list for a new asset with no references). Do not attach an asset task to a shot. Use the user's selected model and execution mode. Inspect the returned task for real status and result asset IDs; submission is not completion. Read result pixels with mstudio_read_image and compare against original evidence and other views. Check the white background, identity, outfit including shoes, product geometry, completeness and unintended additions. Missing pixel access means uninspected. Do not mark a result ready based on task success alone.

Use existing project image IDs directly. Save consuming shot references through set_references(id=shotId,references=[{assetId,purpose}]), preserving unrelated existing references. Use names and reference purposes to explain front, side or identity roles; do not create subject/view records. New reference image results are collected into project media. Inspect result pixels before assigning references. Changing shot references affects future requests, never the actual inputs of old tasks. Do not change shot design, frames or timeline.

Return actual image asset IDs, their roles, inspection status, fixed attributes and specific unresolved dependencies. Continue independent asset work while waiting; when ending with pending generation, return task IDs and the condition for resuming inspection. Dependent frame generation waits for inspected suitable assets and includes their real images. Same-shot A/B continuity still requires the artist to inspect and supply the accepted A frame when generating B; white-background identity references alone do not fix shot geometry. Two failed repairs of the same issue require a concrete blocker or changed approach rather than repeated identical requests.

Detailed role scope and execution methods: [role methods](references/role-methods.md). Read only when relevant.
