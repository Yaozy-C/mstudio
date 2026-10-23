# Photographic direction

Use for live-action or photorealistic character assets, scene stills and control frames. This is the positive construction method: decide what kind of photograph is being taken before describing the subject. Camera and film labels are cues for appearance, not a guarantee of physical simulation.

## Six decisions before the prompt

1. **Capture.** Film or digital, aspect ratio, grain, contrast, color response. Name a real stock only when its look serves the scene; do not use it as a substitute for light.
2. **Lens, distance and height.** Choose the perspective the shot needs, not a fashionable focal length. State camera height relative to the subject and the visible consequence.
3. **Composition.** Attention centre, foreground layer, negative space, eyeline, horizon placement and what stays out of frame.
4. **Light.** Source, size, direction, ratio, bounce and practicals. Every visible highlight and shadow should have a cause.
5. **Color and exposure.** White balance, palette relationships, saturation, highlight roll-off and shadow depth. Keep skin and product colors identifiable.
6. **Surface and finish.** Skin, hair, cloth, wood, metal and glass respond differently; grain, halation and lens falloff belong only when the chosen capture calls for them.

## Write the photograph

- Open with the photographic fact when realism is requested: "A real photograph of..." or "A photorealistic...".
- Name the moment and the camera relationship. "Camera at seated eye level, 50mm, three metres from the platform" is more useful than "cinematic shot".
- Describe light as a physical setup: a window camera-left, a 3:1 falloff, a pale wall bounce, a low sun behind the subject.
- Say what the light does on each material: a soft sheen on the forehead, dry lips, linen folds catching a highlight, matte metal with a narrow reflection.
- Put the subject in a real place with depth and small, specific objects. A generic background makes the subject look pasted in.
- Keep one visual idea per frame. A second idea belongs in another frame, not in more adjectives.

## Breaking the AI look

| Symptom | Usual prompt cause | Positive replacement |
|---|---|---|
| Waxy, uniform skin | "smooth skin", "flawless", "beauty", "retouched" | Pores at the actual scale, regional sheen, fine lines, vellus hair, uneven tone, shadow-side detail |
| Catalogue pose | "full body, standing, neutral expression, plain background" | Mid-action, weight on one leg, hands occupied, gaze off-camera, a real room |
| Flat light | "soft even lighting", "studio light" | Directional window light, 3:1 ratio, shadow-side bounce, motivated practicals |
| Plastic fabric | "clean, crisp, premium" | Folds caused by the pose, seams, weave, slight wear, fabric weight |
| Orange or teal wash | "cinematic, warm sunset, teal and orange" | Name sky, sea, skin and neutral values separately; keep one warm source |
| Over-sharpened detail | "8K, ultra detailed, HDR, sharp focus everywhere" | Natural optical sharpness, shallow depth of field where appropriate, grain, gentle highlight roll-off |
| Generic face | "beautiful, symmetrical, perfect features" | Specific bone structure, slight asymmetry, a distinctive nose or chin, an unposed expression |
| Empty backdrop | "simple background", "minimal" | A lived-in room, wall texture, depth falloff, one or two ordinary objects |

## Camera and optics

| Choice | Visible consequence | Use when |
|---|---|---|
| 24–28mm | Expands space, exaggerates near edges and foreground | Environment, crowd, interior geography |
| 35mm | Natural reportage perspective, readable scene | Establishing stills, two-shot with context |
| 50mm | Close to human perspective, moderate compression | Observed scene, medium two-shot |
| 85–105mm | Compresses background, isolates the face | Portrait, reaction, intimate detail |
| Low camera | Subject gains scale or threat | Performance, product hero |
| Eye level | Direct, intimate, neutral | Dialogue, identity plate |
| High camera | Subject looks smaller or exposed | Vulnerability, overhead layout |

State height and distance together with the lens. A lens label alone does not establish perspective, and a shallow aperture does not repair a weak composition.

## Light and color

- A large source close to the subject is soft; a small or distant source is hard. Name the size rather than saying "soft light".
- Direction creates form: front light flattens, 45-degree side light models, back light separates. Say which side and what fills the shadow.
- A 2:1 ratio is gentle; 3:1 is a clear portrait ratio; 4:1 or more is dramatic. Use a ratio only when it describes the intended look.
- Let the environment supply color: warm sand bounce against a cool dusk sky, a tungsten room against daylight through a window. Do not put a global orange or teal grade on every surface.
- Protect skin and product colors. A warm scene does not make a brown complexion orange, and a cool scene does not make it gray.
- Keep highlights from clipping and shadows from crushing. A real photograph can have either, but the choice should be intentional.

## Asset versus scene light

An identity or construction asset uses readable light so the subject can be inspected: a directional window or a large source with a gentle falloff is enough. It does not need a seamless studio backdrop. A scene still uses motivated story light, which may be harder, darker or more colored; preserve identity and construction while changing the light.

Do not carry an asset plate's backdrop, exposure or pose into a scene. Do not carry a scene's dramatic grade into an identity plate, where it would hide skin or product color.

## Worked example: identity plate

> A real photograph of a fictional woman in her mid-thirties, head and shoulders, 85mm at f/2.8, camera at eye level. She stands in a lived-in room beside a window; the window is a large soft source from camera left, with a gentle falloff to the shadow side and a pale wall bounce. She wears a worn indigo cotton shirt, collar open, one cuff folded unevenly. Her warm brown skin shows pores across the nose and cheeks at this distance, fine lines at the eyes, a slight sheen on the forehead and natural uneven tone; the shadow cheek stays readable. Dark hair is tied back with loose strands at the temple. She is caught between actions, looking just past the lens, mouth relaxed, one shoulder lower. The room falls into soft depth behind her. Natural color, gentle contrast, fine 35mm grain, no retouching.

The prompt gives a place, a light setup, a specific person, a wardrobe state and an unposed moment. It does not ask for "beautiful", "perfect" or "studio".

## Worked example: scene still

> A photorealistic 35mm film still, camera at seated eye level on wet sand, 50mm at T2.8. A woman singer stands on a low timber platform at dusk, hand just released from a microphone, looking down-left with a small dawning smile. In the lower left foreground an older woman sits cross-legged, hands meeting in a clap. The sun is below the horizon: a thin apricot band remains, the sky above is dusty blue-grey, the sea is dark slate. Warm sand bounce lights the faces; a cool ambient fill keeps the shadow side readable. Rust-red linen, cream trousers, grey curls, weathered planks and matte black metal each catch the light differently. Fine grain, soft highlight roll-off, open shadows, muted natural color.

## Check before submitting

- Can I name the camera position, lens, distance and height?
- Does every major light have a source, direction and falloff?
- Is there one clear attention centre and a readable foreground/background relationship?
- Are skin, hair, cloth and metal described through different responses?
- Did I use a positive, visible replacement for every "not" I removed?
- Did I avoid promising a resolution or control the endpoint does not expose?

## Sources and limits

[OpenAI's image prompting guide](https://developers.openai.com/api/docs/guides/image-prompting) recommends describing visible details, requesting "photorealistic" explicitly, using photography language and treating camera specifications as cues. [OpenAI's GPT Image 1.5 prompting guide](https://developers.openai.com/cookbook/examples/multimodal/image-gen-1.5-prompting_guide) recommends prompting as if a real photo is being captured in the moment and explicitly asking for pores, wrinkles, fabric wear and imperfections. These are method references; they do not guarantee that a given endpoint reproduces a physical camera.
