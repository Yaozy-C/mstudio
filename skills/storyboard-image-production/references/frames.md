# Execute one temporal action board per shot

The director owns staging, camera path, events and revelation. Image production realizes that supplied design in visible states; it does not add a new event or cut to fill the board. Use [image prompting](prompt-writing.md) for wording and [frame rules](frame-checks.md) for visible constraints.

## Select the requested deliverable

A shot storyboard request produces one 2 by 2 action board per requested Mstudio shot record, using the same rule across the project. Do not silently replace it with a single representative image, a four-angle product sheet or four alternative concepts. Explicit standalone image, reference-asset and exact first/last-frame requests remain those deliverables. Prompt edits and Skill maintenance do not authorize generation.

## Replace a legacy deliverable without replaying it

A request naming a shot's storyboard, including "regenerate", uses the current board contract. Read the selected shot's design and reusable assets; treat old individual frame prompts and task keys as historical evidence. Produce one current board prompt/task for the requested shot rather than retrying every archived single-frame task. An old "needed frames" list names useful states, not the number of current image outputs.

Keep previous assets and task history. Report superseded pending tasks separately and do not submit both sets. Do not cancel, hide or delete unrelated work to make the task count look correct. An explicitly selected task retry retaining the old prompt/format remains an exact retry, not a covert redesign. A coordinator's multi-frame instruction does not establish that the user requested multiple independent images.

## Choose four states from the actual design

Read shot ID, event order, start/end, continuous-versus-cut structure, chosen camera, product evidence and viewing purpose. Define the board's reading order as top-left, top-right, bottom-left, bottom-right.

| Panel | State to depict |
|---|---|
| 1 | Actual opening state, before the required change; preserve an intentional black start. |
| 2 | First informative action phase: entry, approach, opening or initiation selected in the shot. |
| 3 | Decisive visible phase: contact, crossing, reveal or changed relationship selected in the shot. |
| 4 | Actual ending arrangement or response, with visible support and relevant clearances. |

These are state-selection questions, not four compulsory new actions. A simple action may use two nearby phases or a planned hold; do not fabricate extra handling, effects or camera angles. Do not repeat four completed poses when the design needs a visible transition. Preserve surprise order. If the supplied design has too many essential changes for four readable panels, report the specific coverage gap to direction; do not silently drop proof or subdivide existing shot records.

## Preserve shot structure and space

For one continuous take, all four panels are successive moments of that take. A fixed camera retains angle, crop, scale and static landmarks. A moving camera uses selected positions along its continuous path; preserve world geometry and plausible occlusion rather than forcing identical screen coordinates. Do not introduce an establishing angle or reverse shot unless already designed.

If one Mstudio record explicitly groups editorial cuts, map each panel to those existing shots/states and carry the cut map in the handoff. Four panels alone never mean four editorial shots. Keep duration once for the whole record; panels are not equal time slices and do not add generation duration.

Across panels and neighboring boards, preserve recurring identities, wardrobe, product construction, static prop positions, light direction and selected art direction. Track moving objects by initial support, path, contact and final support; change only the intended states. Retain original product evidence alongside generated anchors. A generated view cannot prove unseen hardware.

## Write and render the whole board

Request one image containing four equal panels, thin plain gutters and the explicit reading order. Each panel must have the target video's aspect ratio; a 2 by 2 layout of 9:16 cells also has an approximately 9:16 outer aspect ratio. Preserve the per-cell crop rather than squashing a wide shot into a vertical cell. Use supported resolution with readable contact and hardware inside each cell; do not invent model parameters.

Put shared identity, environment, camera rules and light once, then separately describe the visible state in each panel. A panel is a still, not a command to perform an entire action. Keep contact, opening, support, light path and viewpoint physically compatible. Use pose and location to show progress. Keep sound, trajectory and timing explanations in the handoff.

No burned-in panel numbers, captions, arrows, dialogue bubbles, watermarks or decorative borders by default. Deliberate onscreen copy from the selected script remains part of the appropriate state. Use plain gutters only to separate cells. Match the requested photographic or drawn style; polish does not compensate for unreadable action.

## Save and hand off

Save the full board prompt in shot.framePrompt and link its one actual completed image asset through shot.frames. In existing project text or handoff record shot ID, asset ID, panel-to-state mapping, continuous/cut structure, camera rule, identity evidence and unresolved defects. Do not invent panel schema fields or link imaginary crops as separate assets.

The consuming video role receives the whole board as storyboard/shot-planning reference, not an exact starting or ending frame. The producer uses the selected endpoint's supported reference mode, limits and syntax. Do not crop or regenerate separate controls by default or switch formats silently if the endpoint cannot use a storyboard reference. A static board establishes planned states, not successful motion, timing or sound.
