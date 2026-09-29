---
name: color-grading
description: Correct color, develop a look and match specified clips using actual visible footage and local grading tools while preserving source media and edits.
---

# Color grading

Translate the viewing goal into observable color choices. Correct problems before styling; a fixed filter is not a professional grade.

Read the current target, referenced footage and latest clips/visual. Thumbnails cannot establish exposure behavior, flicker or changing skin color across a clip. For matching, select a scene reference and compare subject brightness, neutrals, saturation and temperature across representative frames and neighboring shots.

Use mstudio_read_image(assetId,time) for source frames; with clipId, time is clip-relative and applies trimIn/speed and current grading. Do not request the exact endpoint. Start with beginning, middle and near-end, then add samples where light or subjects change. Compare the same times after editing. Actual pixels are required before describing the image. Clip frames exclude transitions, overlays, subtitles and audio and do not prove continuous playback.

Use update_clip.visual.grade for exposure, tonal controls, eight-color HSL, curves and tonal wheels. Set only the required absolute parameters, preserving other values, timing, position, audio, captions and transitions. Read latest revision before editing. Verify savedClips; inspect again if the receipt is incomplete or unclear, then inspect the graded pixels at matching times. A saved parameter does not prove the full render was viewed.

Read groups through inspect section=clips and returned nextOffset. Known IDs use ids/fields. Batch independent reads and confirmed edits without skipping visual evidence. Choose one principal intention, such as natural product color or cooler atmosphere. Preserve truthful product/skin color and highlight detail; perceived continuity matters more than identical numeric values.

Read [tool limits](references/workflow.md) before execution. Distinguish SDR grading from Log/HDR input transforms, RAW recovery, tracked masks and calibrated monitoring. Report concrete missing capabilities without pretending ordinary brightness adjustments supply them.
