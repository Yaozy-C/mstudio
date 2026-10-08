# Write one image at one visible moment

Translate the selected design into a visible instant. Keep the requested art direction; a photograph, a rough storyboard and a graphic layout need different treatment. A still cannot hold incompatible before and after states.

## Visible decisions

- Moment and pose: where the body or object is, its support, weight and decisive contact. Distinguish anticipation from result. For a difficult pose consult [animation principles](../../creative-ad-director/references/animation-principles.md).
- Camera and composition: a compatible view direction, height, visible surfaces, subject scale and crop, with the attention centre and the essential spatial relationships readable. For unresolved staging consult [shot grammar](../../creative-ad-director/references/cinematography.md).
- Light and focus: a coherent source direction, softness and exposure, and the background cues needed to understand the scene. Mood comes from visible light, material or weather.
- Output kind: a clean scene frame, an annotated storyboard or a graphic design. Keep shot IDs, timecodes, arrows and captions out of clean frames; when text is requested, give the exact copy and placement.

Script, dialogue, sound and camera trajectory are context. Include only what is visible at this moment, naming a sound source only when it must be seen. Add detail to remove ambiguity, not new events, people, props or slogans. A surreal effect changes the chosen relationships while keeping a compatible source and state.

## Product structure and references

Openings, closure paths, part count, connections and visible surfaces come from original image evidence. A proposed staging action cannot redefine the product; hidden geometry stays unknown; texture similarity never establishes structural fidelity. Return an evidenced design conflict to direction.

List the supplied images in their actual order and describe each visual use: identity or geometry, composition or light, state. Put that description in `purpose`; `role` is `reference` for guidance or `edit` for the source image being modified. Product photos constrain the product, not the whole original arrangement; keep real markings and drop irrelevant dimension labels or interface chrome.

An independent request cannot see "the previous frame" without that image. For same-view continuity, supply an inspected base as composition and state reference together with the original product evidence. Use "change only X" wording for a real edit; a new scene is a new generation. State a model limitation when it cannot take the required references.

## Worked example: frame f1 of the lunch-bag shot

The design asks for the instant just after the lid settles open, container visible inside. Two inputs exist: the customer's product photo and the lead's white-background identity asset.

Weak request:

```text
references: [{assetId: a-product-front, role: reference, purpose: "content reference"},
             {assetId: a-lead-identity, role: reference, purpose: "reference image"}]
prompt: Cinematic 8K ultra realistic product shot of a woman opening a premium lunch bag, masterpiece, perfect hands, no deformation, no extra fingers, no text, no blur.
```

Nothing names which subject each image supplies, the identity asset's studio light will leak into the office, the prompt describes no moment, and the negative list replaces the visible facts.

Compiled request:

```text
references: [{assetId: a-product-front, role: reference, purpose: "the customer's blue lunch bag: proportions, black perimeter zipper, two silver pulls, front zip pocket and side mesh pockets (geometry and material evidence, not the white background or the label)"},
             {assetId: a-lead-identity, role: reference, purpose: "the lead's face, hair and grey knit sweater identity; not her standing pose, not the studio light"}]
prompt: Close three-quarter view across an office desk in soft window daylight from the left. The blue lunch bag stands on the desk, its top zipper fully open and the padded lid resting back on its rear seam; inside, a closed rectangular lunch container sits upright with visible clearance to the bag walls. The lead's left hand steadies the bag's side; her right hand rests on the desk beside it. She looks down at the opening. Woven blue fabric with fine creases, black zipper tape, two silver pulls at the open end of the track. Desk edge at the bottom of frame; blurred office shelves behind. Natural skin with soft shadow modelling, no sheen beyond the brow.
```

The compiled text names one moment, the visible surfaces and their light, and the relationships the still has to prove. The prompt's parameters (aspect, size) go in the tool's parameter fields, not in the prose.

## Final prompt and check

Write the visible output, the selected state, the composition, the light and the necessary reference constraints in direct paragraphs. Specifications belong in supported parameters. Keep planning notes, review history and quality slogans out of the final text; describe material behaviour instead of adding negative lists.

Compare the output with the intended state, viewpoint, support and contact, identity, components and requested text, then repair the observed mismatch. Fine detail alone does not establish realism. For photographic people, skin and environmental light, read [photographic appearance](../../creative-ad-director/references/photographic-appearance.md) when the task needs it.
