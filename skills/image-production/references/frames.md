# Storyboard frames

Translate the selected shot into one visible moment using the supplied design and the original references. [Image prompt writing](prompt-writing.md) owns the prompt method.

## Preserve design and evidence

Keep the shot purpose, the spatial relationships and the intended reveal. Resolve secondary set details yourself; return missing viewpoints or conflicting action relationships to direction. Product geometry comes from original image evidence, never from an invented director mechanism; do not change an opening, attachment or part to satisfy a contradictory viewpoint.

Use [shot grammar](../../creative-ad-director/references/cinematography.md) when the staging needs work: attention centre, subject-target-obstacle relationship, silhouette, axis, depth, occlusion and negative space. A preparation frame must not show a completed result. Choose rough, clean or photographic treatment by intended use, without automatic polish.

## Dependent frames

A and B frames express only the required state change while keeping landmarks and screen direction; they add no shots and no duration. For a shared unverified identity, viewpoint or difficult contact, create the minimum useful anchor and inspect it before generating its dependents; reuse a suitable inspected anchor; independent images need no anchor step.

Supply the base image as composition and state reference alongside the original identity and product evidence. Keep the selected wardrobe and footwear. A reusable white-background asset does not dictate the scene background. Identity references needed across distinct shots go to [reference assets](reference-assets.md); single-shot details stay here.

## Inspect and hand off

Apply [frame inspection](frame-checks.md) to viewpoint, perspective, support and contact, part count and cross-frame state. A texture repair does not validate uninspected geometry. Fix execution deviations locally; route design conflicts to direction with evidence.

Save the prompt with `mstudio_set_image_prompt(id, framePrompt)` and link real completed frames with `mstudio_set_shot_frames(id, frames)`. Return the affected shot IDs, the actual assets, the selected moments and the concrete open deviations. Motion and audio judgements need their own evidence.
