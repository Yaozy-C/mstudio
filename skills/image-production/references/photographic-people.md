# Photographic people: prompt decisions and examples

Use for photographic character assets and appearance repairs. Retain the selected project art direction and [character identity rules](character-assets.md). Use [photographic direction](photographic-direction.md) to choose the camera, light and color, and [skin realism](skin-realism.md) for close work. The examples below illustrate prompt construction, not a mandatory look, subject list, wardrobe or view pack.

## Choose the photograph before describing the skin

Name the photographic treatment that serves the task: readable identity study, environmental portrait, observed candid or deliberately polished beauty work. Resolve an unspecified choice with the existing brief. An identity study can be calm and plainly lit without resembling a corporate headshot; a cinematic project can retain its cinematic look. Do not impose documentary grain, studio gloss, a dramatic expression or a fake candid action on every person.

State the photographic fact first: "a real photograph" or "a photorealistic photograph". Name the lens, distance and camera height, the light source and its size, the shadow-side fill, and the few material cues that matter at this scale. A lens label can suggest perspective and focus, but it cannot replace those relationships. Keep the face sufficiently resolved for its intended use; very shallow focus is not a universal quality improvement.

Keep comparison labels, demographic test instructions, review notes and internal use-case tags in task metadata. The generator-facing prompt should describe the wanted picture. Use prose or short labeled sections according to complexity; do not require a fixed word count, eight-part formula or complete negative bank. Retain only exclusions relevant to the brief or an observed failure, in syntax the selected endpoint accepts.

## Make skin specific to the person and light

Describe complexion separately from exposure and temporary surface state. A shadow side can be darker without changing the person's complexion. Brightening exposure should not whiten dark skin; reducing an excessive reflection should not make the whole face flat or uniformly matte. Let regional color and sheen vary while the light stays coherent across skin, hair and clothing.

Age influences facial volume, eyelids, neck, fine lines and hair as well as surface detail. Child skin can be smooth without being airbrushed. Older skin is not merely a young face covered with wrinkles. Preserve the actual selected person's features; do not add acne, freckles, asymmetry, age spots, coarse pores or sweat as a universal cure for synthetic skin.

Avoid "flawless", "perfect", "beauty", "retouched", "airbrushed", "porcelain", "glowing" and "studio softbox" for a photographic identity. They pull the model toward a cosmetic catalogue face. Describe the specific region, the scale of detail and the light response instead. Hands, neck, ears and hairline need the same attention as the eyes; a perfect face on a smooth neck or malformed hand still reads as synthetic.

When words leave the photographic finish unresolved, use an available selected appearance image with a declared role: light falloff, skin reflectance, detail softness or color treatment. Keep its role separate from the identity source. Inspect compatibility before submitting; neither a beautiful reference nor a long prompt guarantees the requested rendering.

## Repair the observed region

Use the existing image and the specific feedback supplied by the user or coordinator: facial volume, exposure/color, excessive specular coverage, uniform microtexture or sharpening. Address that assigned property rather than inventing a full retouching pass. An approved identity needs an actual edit input and explicit invariants, not another independent text-to-image request.

Use the accepted base as the edit target. Keep identity, age, complexion, pose, framing and unrelated regions; change the diagnosed appearance property. Light changes are allowed only within that repair's scope. Preserve the specified exposure, sharpness and age while making the assigned edit. Return the new candidate; do not judge acceptance or escalate into further generations yourself.

## Complete prompt examples

Adapt only the example matching the requested deliverable. Their subject details describe proposed new people; use the selected identity image instead when one already exists. Output size, quality and supported controls belong in parameters, not this prose.

### New child identity study

An unretouched photograph of a fictional six-year-old boy with light warm skin and short dark hair, wearing a plain cotton shirt. 85mm at f/2.8, camera at his eye level, head and shoulders, facing almost forward, relaxed and attentive. He sits beside a large window camera-left in an ordinary quiet room; broad indirect daylight models his face, with a pale wall bounce on the shadow side. The face is clear enough to compare its proportions. Smooth child cheeks retain subtle warmth and fine detail appropriate to this distance. Natural color and soft highlight transitions. No beauty retouching or added skin blemishes.

### New adult identity study with a deep complexion

A natural photographic identity study of a fictional thirty-year-old man with deep brown skin, short tightly coiled hair and light facial stubble, wearing a plain cotton shirt. 85mm at f/2.8, camera at eye level, head and shoulders, nearly frontal, calm expression. A large window camera-left supplies soft directional daylight; a pale wall bounce leaves the shadow cheek readable. Retain the depth and hue of his complexion. Small highlights follow the forehead and nose while the cheeks have a gentler surface response. Eyes and facial proportions are clearly resolved; the plain room recedes softly. Restrained retouching, without global skin lightening.

### Environmental portrait of an older person

An observed photograph of a fictional woman in her late sixties with warm brown skin and short gray curls, resting beside the doorway of her home. 85mm at f/2.8, medium close-up at eye level as she quietly looks toward someone just beside the camera. Open shade from the doorway lights her face, with a warm wall bounce into the shadow cheek. Her age reads through the eyelids, cheek volume, mouth and neck as well as naturally placed lines. Skin, cotton clothing and gray hair each respond differently to the light. Natural color, gently resolved detail and a subdued recognizable setting. No glamour retouching or exaggerated weathering.

### Local correction of an excessively shiny face

Edit image 1, the supplied portrait. Reduce only the broad shiny patches over the forehead and nose to a restrained localized sheen, retaining the existing light direction and facial volume. Keep the same person, face geometry, age, complexion, expression, hair, clothing, pose, crop and background. Preserve the surrounding skin color and fine detail at its existing scale. Do not replace the shine with blur, coarse grain or extra blemishes.

## Compare methods without confounding the result

For a new prompt approach, hold the subject brief, references, model, settings and intended format constant as far as the endpoint permits. Change only the assigned design decision and return actual images and request details for coordinator review and user selection. A text-to-image comparison produces independent candidates; it does not prove identity preservation. One output does not establish repeatability across ages or complexions. Test further only within the authorized scope.

## Sources and transfer limits

[OpenAI's GPT Image 2.5 guide](https://developers.openai.com/api/docs/guides/image-prompting) supports concrete photographic briefs, assigned reference roles and narrow edits. [OpenAI's public prompt method](https://github.com/openai/skills/blob/main/skills/.system/imagegen/references/prompting.md) separates prompt shaping from execution settings. [Lovstudio's portrait recipes](https://github.com/lovstudio/professional-portrait-skill/blob/main/references/prompts.md) separate identity preservation, exposure and targeted correction. [Portrait Photo Pack](https://github.com/SamurAIGPT/open-ai-image-agent/blob/main/agents/portrait-photo-pack/SKILL.md) retains an actual identity reference across requested variations. These are method references, not evidence that any wording fixes skin in this endpoint. Do not copy another runtime's tools, provider controls or portrait-retouching defaults.
