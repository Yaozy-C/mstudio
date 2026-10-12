# State continuity and asset dependencies

Use for shot-state, frame and asset dependency rules. [Shot grammar](cinematography.md) owns camera and cutting rules; do not maintain a second photography checklist here.

## Shots, frames and generated clips

An editorial shot is a continuous viewpoint; a Mstudio shot record may group several editorial shots for generation. A frame is one selected moment; a generated clip is a tool output. Preserve internal shot labels/time ranges in the record text and map them to actual footage cuts. Multiple frames do not multiply duration; a generation group does not erase internal shots.

For important action, track source/initial support, path, necessary contact or boundary, and destination/final support. Record only the states required to understand the action; do not generate every state by default. A frame shows one readable instant, not mutually exclusive states at once. Necessary start/end frames retain shared landmarks. Image production executes the designed frames.

## The physical instant in a storyboard

Describe the shot's event and the selected frame moment separately in the existing shot text. A useful action description connects an initial state, a cause or interaction, and the resulting state. The frame brief identifies a particular phase of that event: before contact, during contact or movement, after release, or settled. A representative middle/end moment is not an opening control frame unless production explicitly uses it that way.

At that instant, specify only relationships that affect the intended action:

- Position and orientation relative to the product, surface or other subject; distinguish moving from settled when it matters.
- Contact and support: what touches, holds or bears the object, and whether release has happened yet.
- Possession and count: what each relevant hand holds, where transferred props are, and what has not entered yet.
- Mechanism state: what is closed, unfastened, partly open or fully open; which parts remain attached and which material may deform.

These are selection criteria, not mandatory fields for every frame. Keep stable identity separate from permitted state changes. Do not freeze all poses to enforce consistency or prescribe every video frame. Do not invent precise angles, paths, hinges or pocket openings that the product evidence does not establish. Resolve a missing structural fact when the chosen interaction depends on it; otherwise retain the uncertainty and use a supported action.

For example, a frame of a hand starting to lift a lid can show the closure released enough to permit lifting, the fingers supporting the raised edge, the lid still attached to the body, and the base resting on the table. It must not also claim the lid is fully open and the hand has left. Releasing a closure alone does not establish that the lid has swung open.

A frame after wiping may still show the cloth in the hand and an empty pocket. A later frame may show the stored item supported by the pocket after release. Name the intervening handoff or prop exchange when needed; do not call incompatible hand contents one uninterrupted gesture. Video production realizes the necessary intermediate motion. Pocket entry direction and fit must come from evidence, not from the convenient verb "drop".

## Carry a state through a cut

For a same-instant change of viewpoint, preserve pose, contact gaps, grip, support, part connections and material state across the cut. Camera coordinates may change; the physical event must not advance or reset accidentally. For an editorial time omission, state what action/time is omitted and the resulting incoming state. A continuous shot cannot use that omission to conceal a missing transfer or force.

Hand off initial state, selected frame phase, essential change and outgoing state at the level needed to realize the shot. Image production depicts the chosen instant; video production builds the continuous causal path and final model prompt. Neither role should have to reconcile contradictory director states. Do not require a separate state ledger or an image for every transition.

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
