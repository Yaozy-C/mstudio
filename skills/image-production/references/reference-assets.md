# Shared reference assets

Use for a subject that recurs across distinct shots and needs a consistent identity, or for an explicit request for a reference asset. Same-shot A and B continuity and finished location scenes belong to [storyboard frames](frames.md).

## Decide what is missing

From the scoped storyboard and the existing assets, identify the actual consumers, the required views and the stable attributes. Read missing shot detail or candidate images only where it changes that decision; a direct request for one asset does not need an inventory of the film. Titles alone cannot establish suitability.

For each relevant subject record the consuming shot IDs, the reusable image IDs and the specific gap, distinguishing reusable, uninspected, pending, needs repair and missing. Reuse suitable originals and inspect an unchecked candidate before replacing it. Similar objects are not automatically the same subject; planned outfit or state changes are legitimate variants. Several frames of one shot do not need a shared asset pack.

Generate only the necessary missing views. Original product photos remain the structural evidence. An existing suitable image needs no regeneration merely because its background is not white.

## Design the asset

A new shared asset uses a solid white background, neutral readable light and sufficient margins. Show the required outfit, footwear and accessories; keep real product markings and connections. A small contact shadow may ground the subject. Keep environmental backdrops, floor textures, panels and labels out unless the user asks for a different deliverable.

For a recurring set element, isolate the needed piece; composition and location lighting belong to the artist. Lock identity, outfit and product construction across the useful views. Derive additional views from an inspected identity image; a front, side and back pack is not automatic. For photographic characters consult [photographic appearance](../../creative-ad-director/references/photographic-appearance.md); at full-body scale identity and complete clothing matter more than skin description. Reference lighting supports identification and does not dictate scene lighting.

## Generate, inspect, attach

Create standalone assets with `mstudio_generate_reference_image`, which takes explicit `references` and no shot ID. Compare the result pixels with the originals for identity, geometry, outfit, completeness and background. Attach suitable real image IDs to the consuming shots with `mstudio_upsert_references`, describing the view or identity in `purpose` or the asset name, and preserving unrelated references.

Return the real assets, their intended uses, the fixed attributes and any unresolved dependency. A shared identity reference does not replace the artist's composition and state reference for dependent frames. Keep changes to asset work and the allowed shot references.
