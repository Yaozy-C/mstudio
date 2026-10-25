# Photographic people: prompt decisions and local repairs

Use for photographic character assets and appearance repairs. Retain the selected project art direction and [character identity rules](character-assets.md). Use [photographic direction](photographic-direction.md) to choose the camera, light and color, and [skin realism](skin-realism.md) for close work. Choose subjects, wardrobe, setting and photographic treatment from the current brief and selected references.

## Choose the photograph before describing the skin

Name the photographic treatment that serves the task: readable identity study, environmental portrait, observed candid or deliberately polished beauty work. Resolve an unspecified choice with the existing brief. An identity study can be calm and plainly lit without resembling a corporate headshot; a cinematic project can retain its cinematic look. Do not impose documentary grain, studio gloss, a dramatic expression or a fake candid action on every person.

State the photographic fact first: "a real photograph" or "a photorealistic photograph". Name the lens, distance and camera height, the light source and its size, any motivated fill, and only the material cues that matter at this scale. A lens label can suggest perspective and focus, but it cannot replace those relationships. Keep the face sufficiently resolved for its intended use; very shallow focus is not a universal quality improvement.

Keep comparison labels, demographic test instructions, review notes and internal use-case tags in task metadata. The generator-facing prompt should describe the wanted picture. Use prose or short labeled sections according to complexity; do not require a fixed word count, eight-part formula or complete negative bank. Retain only exclusions relevant to the brief or an observed failure, in syntax the selected endpoint accepts.

## Make skin specific to the person and light

Describe complexion separately from exposure and temporary surface state. A shadow side can be darker without changing the person's complexion. Brightening exposure should not whiten dark skin; reducing an excessive reflection should not make the whole face flat or uniformly matte. Let regional color and sheen vary while the light stays coherent across skin, hair and clothing.

Age influences facial volume, eyelids, neck, fine lines and hair as well as surface detail. Child skin can be smooth without being airbrushed. Older skin is not merely a young face covered with wrinkles. Preserve the actual selected person's features; do not add acne, freckles, asymmetry, age spots, coarse pores or sweat as a universal cure for synthetic skin.

Avoid generic beauty or realism slogans as substitutes for the actual person. A studio source, smooth skin or neutral expression can suit the brief. Diagnose facial volume, coordinated expression, head/neck/body continuity and hand contact before adding surface cues. Use only details the framing can resolve, and retain natural shadow and soft transitions. Occupation does not justify invented aging, scars, dirt or damage.

When words leave the photographic finish unresolved, use an available selected appearance image with a declared role: light falloff, skin reflectance, detail softness or color treatment. Keep its role separate from the identity source. Inspect compatibility before submitting; neither a beautiful reference nor a long prompt guarantees the requested rendering.

## Repair the observed region

Use the existing image and the specific feedback supplied by the user or coordinator: facial volume, exposure/color, excessive specular coverage, uniform microtexture or sharpening. Address that assigned property rather than inventing a full retouching pass. An approved identity needs an actual edit input and explicit invariants, not another independent text-to-image request.

Use the accepted base as the edit target. Keep identity, age, complexion, pose, framing and unrelated regions; change the diagnosed appearance property. Light changes are allowed only within that repair's scope. Preserve the specified exposure, sharpness and age while making the assigned edit. Return the new candidate; do not judge acceptance or escalate into further generations yourself.

## Compare methods without confounding the result

For a new prompt approach, hold the subject brief, references, model, settings and intended format constant as far as the endpoint permits. Change only the assigned design decision and return actual images and request details for coordinator review and user selection. A text-to-image comparison produces independent candidates; it does not prove identity preservation. One output does not establish repeatability across ages or complexions. Test further only within the authorized scope.

## Sources and transfer limits

[OpenAI's GPT Image 2.5 guide](https://developers.openai.com/api/docs/guides/image-prompting) supports concrete photographic briefs, assigned reference roles and narrow edits. [OpenAI's public prompt method](https://github.com/openai/skills/blob/main/skills/.system/imagegen/references/prompting.md) separates prompt shaping from execution settings. [Lovstudio's portrait recipes](https://github.com/lovstudio/professional-portrait-skill/blob/main/references/prompts.md) separate identity preservation, exposure and targeted correction. [Portrait Photo Pack](https://github.com/SamurAIGPT/open-ai-image-agent/blob/main/agents/portrait-photo-pack/SKILL.md) retains an actual identity reference across requested variations. These are method references, not evidence that any wording fixes skin in this endpoint. Do not copy another runtime's tools, provider controls or portrait-retouching defaults.
