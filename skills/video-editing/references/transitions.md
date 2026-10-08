# Transition design

Decide what the cut communicates before adding an effect. A direct cut often best preserves continuous action; choosing no effect is a valid professional decision.

## Read both sides of the join

Read the specified clips and the current clips, tracks, source ranges, speed, existing join, grade and sound. Inspect the outgoing end and the incoming start for motion direction, shot size, attention, composition and brightness. Without readable footage, do not claim matching action, beat alignment or a seamless join.

Sample with `mstudio_read_image(assetId, clipId, time)`, where `time` is clip-relative: two frames before the outgoing endpoint and two near the incoming start; the exact endpoint is unreadable. These frames include the grade but not the composite transition. After an edit, call `mstudio_read_image` with the incoming clip's `assetId` and `clipId` and `transition:true`; `time` then means seconds from the transition start. Inspect the actual beginning, middle and end of the composite. The preview cache invalidates on parameter changes. These frames omit overlays, captions and sound and do not establish full playback.

## Fix the action before the effect

To change the action phase use `mstudio_slip_clip`; for pace `mstudio_retime_clip`; for position `mstudio_move_clip`. A longer dissolve cannot repair an incorrect action match.

## Apply

Call `mstudio_set_transition(id, fromClipId, kind, duration, design)` with `fromClipId` for the outgoing clip and `id` for the incoming one. Simple supported kinds stay appropriate; `kind:custom` with `design` gives masks, progress curves, zoom and offsets; `kind:null` removes the join. The edit affects this join only, not other clips, narration or captions. Verify `savedClips`, read only missing or conflicting parameters, then inspect the composite. Batch independent reads and confirmed edits for the same group.

## Worked example: two joins in the lunch-bag piece

Join c6 to c7: the lid settles open at the end of c6 (frames at c6 2.9 and 3.3 s show the lid at rest, attention on the opening); c7 starts with the hand already gripping the container (frames at 0.1 and 0.4 s). Action continues across the cut and attention stays in place, so the decision is a hard cut: no call, and the report says why.

Join c9 to c10: c9 ends on the bag closed on the desk; c10 opens on the same bag carried out of the room, later the same day. A time jump that the viewer should feel without a hard bump:

```text
mstudio_set_transition(id=c10, fromClipId=c9, kind="fade", duration=0.4)
```

Both clips are full-frame on the base track with at least 0.2 s of handle on each side, so the engine has source frames to blend. After `savedClips` confirms, read the composite with `mstudio_read_image(assetId=<c10 asset>, clipId=c10, transition=true, time=0.05 / 0.2 / 0.35)`: no double bag, no direction jump, the caption on c10 still readable. Audio under the fade is not crossfaded by this call; if the room tone bumps, that is a separate caption or audio edit to request.

Read [selection and engine limits](transition-tools.md) before choosing a kind. Give a concise reason and the relevant cut to review. The name of an effect is not actual camera motion or optical flow, and saved parameters are not visual acceptance. Respect a specified effect while stating the concrete source or tool limitations.
