# Photographic direction for boards

Use for live-action storyboard panels and authorized look frames. A board is four photographs, not four illustrations of a shot. Decide the shared photographic treatment once, then give each panel its own visible instant.

## Shared look block

Put these decisions once at the top of the board prompt, not in every panel:

- Capture: film or digital, grain, contrast and color response.
- Lens and camera: focal length, distance, height and the resulting perspective.
- Light: source, size, direction, ratio, bounce and practicals. Panels may change exposure or time of day only when the design does.
- Palette: sky, sea, skin, wardrobe and neutral values as separate relationships. Do not reduce the whole film to "warm orange".
- Finish: highlight roll-off, shadow depth, lens character and grain.

## Per-panel direction

Each panel needs one photographic idea: the attention centre, the state, and the light/value relationship that makes that state readable. Do not repeat the same adjectives in all four cells. A fixed camera keeps its height, lens and landmarks; a moving camera keeps world geometry and changes projection.

For a continuous take, panels are successive moments of one photograph, not four stylistic variations. For grouped editorial shots, preserve the designed cut map and give each viewpoint its own camera relationship.

## Board scale

At a 2×2 board, each panel is roughly one quarter of the output. If the delivered board is 1672×941, a panel is about 836×470: enough for blocking, not for pore-level skin or precise fingers. State the actual limitation instead of asking the model for detail the panel cannot hold. When the look matters more than the sequence, an authorized full-frame look frame is the right artifact; do not demand a board to carry both.

## Breaking the AI look

| Symptom | Likely cause | Positive repair |
|---|---|---|
| Four identical faces | One identity description repeated, no per-panel state | Give each panel one distinct gaze, mouth, hand and weight state |
| Orange wash across all panels | "Warm sunset" repeated without color relationships | Name sky, sea, skin and neutral values; keep one warm source |
| Flat panels | "Soft even light" in every cell | One motivated source per setup, with a shadow side and bounce |
| Catalogue background | Plain backdrop carried from an asset plate | Put the figures in the actual location with depth and landmarks |
| Mushy hands/faces | Too much detail demanded at 836×470 | Simplify the pose, keep hands occupied and visible, accept blocking scale |
| Grid leakage | Model treats the board as a layout | Ask for four full-bleed scene panels with plain gutters, no labels or arrows |
| Inconsistent crowd | Generic "audience" | Give a count, a few distinct postures and clothing colors, and keep them across panels |

## Worked shared block

> A photorealistic 35mm film board, four equal 16:9 scene panels with thin plain gutters, camera at seated eye level on wet sand, 50mm at T2.8. The sun is below the horizon: a thin apricot band remains, the sky above is dusty blue-grey, the sea is dark slate. Warm sand bounce lights the faces; a cool ambient fill keeps the shadow side readable. Rust-red linen, cream trousers, grey curls, weathered planks and matte black metal each catch the light differently. Fine grain, soft highlight roll-off, open shadows, muted natural color.

Then each panel describes only its instant: who looks where, what the hands do, what changes in the light or sea.

## Check before submitting

- Is the shared look stated once, and is each panel a distinct state rather than a duplicate?
- Does every panel have a readable attention centre at actual panel scale?
- Are the camera height, lens and landmarks consistent where the design requires it?
- Did I replace style slogans with visible light, color and material relationships?
- Did I state the real panel resolution instead of promising detail it cannot hold?

## Sources and limits

[OpenAI's image prompting guide](https://developers.openai.com/api/docs/guides/image-prompting) recommends describing visible details, requesting "photorealistic" explicitly and treating camera specifications as cues. The panel-scale arithmetic describes the delivered image, not a model capability. These methods do not guarantee panel consistency or exact motion.
