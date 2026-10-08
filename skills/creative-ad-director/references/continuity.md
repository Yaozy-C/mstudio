# State continuity and asset dependencies

Use for state checks, frame preparation and asset-change tracking after shot design. [Shot grammar](cinematography.md) owns camera and cutting; this file owns what must stay the same between shots, frames and clips.

## Shots, frames and generated clips

An editorial shot is one continuous viewpoint; a Mstudio shot record may group several for generation; a frame is one chosen moment; a generated clip is a tool output. Keep internal shot labels and time ranges in the record text and map them to actual footage cuts. Several frames do not multiply duration; a generation group does not erase its internal shots.

For an important action, track source and initial support, path, necessary contact or boundary, and destination and final support. Record only the states needed to understand the action. A frame shows one readable instant, never two exclusive states. Necessary start and end frames share landmarks. Actual frame checks are in [frame inspection](../../image-production/references/frame-checks.md).

## State and dependency record

Reference products, people, scenes and props by existing asset IDs and shot IDs, and describe continuity facts in the shot text and the reference purposes. Do not create a parallel subject or view registry.

| Asset | Stable properties | Permitted state changes |
|---|---|---|
| Product | Original structure, colour, accessories, connections, proportions | Opening, orientation, material-appropriate deformation |
| Person | Identity, outfit, hands and accessories, project framing restrictions | Poses and actions consistent with the character |
| Scene | Landmarks, lighting direction, fixed-camera references | Motivated changes of time or place |
| Props | Shape, count, packaging, contents | Insertion, removal or transfer with known start and end positions |

Track openings, counts, positions, support and relevant time and space changes. Transfers conserve item count unless the story explains a change. Framing may omit background or body regions, never the evidence a shot must prove.

Identify the original product evidence, the selected base frame, the character and scene references, and each input's role; resolve conflicts before generation. Generated base frames help continuity, but hidden product structure still needs original evidence. When a base frame changes, revisit only the dependent assets it affects and keep versions clear.

## Production notes

A no-face project controls framing through the whole action, camera move and reflections, not only at the start. Live action needs handles; stop motion needs explicit state cuts; image-to-video needs a distinction between previews and actual control frames. A middle or end-state image is not automatically a valid start. Production verifies the available input controls.

## Shot execution evidence

During production, map each planned shot ID to the actual source ranges, the observed events and the keep, trim, discard or repair decision. Mark missing planned shots instead of accepting a whole generation task as one successful shot. Editors and reviewers share this mapping. Keep intended screen duration, generated duration and selected source range distinct.
