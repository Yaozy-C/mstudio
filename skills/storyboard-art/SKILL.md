---
name: storyboard-art
description: Execute designed shots as storyboard frames in Mstudio, including static composition, image prompts, generation and local image repairs.
---

# Storyboard frames

The short [CORE.md](CORE.md) is automatically loaded from the database. For every image/video prompt, the animation principles and applicable prompt guide are mandatory; other detailed methods are read when needed.

Read the current shot, relevant script, director handoff and original references. Preserve shot purpose, action and spatial relationships. Resolve secondary set details; return a specific gap when the required viewpoint or moment is missing. Do not rewrite the story, duration or video prompt.

Before translating a product shot, apply the structural-evidence and reference rules in [image prompts](../creative-ad-director/references/image-prompt-writing.md). A director's design is not product evidence. Resolve a conflict before generating dependent frames; do not satisfy a camera or action instruction by changing the product's opening, connections or parts.

Apply staging and handoff from [shot grammar](../creative-ad-director/references/cinematography.md): attention center, subject/target/obstacle relationship, direction, key pose and meaningful occlusion. A preparation frame must not show a completed result or disclose a planned surprise. A change to shot expression goes back to the director.

Choose rough, clean or realistic production frames by use, not automatic polish. Verify difficult static relationships before expanding; reuse accepted images. Each frame shows one moment. A/B frames retain landmarks and readable state changes without adding shots or duration.

For frames that depend on a shared unverified identity, viewpoint or interaction, generate the minimum useful anchor first, wait for its real asset, and inspect its pixels against the originals. Pending, queued or successful submission is not a passed anchor. Until it passes, continue independent design work but do not submit the dependent batch. Reuse a suitable already-inspected anchor; this is an execution dependency, not another user approval or a mandatory sample for unrelated images. If the run must end while waiting, return the task ID and exact continuation condition.

## Prompts and tasks

Read [image prompts](../creative-ad-director/references/image-prompt-writing.md). Save shot.framePrompt for shot-draft work. When a generation card is referenced, inspect(section=generation, taskKey=referencedTask.key) and update only that task with update_generation(taskKey,text); do not also overwrite the shot draft unless requested. Do not browse all tasks when the exact key is known. Read only the target and necessary evidence; history does not add unrelated work.

Generate images only within the requested scope. Completion yields a real assetId for shot.frames; task IDs are not asset IDs. Do not duplicate running tasks. Text-only work does not generate images.

## Inspect and return

Use [frame inspection](../product-storyboard/references/preview-images.md) for viewpoint, identity, perspective, support, contact and cross-frame state. Fix execution deviations locally; send design conflicts to the director. Return shot IDs, actual assets, selected moments and concrete deviations. Static correctness does not establish motion or audio.

Use mstudio_read_image(assetId) for current project images. mstudio_reopen_image accepts only an imageId from a historical unloaded-image notice. Filenames, task IDs and asset IDs are not interchangeable. If image input is unavailable, report the limitation instead of guessing or claiming to have viewed metadata.

Read relevant existing references and shared asset records/images before frame work. Missing identity references repeated across distinct shots go to asset-designer via coordination after an inventory check. Do not request public asset generation for each single-shot element or merely because a shot has A/B frames. Preserve selected identity, wardrobe and footwear; choosing an outfit does not mean reselecting it for every frame. Supply ready shared images as identity references and inspected A as composition/state reference for dependent B. White asset backgrounds do not dictate the scene background.

Detailed role scope and execution methods: [role methods](references/role-methods.md). Read only when relevant.
