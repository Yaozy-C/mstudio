# State continuity and asset dependencies

Use for shot-state, frame and asset dependency rules. [Shot grammar](cinematography.md) owns camera and cutting rules; do not maintain a second photography checklist here.

## Shots, frames and generated clips

An editorial shot is a continuous viewpoint; a Mstudio shot record is one continuous shot. A frame is one selected moment; a generated clip is a tool output. Do not group several editorial shots into one record. If the scene needs a cut, save separate records and map each to its actual footage. Multiple frames do not multiply duration.

For important action, track source/initial support, path, necessary contact or boundary, and destination/final support. Record only the states required to understand the action; do not generate every state by default. A frame shows one readable instant, not mutually exclusive states at once. Necessary start/end frames retain shared landmarks. Image production executes the designed frames.

## The physical instant in a storyboard

Describe the shot's event and the selected frame moment separately in the existing shot text. A useful action description connects an initial state, a cause or interaction, and the resulting state. The frame brief identifies a particular phase of that event: before contact, during contact or movement, after release, or settled. A representative middle/end moment is not an opening control frame unless production explicitly uses it that way.

At that instant, specify only relationships that affect the intended action:

- Position and orientation relative to the product, surface or other subject; distinguish moving from settled when it matters.
- Contact and support: what touches, holds or bears the object, and whether release has happened yet.
- Possession and count: what each relevant hand holds, where transferred props are, and what has not entered yet.
- Mechanism state: what is closed, unfastened, partly open or fully open; which parts remain attached and which material may deform.

These are selection criteria, not mandatory fields for every frame. Direction supplies the story-relevant states and spatial relationships; production preserves visual identity, interprets the original component geometry and realizes the detailed physical transition. Do not freeze poses, visible product sides or screen coordinates to enforce consistency. Resolve a missing structural fact when the chosen interaction depends on it; uncertainty about an unrelated hidden part does not prohibit a new camera position.

Keep every depicted state compatible with the selected instant. Distinguish removal of a constraint from the force that causes the next movement, and identify the contact and support required at that instant.

When possession changes between states, identify the intervening transfer or exchange and the receiving support. Derive entry direction, fit and mechanism travel from product evidence. Video production realizes the necessary intermediate motion.

## Carry a state through a cut

For a same-instant change of viewpoint, preserve pose, contact gaps, grip, support, part connections and material state across the cut. Camera coordinates may change; the physical event must not advance or reset accidentally. For an editorial time omission, state what action/time is omitted and the resulting incoming state. A continuous shot cannot use that omission to conceal a missing transfer or force.

Hand off initial state, selected frame phase, essential change and outgoing state at the level needed to realize the shot. Image production depicts the chosen instant; video production builds the continuous causal path and final model prompt. Neither role should have to reconcile contradictory director states. Do not require a separate state ledger or an image for every transition.

## State and dependency record

Reference existing project asset IDs and shot IDs rather than creating a parallel subject or view registry. State intentional changes of identity, outfit, place, time or prop state when they matter to the sequence. Track the possession, count and spatial relationships needed to understand an exchange or reveal; framing may omit everything the shot does not need to show.

Pass relevant original product, character and scene references to production with their intended visual use. Image and video production own concrete input bindings, preservation of visible components and local generated repairs. A selected base establishes composition/state, not new product facts. A render that changes an accessory or silhouette needs an execution correction; a design that relies on an unsupported function needs a design/factual resolution. Keep these failures distinct.

## Production handoff

A no-face project must control framing throughout action, camera movement and reflections, not only at the start. Live action needs appropriate handles; stop motion needs explicit state cuts; image-to-video needs a distinction between previews and actual control frames. A middle/end-state image is not automatically a valid action start. A full-reference storyboard remains a sequence reference; it does not require additional start/end control images. Production preserves the selected input mode and uses only supported inputs.

## Source and shot relationships

Design handoff follows [shot constraints and handoff](cinematography.md#shot-constraints-and-handoff) in the existing shot text, without another camera/action/timing table.

During production map planned shot IDs to actual source ranges, observed events and keep/trim/discard/repair decisions. Mark missing planned shots instead of accepting an entire generation task as one successful shot. Editing uses these actual source relationships. Keep intended use duration, generated duration and selected source range distinct.
