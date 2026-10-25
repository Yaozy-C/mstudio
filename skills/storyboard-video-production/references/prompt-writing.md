# Convert shot design into a video prompt

Establish the shot in model-independent terms first, then adapt supported reference syntax and capabilities. Provider rewrite-output schemas are not automatically required submission formats. For difficult weight, contact or timing problems, consult [animation principles](shot-execution.md); apply only principles relevant to this event.

Use for video prompts, production handoff and confused-action diagnosis. Adapt mode, fields and syntax to the actual model. Describe the current shot; method guidance does not establish a tested generation result.

## Three different units

A script section describes a dramatic change; an editorial shot describes one viewpoint; a generation group describes a model call and occupies one Mstudio production shot record. They need not map one-to-one.

Each Mstudio video generation unit must be at least 5 seconds, or the selected model's higher minimum, and remain within its supported maximum. Meet that length by grouping adjacent, narratively compatible short shots into one generation task. Preserve every internal shot's complete description, exact duration, viewpoint, action and designed cut in the actual submitted prompt. Sub-second shots are valid; there is no 1-second floor. Read those relative time ranges and the unit span from the saved shot design; return structural grouping changes through the coordinator to direction. Use one task anchored to the unit, rather than a separate task for each internal shot. Preserve the selected continuous-take or cut intent; do not invent cuts to fill a group.

Combine a short final group with a compatible neighboring unit when the model's maximum allows. If the complete requested passage is itself shorter than 5 seconds or must remain isolated, generate a source of at least 5 seconds with clearly separated editing handles outside the intended passage. Keep the internal action timings and final-film runtime unchanged; do not add story events, repeats, slow motion or prolonged internal shots to fill the source. Reference-input minimum lengths are separate service constraints, not this output-unit rule.

Only when the selected model or input route cannot retain the planned cuts or required references, identify that concrete limitation and generate the affected parts separately with editing handles, handing the intended cut map to editing. Do not use this exception merely because the incoming plan has separate short records. Generation length does not determine selected edit length. Do not slow actions or lengthen the finished film to consume the minimum. Preserve this mapping in existing text and source ranges, without inventing schema fields.

## Design before conversion

Use the supplied shot constraints; consult [shot grammar](shot-execution.md) when the camera or staging remains ambiguous. Preserve the viewing experience and essential events while repairing execution. Use reference geometry and support compatible with the selected action. Return concrete spatial design conflicts to creative direction. Resolve input/wording issues within production without reopening the concept.

The model text carries the current scene relationships, key change, framing/camera/cuts that show it, necessary sound and identity constraints. Do not concatenate all storyboard columns. Separate subject movement from camera movement. Detail should remove ambiguity; brevity is not the goal if it deletes essential relationships.

A single event can contain multiple risky grasps, releases and occlusions. Choose readable staging, matching viewpoints and motivated cuts. Keep every intermediate phase needed to explain a change of contact, support, grip, constraint or possession; remove only preparation that changes none of these. User-required continuous proof remains visible. Leave unlocked performance freedom without simultaneously demanding a conflicting second-by-second schedule.

## Resolve the spatial shot

Before prose conversion, establish the initial camera location relative to the subject, viewing direction, framing, visible surfaces and occlusion. Then establish the subject's initial state/support, action path and resulting state. Separately establish the camera's start, physical path, speed and end relative to that action; distinguish camera travel from zoom. A continuous take must have a traversable path and a consistent view through openings.

Establish the source and path of visible light. Check spatial access, mechanism travel, clearances and object fit against the actual scene and product construction. Describe only product features visible from the chosen angle; never move outside merely to show more reference details unless the design calls for it.

Use compatible camera and action geometry. Do not silently combine an internal viewpoint with an external dive, or a fixed camera with incompatible travel. Preserve the user's clarified intent; if essential constraints remain incompatible, report the exact decision needed rather than submitting a guessed compromise. Keep planning commentary out of the model prompt.

## Motion prompt contract

For a short narrative clip, write one coherent paragraph or a small set of paragraphs that covers:

1. **Action sequence.** Designed events, hierarchy, causes and result, with speed, force, overlap and duration derived from the creative intent.
2. **Camera.** Start and end position, height, lens, movement type, amplitude and speed, coordinated with the action and intended viewing relationship.
3. **Performance.** Intention, available stimulus, attention, response intensity, latency and overlap, chosen for the actual interaction.
4. **Light and material.** The motivated source, what stays fixed, and the material response at the event.
5. **Sound and speech.** Exact line, language, speaker, delivery and the visible beat; ambient and material sounds with a cause.
6. **Reference roles.** Which input supplies identity, product, environment, motion or a control frame, and what it does not supply.

For image-to-video, the still already supplies appearance. Spend the prompt on motion, camera, performance and sound; do not re-describe the whole image or ask for a new grade. For a board, state the panel-to-state mapping and whether the result is one continuous take or the designed cuts.

Read [motion realism](motion-realism.md) for the failure table and stability rules. Do not answer a morphing face with more adjectives; reduce rotation, keep the face larger, use a selected identity input and shorten the action.

## Describe changes the viewer can actually see

Describe the temporal transformation and its physical causes. Start from the actual first visible state, describe the necessary intermediate physical states and their causes, and end in the requested state. Apply [contact and motion](shot-execution.md#contact-and-motion) to changes of support, grip, constraints and possession; put the resulting relationships into the submitted prompt, not only planning notes. If the clip starts mid-action, describe the ongoing phase rather than replaying preparation. Specify event dependencies, overlap and any purposeful holds from the selected design; use exact timing where it is required. Preserve exact user-locked beats without inventing impossible choreography.

Use only the dimensions relevant to this shot:

| Dimension | Make the model instruction observable |
|---|---|
| Subject and state | Name each actor/object consistently; describe where it starts, what part changes, the direction/path or pivot, relevant acceleration/contact, and where it ends. Separate translation, rotation, opening, deformation and changes in contents. Specify what stays fixed rather than asking the entire scene to transform. |
| Camera and screen motion | Describe camera movement separately from subject movement: start/end relation, trigger and path. State the intended change in screen position, scale, visibility or spatial relationship where it matters to the shot. Tracking a subject can keep it still in frame despite world motion; camera speed alone does not prove subject speed. A zoom, dolly, pan, orbit and product rotation are different changes. |
| Framing, visibility and focus | Track relevant entry/exit, crop, near-edge occlusion and what is revealed or stays hidden during movement. Preserve no-face constraints throughout. For a required focus pull, name the initial and final focus targets and its event cue; focus change is not camera travel. Depth of field must retain the relationship or proof the viewer needs. Do not add a rack focus by default. |
| Light, color and material | Establish the source and explain a requested change through an opening, moving object/source, reflection or explicit stylized rule. Distinguish a highlight traveling across a surface from the surface changing color. Keep unrelated scene illumination and identity stable. Derive each visible material response from the same physical event and its actual surface properties. Do not invent a light sweep or weather change to make a static scene busy. |
| Performance and environment | Keep the main event readable and derive secondary motion from physical causes and the selected performance. Establish who initiates and who reacts. Do not make every actor react together or add ambient motion, wind, particles or microgestures absent from the design. |
| Sound and speech | Preserve exact requested dialogue and language, speaker and relevant relation to action. Describe required action sound by source/material, onset and useful decay at its visible event; distinguish ambience, offscreen sound and music. A continuous background bed differs from an isolated impact. Respect requested silence and do not invent dialogue or score. Sound may motivate an offscreen event but cannot replace required visible proof. |

This table is an internal authoring aid, not six compulsory prompt headings. Write coherent shot prose with the necessary changes and locks. Do not describe mutually exclusive states as simultaneous or use a dissolve/morph to conceal an unexplained object transfer. Preserve deliberately selected surreal behavior with a consistent rule, cause and resulting state rather than treating fantasy as unrestricted drift.

## Motion and audio capability boundaries

A text description expresses intent. Actual first/last frames, motion references, trajectories, camera controls and native audio require support in the selected endpoint; ordinary reference images do not provide those controls. Use exposed parameters for duration, aspect and other technical specifications instead of inventing prompt commands. If audio, exact timing or a motion-control input is unavailable, preserve the requested relationship in the handoff and report the missing execution route; do not claim prose enabled it or silently switch services. Producing native speech through an authorized video-generation tool is part of video production; it does not require a standalone audio-generation tool. Include exact required dialogue or voice-over, language, speaker delivery and timing relative to action in the submitted prompt when native audio is supported. A speaker may remain offscreen. A requirement to add subtitles in editing does not authorize a no-speech or no-voice-over instruction. Keep a separately selected audio route when applicable; otherwise report a concrete endpoint limitation before handing required speech elsewhere. Unknown capability is not proof that native speech is unavailable. This method does not grant timeline-editing or standalone audio-generation tools.

A missing movement detail may use a coherent, reversible realization choice within the selected design. Conflicting essential viewpoint, action order or input state requires an explicit conflict report. Do not reopen the creative direction merely to fill every dimension above.

## Compile the current segment

Organize according to model capability; simple shots can be one paragraph:

- Scene, purpose and style, with the decisive event and camera relationship easy to identify.
- Actual references and their identity, layout, motion and camera roles; resolve appearance/motion conflicts.
- Single/multiple-shot structure, necessary cuts, viewpoint, camera-to-subject relationship, starting composition and action/result beats. A continuous take cannot secretly require a new viewpoint.
- Locks and permitted variation: camera relationships, direction, event order, identity/parts and required sound. Distinguish user constraints from director decisions without casually deleting the latter.
- Motion timing, force, contact response and performance intensity derived from the selected event, material and intended audience experience.

Exact times/frames express targets; editorial cuts establish precise final timing. Preserve the selected shot rules and performance in the prompt. If the physical chain cannot fit, resolve duration or the supported generation route within the selected design; do not meet a timestamp by deleting a necessary phase.

## Inputs and modes

Keep every user-designated generation reference as an actual input with its requested purpose. Select the minimum relevant set only for optional additional references, with roles matching actual upload order. Do not substitute prose or sampled stills for required media, or omit it because of placeholder appearance or style risk. If a concrete endpoint limit or missing source prevents use, report the conflict and leave that request unsubmitted until the user changes the requirement or resolves the limitation.

Input roles:

- Product identity: shape, material and parts; not all character blocking or scene layout.
- Scene: positions, scale, environment and camera direction without overriding original product facts.
- Action state: relevant poses before/after the event, identified as references or actual control frames.
- Actual first/last frames: use alignment language only when submitted through supported frame-control inputs.

For storyboard-driven generation attach the selected temporal board as the shot-planning reference. Do not attach unrelated product photos or boards from other shots. Add necessary scene references only within existing authorization; not every shot needs new images. Keep product/hand support and contact coherent. Ordinary references can express an end state but do not guarantee last-frame control. An actual first-frame control must not begin after the required action is already complete. Keep each input's subject, purpose and technical role consistent when adding or replacing it.

## Compile the board into motion

Begin by identifying the actual supplied board and its reading order. Describe panel 1 through panel 4 as temporal states, with the physically necessary transitions connecting each pair. Panels are sparse samples: infer and describe the intervening contact, force, support and release states from evidenced construction, rather than treating panel poses as the complete action. Do not merely say "follow the storyboard" or make four static holds. Describe entry/exit, support, release, contact and the actual end when relevant; retain subject/camera motion, material response and sound from the selected design.

For one continuous shot, explicitly request one full-screen uninterrupted take developing through the four states. The grid is planning information: the final video has no panel layout, gutters, board labels or slide transitions. Keep the fixed camera fixed; for a moving camera preserve the designed continuous path and world relationships. Do not call panels separate shots or use CUT at each panel boundary.

For an existing group of editorial shots, map cells to those shots and retain only its planned cut points. A four-panel board neither forces four cuts nor four equal durations. Use the selected duration, specified timing and feasible action pace. Missing state mapping or a visibly incorrect board is an input defect to return to image production, not something to hide with prose.

Preserve all required states in order without repeats, unauthorized cuts, grid leakage, morphing, teleportation, product drift or wrong endpoints. Do not claim uninterrupted motion from sampled frames. Do not claim storyboard conditioning is more reliable without comparable output evidence.

## Action-source failures

For transfers, pursuit and catches, compare actual reference position/support with the intended start. Prefer compatible references, a crop excluding the conflicting object, or a clear source insert joined to an accepted result, when consistent with the selected expression and authorization. Do not repeatedly add negations to the same contradictory image. If no suitable input exists, report the limitation and choose a feasible route.

Keep the initial location, path, contact and destination coherent; a correct ending does not excuse an impossible beginning. Permitted anthropomorphism is not unlimited floating or teleportation.

## Final prompt and handoff

Use one complete final prompt with consistent reference roles/order, input mode and supported duration. Check that every change in contact, support, grip, constraint or possession has a described cause and transition, including between board panels. Do not replay already-completed actions, contradict single/multiple-shot intent or require impossible timing. Do not invent inaccessible service rewriting.

A prompt-only request ends with saving. Existing same-scope authorization remains valid. Generate only within the requested scope using supported parameters. Repair a concrete defect without adding mandatory test generations or approval rounds.

Hand off shot/segment ID, intended change, essential event, inputs/order, actual mode, final prompt and essential constraints in existing records.

For live-action reactions and dialogue, read [naturalistic performance](shot-execution.md) when needed. For floaty, morphing or weightless results, use [motion realism](motion-realism.md) before adding more prompt detail.
