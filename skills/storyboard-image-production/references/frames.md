# Execute one temporal action board per shot

The director owns staging, camera path, events and revelation. Image production realizes that supplied design in visible states; it does not add a new event or cut to fill the board. Use [image prompting](prompt-writing.md) for wording and [frame rules](frame-checks.md) for visible constraints.

## Select the requested deliverable

A shot storyboard request produces one 2 by 2 action board per requested Mstudio shot record, using the same rule across the project. Do not silently replace it with a single representative image, a four-angle product sheet or four alternative concepts. Explicit standalone-image and exact first/last-frame requests retain their format. Prompt edits and Skill maintenance do not authorize generation.

## Replace a legacy deliverable without replaying it

A request naming a shot's storyboard, including "regenerate", uses the current board contract. Read the selected shot's design and reusable assets; treat old individual frame prompts and task keys as historical evidence. Produce one current board prompt/task for the requested shot rather than retrying every archived single-frame task. An old "needed frames" list names useful states, not the number of current image outputs.

Keep previous assets and task history. Report superseded pending tasks separately and do not submit both sets. Do not cancel, hide or delete unrelated work to make the task count look correct. An explicitly selected task retry retaining the old prompt/format remains an exact retry, not a covert redesign. A coordinator's multi-frame instruction does not establish that the user requested multiple independent images.

## Choose four moments from the actual design

Read shot ID, viewing purpose, event order, selected camera/focus, and continuous-versus-cut structure. Read the board top-left, top-right, bottom-left, bottom-right. The layout is a delivery convention, not a method for inventing the shots.

For a continuous take, select informative moments that communicate its development and necessary initial/result relationships. A changing composition or focus can matter even when a person remains still. A hold or nearby states can be valid; do not invent handling, effects or camera positions to differentiate cells. Preserve an intentional black opening and any essential reveal order.

For a record grouping editorial shots, choose representative moments from those shots in their designed order, preserving the cut map and the contribution of each viewpoint. Do not replace viewpoint choices and cut relationships with a generic operation-step sequence. An action phase is not automatically an editorial shot, and four cells do not require four cuts.

If four panels cannot cover the design's essential relationships or editorial shots, report the specific coverage gap; do not silently drop a shot, rewrite the sequence or subdivide the saved record. Keep the chosen workflow and scope while direction resolves a design/coverage conflict.

## Preserve shot structure and space

For one continuous take, all four panels are successive moments of that take. A fixed camera retains angle, crop, scale and static landmarks. A moving camera uses selected positions along its continuous path; preserve world geometry and plausible occlusion rather than forcing identical screen coordinates. Do not introduce an establishing angle or reverse shot unless already designed.

If one Mstudio record explicitly groups editorial cuts, map each panel to those existing shots/states and carry the cut map in the handoff. Four panels alone never mean four editorial shots. Keep duration once for the whole record; panels are not equal time slices and do not add generation duration.

Across panels and neighboring boards, preserve recurring identities, wardrobe, product construction, static prop positions, light direction and selected art direction. Track moving objects by initial support, path, contact and final support; change only the intended states. Retain original product evidence alongside generated anchors. A generated view cannot prove unseen hardware.

## Write and render the whole board

Request one image containing four equal panels, thin plain gutters and the explicit reading order. Each panel must have the target video's aspect ratio; a 2 by 2 layout of 9:16 cells also has an approximately 9:16 outer aspect ratio. Preserve the per-cell crop rather than squashing a wide shot into a vertical cell. Use supported resolution with readable contact and hardware inside each cell; do not invent model parameters.

Put shared identity, environment, camera rules and light once, then separately describe the visible state in each panel. A panel is a still, not a command to perform an entire action. Keep contact, opening, support, light path and viewpoint physically compatible. Use pose and location to show progress. Keep sound, trajectory and timing explanations in the handoff.

## Photographic direction and panel scale

Use [photographic direction](photographic-direction.md) to state the capture, lens, camera height, motivated light, palette and finish once. Each panel then needs one distinct visible state and one light/value decision; do not repeat the same adjectives in all four cells. For faces and hands, use [skin realism](skin-realism.md) only at the scale the panel can show.

At a 2 by 2 layout, a panel is one quarter of the output. If the board is 1672 by 941, each panel is about 836 by 470: enough for blocking, not for pore-level skin or precise fingers. Do not ask the board to prove detail that the panel cannot hold; state the limitation and propose an authorized full-frame look or control frame when that detail matters.

Check support surfaces and footwear across panels. A character's support surface can change unintentionally, and footwear can disappear at panel scale even when the rest of the wardrobe remains consistent. If the support surface or footwear is story-critical, use a layout sketch or generate the panels separately at full frame and composite; do not accept a board that seats the character on a different surface.

No burned-in panel numbers, captions, arrows, dialogue bubbles, watermarks or decorative borders by default. Deliberate onscreen copy from the selected script remains part of the appropriate state. Use plain gutters only to separate cells. Match the requested photographic or drawn style; polish does not compensate for unreadable action.

## Save and hand off

Supply the full board prompt to the generation-task tool and link its actual completed asset with mstudio_set_shot_frames. The task owns the prompt; shot.framePrompt is not writable through this interface. In existing project text or handoff record shot ID, asset ID, panel-to-state mapping, continuous/cut structure, camera rule, identity evidence and unresolved defects. Do not invent panel schema fields or link imaginary crops as separate assets.

The consuming video role receives the whole board as storyboard/shot-planning reference, not an exact starting or ending frame. The producer uses the selected endpoint's supported reference mode, limits and syntax. Do not crop or regenerate separate controls by default or switch formats silently if the endpoint cannot use a storyboard reference. A static board establishes planned states, not successful motion, timing or sound.
