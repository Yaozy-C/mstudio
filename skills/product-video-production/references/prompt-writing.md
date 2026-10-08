# Convert shot design into a video prompt

Use the selected model's injected syntax and capabilities; the structure below organises the content, not the API fields. Examples illustrate design, not tested generation results.

## Design before conversion

Use the saved shot text per [shot design](../../creative-ad-director/references/shot-design.md#what-one-shot-record-holds); consult [shot grammar](../../creative-ad-director/references/cinematography.md) only when camera or staging is still ambiguous. Preserve the viewing experience and the essential events while repairing execution. Compare each reference's actual geometry and starting support with the design; director text does not prove image content. Return a spatial design conflict to direction with evidence; resolve input and wording issues here without reopening the concept.

For difficult weight, contact or timing, apply the relevant [animation principles](../../creative-ad-director/references/animation-principles.md). For live-action reactions and dialogue, read [naturalistic performance](../../creative-ad-director/references/naturalistic-performance.md).

## Grouping editorial shots

Meet the model's minimum clip length by grouping compatible short shots, not by making every cut last that long. Keep internal time ranges, viewpoints, actions and explicit hard cuts in the prompt: one minimum-length group can hold a 0-0.7s approach, a cut to a 0.7-1.5s overhead landing, and a cut to the result for the rest; these remain three editorial shots. Do not request both a continuous take and hard cuts. If the model cannot keep the cuts or the references, generate compatible parts separately with editing handles and assemble the cuts in the timeline. Generation length never sets the edit length.

## What the prompt carries

Organise by model capability; a simple shot can be one paragraph. Cover, in this order:

1. Scene, purpose and style, with the decisive event and the camera relationship easy to find.
2. The actual references and their visual roles: identity, layout, motion, camera; resolve any appearance-versus-motion conflict.
3. Single or multi-shot structure, the necessary cuts, the viewpoint and camera-to-subject relationship, the starting composition, and the action and result beats. A continuous take cannot secretly require a new viewpoint.
4. Locks and permitted variation: camera relationships, direction, event order, identity and parts, required sound. Keep the user's constraints and the director's decisions distinct; do not delete the latter casually.
5. Material-specific anticipation, speed curve, contact response, follow-through and reaction, instead of a slogan about cinematic motion.

Write subject movement and camera movement as separate sentences. Detail removes ambiguity; brevity is not a goal when it deletes an essential relationship. Do not concatenate every storyboard column. Omit only non-essential preparation; a user-required continuous proof stays visible. Leave unlocked performance freedom without also demanding a conflicting second-by-second schedule. Exact times express targets; editorial cuts set the final timing.

## Worked example: one generation group

The shot record for shot 2 of the lunch-bag piece asks for zipper travel, the lid lifting, a cut, and the container lifted out with clearance; the film keeps about 3 seconds. The selected model's minimum clip is 6 seconds and it supports internal cuts. An inspected start image exists (a-start-f0: hand already on the slider, bag closed), plus the product photo and the lead's identity asset.

Weak request, in the style that fails: one 15-second call covering all five shots of the film, two people, four lines of dialogue, every hand position ("steadies with the left hand, repositions the right hand, re-grips the slider"), and a closing block of twenty prohibitions ("no melting, no morphing, no extra fingers, no floating objects"). The model receives a shot list and a rulebook instead of one playable event; nothing in it can be inspected against a single intended action.

Compiled request for this group. The selected model's `single` mode takes exactly one input, the first frame, so the product photo and the identity asset are not attached here: they were consumed when frame a-start-f0 was generated and inspected, and the frame now carries them. On a model whose mode accepts appearance references, the same two assets would go in as `role=reference` with `mode=multi` and the purposes written in the frame example, and no first-frame control would exist.

```text
mode: single
references: [{assetId: a-start-f0, role: first-frame, purpose: "actual first frame: the lead's hand already on the slider of the closed blue lunch bag, close three-quarter view across the desk; carries the inspected product geometry and the lead's identity"}]
parameters: duration 6, aspect and resolution as locked by the user
prompt: Live action, office desk in soft window daylight. Continue from the first frame: the lead's right hand draws the slider in one short pull along the full top zipper while the left hand steadies the bag's side; the padded lid lifts and settles back on its rear seam, revealing a closed rectangular lunch container upright inside with clearance to the walls. The camera holds still through the pull and the lid settling. Cut to a slightly higher view over the opening: the same hand grips the container and lifts it vertically through the open top without touching the walls, then lowers it onto the clear desk beside the bag; the hand stays on it until the base is flat. The bag keeps its rectangular shape, both silver pulls stay at the open end of the track, the front zip pocket stays closed. Sound: zipper travel, the lid's fabric settling, one soft contact as the container base meets the desk; office room tone, no music.
```

Replace "Cut to" with the selected model's shot syntax when it has one. The decisive events are the pull, the lid settling, the clearance and the landing; the omitted preparation is not described and not forbidden, it is simply absent. The three locks from the shot record are stated as visible facts, not as a negative list. In the edit, only the ranges that show the pull and the clearance will be kept.

## Inputs and modes

Use the minimum relevant set, with roles matching the actual upload order:

- Product identity: shape, material and parts, not the whole original arrangement.
- Scene: positions, scale, environment and camera direction, without overriding product facts.
- Action state: the relevant pose before or after the event, declared as a reference or as an actual control frame.
- Actual first or last frames: use alignment language only when the input is submitted as `first-frame` or `last-frame`; an ordinary `reference` can express an end state but does not control the last frame.

Do not attach unrelated product photos or whole storyboard sheets. Add scene references only within existing authorisation; not every shot needs new images. After adding or replacing an input, recheck every role.

## Action-source failures

For transfers, pursuit and catches, compare the reference's actual position and support with the intended start. Prefer a compatible reference, a crop that excludes the conflicting object, or a clear source insert joined to an accepted result, when that fits the chosen expression and authorisation. Do not keep adding negations to the same contradictory image; if no suitable input exists, report it and choose a feasible route. Check from the source: initial location, path and contact, then destination. A correct second half does not cancel a wrong beginning; permitted anthropomorphism is not unlimited floating or teleportation.

## Before submitting

- The prompt describes one playable event per group, with the decisive contact or change and its result; preparation is absent, not forbidden.
- Every reference has a `role` the model supports and a `purpose` naming its subject and use; the prompt text uses the same subjects.
- Continuous take or explicit cuts, never both; the first frame, if any, does not already contain the action the prompt requests.
- Duration, aspect and resolution sit in parameters and match the user's locks; an unsupported lock is reported.
- Subject movement and camera movement are separate sentences; locks are stated as visible facts.
- The acceptance points are written down: which frames and which playback checks will decide keep, repair or change route.

A prompt-only request ends with saving. An authorised test states the concrete question it answers. Keep, repair or change route by the actual output, not by prompt completeness. Hand off the shot or segment ID, the intended change, the essential event, the inputs and their order, the actual mode, the final prompt and the acceptance points.
