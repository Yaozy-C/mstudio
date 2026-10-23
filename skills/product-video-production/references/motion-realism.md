# Product motion realism

Use for product demos, hands-on demonstrations, packshots in motion and short commercial clips. A product clip is a physical demonstration, not a moving still. The product's geometry, label and material must survive every frame; the motion must have a cause, weight and a readable end.

## Motion brief

1. **Product fact.** Which product, which surface faces camera, what must remain readable, and which parts may move.
2. **Action.** One benefit or handling beat per short clip: open, close, place, lift, pour, press, turn, wear. State the start, the decisive contact and the end.
3. **Hands.** Which hand enters, from where, what it grips, how much force, and where it exits. Hands are the most common synthetic failure in product video.
4. **Camera.** Product hero angle, lens, height, distance, movement and speed. A turntable, dolly, push-in and orbit are different moves.
5. **Light and material.** The source that reveals the product's finish, the reflection shape, the shadow contact, and what stays fixed.
6. **Sound and speech.** Exact line or voice-over, language, delivery and the visible beat it belongs to; material sounds with a cause.

## Make the product behave

- Keep rigid parts rigid. A bottle, box, zip pull or metal clasp may move as a mechanism; it must not bend like cloth.
- Let soft goods deform under load: a bag side compresses when set down, a strap goes slack when released, fabric folds follow the hand.
- Show contact and support: the base meets the surface, the hand grips, the lid resists then opens, the product settles under its own weight.
- Give the material its own response: matte board with a broad soft highlight, coated metal with a narrow reflection, glass with edge thickness and transmission, woven fabric with directional sheen.
- Keep the label plane facing the camera when it must be read. Do not rotate the product so the label leaves frame and then claim it was preserved.
- One benefit per clip. A five-second clip cannot prove capacity, insulation, zipper, strap and interior at once.

## AI product-video failures

| Failure | Usual cause | Repair |
|---|---|---|
| Product changes shape or proportion | No product reference, or a complex deformation request | Bind the original product evidence; keep the action within rigid/soft behavior |
| Label warps or turns away | Product rotates too far, or the label is not bound | Keep the label plane visible; limit rotation; state the exact readable region |
| Extra fingers or merged hands | Hand action near the product, occlusion, fast movement | Simplify the grip, keep the hand against a clean area, show entry and exit |
| Product floats or slides | No support/contact description | State the surface, base contact, weight and shadow |
| Uniform turntable speed | "Rotate the product" with no beat | Give a start, a slow reveal, a hold on the label and a settle |
| Light sweep changes product color | "Cinematic light" without a source | Name the source, its path and what stays constant; one change only |
| Hand or product morphs into another object | Two actions in one clip | Split into separate generations; keep one action per clip |
| Background moves but product does not | Camera move described without a subject path | Separate camera movement from product movement; give each a start and end |
| Audio does not match the action | Generic music or no sound cue | Put the exact line and the visible beat in the prompt; add material sounds with causes |

## Camera and staging

- A product hero uses a clean angle and a stable horizon. A slow push-in or a controlled 30-degree orbit is enough; a floating camera with no target reads as synthetic.
- Keep the product large enough that its material and label survive. A small product in a wide frame cannot prove construction.
- Use a real surface and a motivated light. A seamless white background is optional, not a default; it often produces a catalogue look.
- When a hand enters, stage the entry from the frame edge, not from inside the product. Show the grip before the force.
- For a cut, make the outgoing and incoming states physically compatible. Do not use a morph to hide a change of grip or support.

## Worked five-second prompt

> One continuous 5-second product clip, no cuts. A matte black 25-litre lunch bag stands on a pale oak counter, front panel facing camera, label readable. A hand enters from frame right, grips the top handle with four fingers, lifts the bag ten centimetres and sets it down; the base meets the counter, the shadow compresses under it and the side panel flexes slightly under the load. Camera at counter height, 50mm, a slow 15-centimetre push-in; the label stays facing camera. Large soft window light from camera left, a narrow highlight along the coated zip, neutral color, fine grain. Sound: fabric settle, a soft base contact, no music. Preserve the bag's proportions, zip line, handle roots and label from the reference image.

## Check before submitting

- Is the product's geometry, label and material bound to actual evidence?
- Is there one action with a clear start, contact and end?
- Do hands enter, grip and leave in a readable way?
- Does the light reveal the material without changing its color?
- Is the camera move motivated and compatible with the product angle?
- Does the route support the requested duration, audio and reference?

## Sources and limits

[OpenAI's image prompting guide](https://developers.openai.com/api/docs/guides/image-prompting) recommends describing materials, lighting, color and visible detail rather than relying on mood words. [MiniMax H3's video prompt guide](https://huggingface.co/MiniMaxAI/MiniMax-H3/blob/main/docs/VIDEO_PROMPT_WRITING_GUIDE_ref_en.md) requires explicit composition, action, camera, sound and reference roles. [Seedance's prompting guide](https://docs.seedance.tv/en/seedance-2-complete-prompting-guide) distinguishes text, image and reference modes. These are prompt methods, not guarantees of product fidelity.
