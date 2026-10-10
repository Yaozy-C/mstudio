# Design the join

## Select by meaning

| Type | Appropriate use and risk |
|---|---|
| Hard cut | Clear information, action continuation and direct rhythm; compare before adding an effect |
| fade | Time or emotional transition; overlapping complex images can obscure decisive action |
| fadeblack / fadewhite | Section boundaries; white needs a light/narrative motive and should avoid repeated harsh flashes |
| slideleft/right, smoothleft/right | Coordinate direction with movement and subject entry; this is a filter, not continuous camera travel |
| wipeleft/right, circleopen/close | Deliberately graphic segmentation; real foreground occlusion requires masks/evidence, not unavailable automatic segmentation |
| dissolve | Grain-like dissolution, distinct from smooth fade; use for intentional graphic texture |

Choose duration from movement and recognition needs. A few frames to roughly half a second can be a short-video comparison range, not a universal prescription. Long dissolves can convey elapsed time but cannot hide missing events, axis mistakes or identity changes.

## Execution semantics

Use set_transition(kind="custom",design={...}) for a custom design. design combines mask (uniform, linear or radial), angle, center:[x,y], feather, curve, outgoingZoom/incomingZoom and outgoingOffset/incomingOffset. Positions/offsets are frame proportions; zoom spans 1–4. The outgoing image travels from its normal framing to the end state; the incoming image returns from its start state to normal framing. Sampling outside the frame extends edge pixels and can create streaks. Scaling is not optical motion blur.

curve has 2–8 [timeFraction,progressFraction] points, beginning [0,0] and ending [1,1], strictly increasing time and nondecreasing progress. Choose acceleration/holds/deceleration and zoom center from actual action/composition, not a fixed custom preset.

A transition occupies half its duration on either side of the cut. Current support is adjacent full-frame opaque clips on the base picture track, with overlays retaining their order. Duration spans 0.05–3 seconds and cannot exceed either clip's duration. A third overlapping clip invalidates the join. Moving/deleting adjacency stops rendering the old join.

Use available source handles first; at source boundaries the engine extends edge frames, potentially creating a hold. This is not optical-flow interpolation. Fast action may need more handles, a shorter effect or a hard cut. The application does not secretly move clips, shorten the film or retime captions. Visual transitions do not crossfade audio; J/L cuts and audio fades need separate authorized edits.

Keep transition handles free of unrelated action. Avoid double subjects, direction jumps, edge artifacts and bounce-back; preserve readable captions and continuous audio. Do not claim viewing from saved settings.
