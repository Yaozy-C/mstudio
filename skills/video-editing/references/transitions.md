# Transition design

Determine what the cut communicates before adding an effect. A direct cut may best preserve already continuous action. Choosing no effect is a valid professional decision.

Read the specified clips and current clips/tracks, source ranges, speed, join, grading and sound. Inspect the outgoing end and incoming start for motion direction, shot size, attention, composition and brightness. Without readable footage, do not claim matching action, beat alignment or a seamless join.

Use mstudio_read_image(assetId,clipId,time), where time is clip-relative. Sample two frames before the outgoing endpoint and two near the incoming start; the exact endpoint is unreadable. Clip frames include grading, not the composite transition. After an edit, use the incoming clip's assetId/clipId with transition:true; time then means seconds from transition start. Inspect actual beginning/middle/end composites. Preview cache invalidates on parameter changes. These frames omit overlays, captions and sound and do not establish full playback.

To change the action phase use slip_clip; for pace use retime_clip; for position use move_clip. A longer dissolve cannot repair an incorrect action match.

Apply set_transition with fromClipId for outgoing, id for incoming, kind and total duration. Use kind:custom with design for masks, progress curves, zoom and offsets; simple supported types remain appropriate. kind:null removes it. Edits affect this join, not other clips, narration or captions. Verify savedClips and read only missing or conflicting parameters before visual rechecking.

Batch independent reads and confirmed edits for the same group. Read [selection and engine limits](transition-tools.md). Give a concise reason and the relevant cut to review. The name of an effect is not actual camera motion or optical flow, and saved parameters are not visual acceptance. Respect a specified effect while stating concrete source/tool limitations.
