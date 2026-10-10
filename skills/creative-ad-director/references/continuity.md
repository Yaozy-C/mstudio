# State continuity and asset dependencies

Use for shot-state, frame and asset dependency rules. [Shot grammar](cinematography.md) owns camera and cutting rules; do not maintain a second photography checklist here.

## Shots, frames and generated clips

An editorial shot is a continuous viewpoint; a Mstudio shot record may group several editorial shots for generation. A frame is one selected moment; a generated clip is a tool output. Preserve internal shot labels/time ranges in the record text and map them to actual footage cuts. Multiple frames do not multiply duration; a generation group does not erase internal shots.

For important action, track source/initial support, path, necessary contact or boundary, and destination/final support. Record only the states required to understand the action; do not generate every state by default. A frame shows one readable instant, not mutually exclusive states at once. Necessary start/end frames retain shared landmarks. Image production executes the designed frames.

## State and dependency record

Reference existing project asset IDs and shot IDs for products, people, scenes and props. Describe continuity facts in the current shot and reference purposes; do not create a parallel subject or view registry.

| Asset | Stable properties | Permitted state changes |
|---|---|---|
| Product | Original structure, color, accessories, connections and supported proportions | Opening, orientation and material-appropriate deformation |
| Person | Identity, outfit, hands/accessories and project framing restrictions | Poses and actions consistent with the character |
| Scene | Landmarks, lighting direction and fixed-camera references | Motivated time/place changes |
| Props | Shape, count, packaging and contents | Insertion, removal or transfer with known start/end positions |

Track openings, counts, positions, support and relevant time/space changes. Transfers conserve item count unless the story explains a change. Framing may omit background or body regions, but not evidence the shot must prove.

Identify original product evidence, selected base frame, character/scene references and each input's role. Resolve conflicting references before generation. Generated base frames help continuity; hidden product structure still requires original evidence. A changed base must not silently alter unrelated assets or their selected design.

## Production handoff

A no-face project must control framing throughout action, camera movement and reflections, not only at the start. Live action needs appropriate handles; stop motion needs explicit state cuts; image-to-video needs a distinction between previews and actual control frames. A middle/end-state image is not automatically a valid action start. Production uses supported input controls.

## Source and shot relationships

Design handoff follows [shot constraints and handoff](cinematography.md#shot-constraints-and-handoff) in the existing shot text, without another camera/action/timing table.

During production map planned shot IDs to actual source ranges, observed events and keep/trim/discard/repair decisions. Mark missing planned shots instead of accepting an entire generation task as one successful shot. Editing uses these actual source relationships. Keep intended use duration, generated duration and selected source range distinct.
