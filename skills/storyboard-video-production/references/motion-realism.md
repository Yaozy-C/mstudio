# Motion realism and performance

Use for narrative video prompts, especially image-to-video and board-to-motion work. A clip is a performance in time, not a slideshow of stills. The image or board already supplies appearance; the prompt must supply cause, weight, timing, camera behavior and sound.

## Motion brief

Decide these before writing:

1. **Subject action.** The starting state, the decisive change, the ending state, and who or what causes it.
2. **Camera.** Start and end position, height, lens, movement type, amplitude and speed. A camera move is not a substitute for subject action.
3. **Focus and visibility.** What stays sharp, what enters or leaves frame, what remains hidden.
4. **Light and atmosphere.** The source that motivates a change, and the color/air relationship that stays stable.
5. **Sound and speech.** Exact dialogue or voice-over, language, speaker, delivery and the visible event it belongs to; ambient and material sounds with a cause.
6. **Reference roles.** Which input supplies identity, product geometry, environment, motion or a control frame, and what it does not supply.

## Make motion observable

- One dominant action per short generation, with a readable start and end. A second simultaneous action competes with it.
- Write cause and effect, not labels: "her weight shifts to the front foot, the plank flexes slightly, the cloth settles after she stops".
- Give contact a consequence: a hand presses into fabric, a foot plants and the body's weight follows, a lid resists then gives way.
- Use a speed curve where it matters: anticipation, acceleration, contact, follow-through, settle. Do not describe every action as "smooth".
- Add only motivated secondary motion: hair lags when the head turns, a shirt hem settles after the body stops, a cable swings after the stand is moved.
- Performance is reaction latency and attention, not a facial-expression label. Establish what the person notices, when the eyes move, when the breath or blink happens, and what the mouth does before or after the line.
- Keep the camera still or simple when the action is the subject. A floating orbit or a slow push that does not change the information makes the shot feel synthetic.

## AI motion failures

| Failure | Usual cause | Repair |
|---|---|---|
| Face morphs or changes identity | Too much head rotation, small face, no identity reference | Keep the face larger and more stable; reduce rotation; use a selected identity frame; shorten the clip |
| Hands mutate or gain fingers | Complex hand action, hands near face, occlusion | Simplify the gesture, keep hands visible against a clean background, show grip and release |
| Feet slide or weight floats | Walking or turning without planted contact | Describe the plant, weight transfer and ground contact; cut before the step if it is not needed |
| Uniform, floaty speed | "Smooth cinematic motion" with no beats | Give anticipation, acceleration, contact and settle; hold the camera |
| Background warps or crowds clone | Complex background motion, many people, camera travel | Reduce camera movement, keep background simple, give a fixed count and few distinct people |
| Mouth does not match speech | Dialogue not assigned to a speaker or timing | Put exact line, language, speaker and action cue in the prompt; keep the face readable |
| Extra cut or grid leak | Board treated as four shots or as a layout | State one continuous take or the exact designed cuts; ask for full-screen output with no board layout |
| Light or grade shifts mid-clip | Vague "cinematic" grade, changing source | Name the light source and keep it fixed; describe one motivated change only |
| Product/wardrobe drift | No product reference or too many competing actions | Bind product geometry and wardrobe explicitly; keep the action simple |

## Image-to-video and board-to-motion

An image already fixes composition, identity, light and color. The prompt should mostly describe motion, camera, performance and sound; do not re-describe the entire still. For a board, say which panel maps to which state and whether the result is one continuous take or the designed cuts. Never ask the model to preserve the grid.

Keep the camera move compatible with the supplied frame. A first frame that looks at a face from the front does not support a continuous 180-degree orbit without new information. If the action needs a view the source cannot support, return the input gap rather than hiding it with prose.

## Performance direction

Describe attention, not emotion labels:

- "She hears the clap off-screen; her eyes move first, then her chin, then a small smile; the breath is still held."
- "He starts to turn away, stops with his weight on the back foot, and only then looks back."
- "She claps once, watches the older woman, and claps again on the next beat; the second clap is softer."

Avoid "she is happy then surprised then moved" in a five-second clip. That is three performances without causes.

## Model-aware checks

Use the selected model's injected rules for limits and inputs. Check these before submitting:

- Does the model support a first frame, last frame or only text/reference?
- Is native audio available, and does the requested line fit the duration?
- What is the supported duration and aspect, and does the action fit it?
- Does the route accept the whole board as a reference, or only a single image?
- Does the prompt rely on a control the route does not expose?

Do not copy another model's syntax into a route that does not support it. A prompt phrase does not create a control the endpoint lacks.

## Worked five-second prompt

> One continuous 5-second take, no cuts. A woman in a rust-red linen shirt stands at a microphone on a low timber platform at dusk, hand resting on the stand. She hears a clap from the audience; her eyes move first, then her head turns down-left, and a small smile arrives after a beat. Her fingers release the microphone one by one and her hand floats a few centimetres away. The camera stays at seated eye level, 50mm, with a very slow 10-centimetre push-in; it does not orbit. Warm sand bounce on the face, cool dusk ambient in the shadow; the sea stays dark slate. Sound: one clear clap, a soft breath, distant waves, no music. Preserve her face, hair, shirt and trousers from the reference frame.

## Check before submitting

- Is there one dominant action with a cause and a visible result?
- Does the camera move have a reason, an amplitude and a speed?
- Are contact, weight and secondary motion described where they matter?
- Is the performance written as attention and timing, not emotion labels?
- Are speech, speaker and sound events tied to visible moments?
- Does the route actually support the requested frame, duration and audio?

## Sources and limits

[MiniMax H3's video prompt guide](https://huggingface.co/MiniMaxAI/MiniMax-H3/blob/main/docs/VIDEO_PROMPT_WRITING_GUIDE_ref_en.md) requires composition, subject position, environment, lighting, action, camera movement, sound and reference roles to be explicit in the detailed description. [Seedance's prompting guide](https://docs.seedance.tv/en/seedance-2-complete-prompting-guide) distinguishes text-to-video, image-to-video and reference-to-video, and treats image-to-video prompts as primarily motion direction. [Google's Veo guide](https://ai.google.dev/gemini-api/docs/veo) documents first/last-frame and native-audio routes. These are method references, not guarantees of motion quality or model behavior.
