# Image and storyboard case evidence

Reviewed 2026-10-09. This authoring record is outside the packaged runtime Skills. Rules distilled from these cases live in `skills/image-production/`; case-specific settings are not universal requirements.

## Higgsfield: reusable assets for a football/robot advertisement

Source: [Stage 1: Building the Assets](https://higgsfield.ai/academy/courses/cinematic-ad-e2e/stage-1-building-the-assets).

The published prompts request a can view sheet, a full-body/face character sheet, an empty sunny NYC street, a front/rear robot sheet on neutral gray, and separate ball and goal references. Some transformations are short; the newly designed robot receives detailed views, materials and lighting. The lesson describes testing the character in the location before locking them.

Implications: reference design depends on its consumer. Isolated identity, a multi-view sheet and a reusable full location are all legitimate assets. White backgrounds are not mandatory. A short instruction may suffice with a strong actual input; a new designed subject needs its missing information. Do not copy provider aliases, model rankings, batch costs or specific robot specifications into general methods.

Limit: prompts and lesson text were inspected; the complete generated advertisement was not watched. The vendor's claim that generated unseen product views prevent hallucinations does not establish their factual accuracy. Real product evidence still governs construction.

## Higgsfield: street layout repair in a headphones advertisement

Sources: [Street scene and map trick](https://higgsfield.ai/academy/courses/ai-ad-3-step/scene-3-the-street-and-the-map-trick), [shot list and skill](https://higgsfield.ai/academy/courses/ai-ad-3-step/the-shot-list-and-the-skill).

The lesson reports unstable time of day, walking direction and inflatable dancer placement/size. Its proposed repair is a layout sketch anchored to a hydrant, followed by revised lighting, direction and physical staging instructions. The displayed sketch was visually inspected: a line-drawn street corner places the dancer beside the hydrant, with buildings, a tree and crosswalk anchoring the space. The shot-list lesson reuses named assets and a shared visual style while distinguishing shots.

Implications: use concrete landmarks and, where needed, a layout reference for persistent spatial drift. Reuse accepted location/identity/style baselines across shots while changing selected camera and event. A layout sketch carries staging, not photoreal appearance or unseen product facts.

Limit: the repaired video sequence was not watched or independently reproduced. The claim that text cannot solve spatial problems is too broad; maps are an available repair route, not a mandatory step or guarantee.

## Krea: four-scene astronaut storyboard

Source: [Example 7, Four-Scene Sci-Fi Film Storyboard](https://www.krea.ai/blog/seedream-5-0-pro-on-krea-12-complex-infographic-reasoning-examples-2026).

Both the published prompt and actual example image were inspected. The prompt orders repair, meteor strike, dodge and injured escape, asking for shared astronaut, spacecraft and art direction. The image is a single four-panel board with separate causal scenes and captions. It demonstrates a legitimate joint storyboard output rather than only an assembled contact sheet.

Observed limits: the second caption contains a spelling defect; the final panel shows an astronaut beside an open hatch, but injury is not clearly established by visible evidence. Labels and polished rendering do not prove every requested state was realized. The sample does not establish general model reliability or usable independent video control frames.

Implications: choose a joint board for sequence review when useful; keep each panel's event and state distinct. Inspect causal readability and requested states in the pixels. Independent clean controls require their own framing, resolution and state checks. Four panels are this example's choice, not a universal template.

## Disney Animation: storyboard development

Source: [Story process](https://disneyanimation.com/process/story/).

The official process describes collaboration on narrative, emotion, timing, staging and framing, with thumbnails refined into clearer drawings and shading. The script-thumbnail image was visually inspected: small, simple panels sit beside script text, connected to relevant passages. The page also provides successive refinement examples and boards from *Once Upon a Snowman* (2020).

Implications: early boards can resolve selected staging and story relationships before expensive polish. Choose the finish level for the review decision. In Mstudio's separated roles, the director owns those design decisions and image production realizes them; this division is a product choice, not a claim that Disney uses the same roles.

Limit: this is evidence for a storyboard workflow, not an AI prompt benchmark. Static boards communicate planned movement but cannot verify playback, speed, sound or mechanical feasibility.

## Changes distilled into runtime methods

- Select reference backgrounds and views by purpose; allow location baselines and useful view sheets.
- Distinguish sequence-review boards, identity sheets and independent production controls.
- Preserve shared scene constraints while specifying per-panel events and selected cameras.
- Match prompt detail to actual supplied images and missing information.
- Use layouts for concrete spatial failures; verify states in pixels rather than captions.

No paid generation, user media upload or model-quality comparison was performed in this review. These methods need validation against Mstudio's actual outputs; unit tests establish storage and routing behavior only.

## Follow-up: actual storyboard-to-video submission

Source: [PoYo's H3 storyboard workflow](https://poyo.ai/hub/gpt-image-2-5-minimax-h3-storyboard-video), especially its [published original request](https://storage.poyo.ai/blog/gpt-image-25-p0-20260910/derived-plan-ce54d9d8c0b9.json).

The request was inspected, not just the article summary. Its `story-video` item uses model `hailuo-03`, exactly one URL in `reference_image_urls`, a 12-second request and a prompt assigning the four panels to consecutive full-screen shots. No first/last-frame fields appear in that request. The provider's twelve sampled output frames were visually inspected: they show separate full-screen compositions progressing from courier/pot to seed handling, planting and flower. These are supplied output samples; the complete video has not been independently reviewed or regenerated.

This supports a concrete whole-board-as-one-reference route on that host. It does not establish equal reliability across hosts, exact timestamp compliance or continuous uncut motion: the example explicitly requests four shots, whereas the lunch-bag design requires four phases within one shot. A prompt for that design must preserve the continuous camera and describe phases rather than introducing cuts.

The [MiniMax official reference rewrite guide](https://huggingface.co/MiniMaxAI/MiniMax-H3/blob/main/docs/VIDEO_PROMPT_WRITING_GUIDE_ref_en.md) explicitly recognizes storyboard/shot-planning references and requires their shot mapping and planning roles to be explained. This is rewrite guidance, not a mandatory six-section submission schema. Mstudio's existing H3 reference adapter maps inputs with `role=reference` to `reference_image_urls`; its separate keyframe adapter maps first/last frames to `image_url` and `end_image_url`. No generation was submitted in this follow-up.
