# Transition kinds and engine limits

## Select by meaning

| Kind | Appropriate use and risk |
|---|---|
| Hard cut | Clear information, action continuation and direct rhythm; compare before adding an effect |
| `fade` | Time or emotional transition; overlapping complex images can obscure decisive action |
| `fadeblack`, `fadewhite` | Section boundaries; white needs a light or narrative motive and should avoid repeated harsh flashes |
| `slideleft`, `slideright`, `smoothleft`, `smoothright` | Coordinate direction with movement and subject entry; a filter, not continuous camera travel |
| `wipeleft`, `wiperight`, `circleopen`, `circleclose` | Deliberate graphic segmentation; real foreground occlusion needs masks and evidence, not automatic segmentation |
| `dissolve` | Grain-like dissolution, distinct from a smooth fade; for intentional graphic texture |

Choose the duration from movement and recognition needs. A few frames to roughly half a second is a common short-video range, not a prescription. A long dissolve can convey elapsed time but cannot hide a missing event, an axis mistake or an identity change.

## Custom design

`mstudio_set_transition(..., kind="custom", design={...})`: `design` combines `mask` (`uniform`, `linear` or `radial`), `angle`, `center:[x,y]`, `feather`, `curve`, `outgoingZoom` and `incomingZoom`, and `outgoingOffset` and `incomingOffset`. Positions and offsets are frame proportions; zoom spans 1 to 4. The outgoing image travels from its normal framing to its end state; the incoming image returns from its start state to normal framing. Sampling outside the frame extends edge pixels and can streak. Scaling is not optical motion blur.

`curve` holds 2 to 8 `[timeFraction, progressFraction]` points, starting `[0,0]` and ending `[1,1]`, with strictly increasing time and non-decreasing progress. Choose acceleration, holds, deceleration and the zoom centre from the actual action and composition, not from a fixed preset.

## Engine limits

- A transition occupies half its duration on each side of the cut. It needs adjacent full-frame opaque clips on the base picture track; overlays keep their order. Duration spans 0.05 to 3 seconds and cannot exceed either clip. A third overlapping clip invalidates the join; moving or deleting a neighbour stops rendering it.
- The engine uses available source handles first; at a source boundary it extends edge frames, which can create a hold. This is not optical-flow interpolation. Fast action may need more handles, a shorter effect or a hard cut.
- The application never silently moves clips, shortens the film or retimes captions. Visual transitions do not crossfade audio; J and L cuts and audio fades need separate authorised edits.

Inspect the actual composite for unwanted handle content, double subjects, direction jumps, edge artefacts and bounce-back, and the complete result for readable captions and audio continuity. Saved settings do not prove viewing.
