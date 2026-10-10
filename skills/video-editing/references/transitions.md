# Transition design

Determine what the cut communicates before adding an effect. A direct cut may best preserve already continuous action. Choosing no effect is a valid professional decision.

Use the supplied target clip IDs, source ranges, speed, join, grading and sound. Retrieve only missing edit inputs. Outgoing and incoming phases must retain coherent motion direction, attention, composition and brightness; do not claim unsupported beat alignment or seamless action.

Use mstudio_read_image(assetId,clipId,time), where time is clip-relative. The exact clip endpoint is unreadable; use an in-range time when a frame is needed. Clip frames include grading, not the composite transition. For a requested transition preview, use the incoming clip's assetId/clipId with transition:true; time means seconds from transition start. Preview cache invalidates on parameter changes. These frames omit overlays, captions and sound and do not establish full playback.

To change the action phase use slip_clip; for pace use retime_clip; for position use move_clip. A longer dissolve cannot repair an incorrect action match.

Apply set_transition with fromClipId for outgoing, id for incoming, kind and total duration. Use kind:custom with design for masks, progress curves, zoom and offsets; simple supported types remain appropriate. kind:null removes it. Edits affect this join, not other clips, narration or captions. Use savedClips as the committed edit; retrieve only missing parameters.

Batch independent reads and confirmed edits for the same group. Read [selection and engine limits](transition-tools.md). Give a concise reason and the affected cut ID. The name of an effect is not actual camera motion or optical flow, and saved parameters are not visual acceptance. Respect a specified effect while stating concrete source/tool limitations.
