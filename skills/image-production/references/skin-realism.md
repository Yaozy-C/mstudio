# Skin realism

Use for photographic people at medium close-up or closer, and for repairing synthetic skin. Skin is not a texture layer added after the face: it is anatomy, regional color and light response seen at a specific distance. Texture alone cannot make a face photographically credible.

## Read the face before the texture

- Establish bone structure, face shape, jaw, chin, nose bridge and cheekbones before surface detail.
- Allow slight asymmetry: one eye a little lower, one brow different, a nose that is not centered by geometry. Symmetry imposed by the prompt reads as synthetic.
- Age lives in eyelids, under-eye volume, nasolabial folds, mouth corners, neck and hands before it lives in "wrinkles".
- Hairline, ears and neck are common synthetic tells. Give them the same attention as the eyes.
- At full-body scale, identity and silhouette matter more than microscopic pores. Do not describe pore-level detail that the frame cannot show.

## Regional color

- Let forehead, cheeks, nose, perioral area, ears and neck differ in warmth and redness at low amplitude.
- A shadow side is darker, not a different complexion. Do not lighten deep skin to make it readable; use bounce or exposure, not a global skin change.
- Keep lips, eyelids and nostrils distinct from cheek tone. A uniform skin color across the whole face is a mask.
- Sweat, oil, dust, sunburn and makeup are temporary states. Do not bake them into a reusable identity asset unless the design requires that state.

## Surface at the actual scale

- At a head-and-shoulders portrait, pores are visible mainly on the nose, inner cheeks and forehead; they soften on the outer cheeks and neck.
- Fine lines appear at the eyes, mouth and neck according to age. Do not give a young face an old texture map, or an older face a young person's volume.
- Vellus hair catches light at the jaw and temple. Individual loose hairs cross the skin without drawing a hard outline.
- Normal sheen follows the light: a small highlight on the nose tip, forehead and lower lip. It is not a continuous gloss over the cheeks.
- Dry lips, a slightly uneven lip line, faint under-eye shadow and small color irregularities are ordinary. They are not "flaws" to be removed.
- Fabric, hair and skin must not share one gloss level. If the shirt is matte, the forehead can still have a localized sheen.

## Light response

- Name the source size, direction and fill. A large window creates a soft transition; a small source creates a harder specular highlight.
- Skin transmits a little warmth in the shadow side, especially on ears, nose and fingers. This is a subtle color relationship, not a red overlay.
- The specular highlight should sit on a facial plane facing the source. A highlight that wraps the whole face is a plastic tell.
- Keep the shadow side readable without flattening the key. A 3:1 ratio is a useful portrait starting point, not a rule for every scene.

## Hands, neck and ears

- Check finger count, knuckle direction, nail direction and where each hand is supported. Hands are a stronger realism signal than pores.
- Neck tendons, Adam's apple, collarbones and ears need age-appropriate structure. A perfectly smooth neck under an older face is a tell.
- Hair roots, sideburns and the hairline should sit on the skull, not as a painted edge.
- If a hand holds an object, describe contact, grip pressure and the object's response before describing skin.

## Example: woman in her thirties, window light

> A real photograph of a fictional woman in her mid-thirties, head and shoulders, 85mm at f/2.8, camera at eye level. A large window camera-left supplies soft directional daylight; a pale wall fills the shadow side at roughly 3:1. She has warm brown skin with visible pores across the nose and inner cheeks, faint lines at the eyes, a slight sheen on the forehead, dry lips and natural uneven tone. Her dark hair is tied back, with loose strands at the temple. One shoulder is lower; she looks just past the lens. The plain room falls into soft depth. Natural color, fine grain, no retouching.

## Example: woman in her sixties, open shade

> An observed photograph of a fictional woman in her late sixties, medium close-up at eye level, 85mm. Open shade from a doorway lights her face; a warm wall bounces a little light into the shadow cheek. Her age reads through eyelid weight, cheek volume, mouth corners, neck lines and the backs of her hands. Short grey curls have individual strands and a slightly coarse texture. Skin shows pores, faint sun damage and a localized sheen on the nose and forehead. Indigo linen shirt with a softened collar. Muted natural color, gentle highlight roll-off, fine grain.

## Repair the observed region

Use the existing image and the specific feedback: facial volume, color, specular coverage, uniform microtexture or sharpening. Change only that property and keep identity, age, pose, framing, light direction and unrelated regions. Do not answer "shiny" with blur, "waxy" with coarse grit, or "too smooth" with a global pore overlay.

## Cues that create synthetic skin

- "Flawless", "perfect", "beauty", "retouched", "airbrushed", "porcelain", "glowing".
- "Studio softbox", "even lighting", "no shadows" on a face.
- "Smooth skin" without a scale, light direction or regional variation.
- "8K", "ultra detailed", "HDR", "sharp focus" applied to the whole frame.
- Symmetrical features, identical eyes, a perfectly centered nose.
- Skin that is uniformly matte or uniformly glossy.
- Hands hidden, blurred or cropped because they are hard to draw.

## Check before submitting

- Is the face specific enough to recognize, with a real bone structure and slight asymmetry?
- Does the skin have regional color and a light-driven highlight rather than a global texture?
- Are neck, ears, hairline and hands consistent with the face's age?
- Is the skin's detail appropriate to the actual framing distance?
- Did I avoid beauty and studio-polish words that pull toward a retouched look?

## Sources and limits

[OpenAI's GPT Image 1.5 prompting guide](https://developers.openai.com/cookbook/examples/multimodal/image-gen-1.5-prompting_guide) recommends prompting as if a real photo is being captured, using photography language and explicitly asking for pores, wrinkles, fabric wear and imperfections while avoiding studio-polish language. [OpenAI's image prompting guide](https://developers.openai.com/api/docs/guides/image-prompting) recommends describing visible details and treating camera specifications as cues. These are prompt methods, not guarantees of anatomical or skin fidelity.
