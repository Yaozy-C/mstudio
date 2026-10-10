# Color grading

Translate the viewing goal into observable color choices. Correct problems before styling; a fixed filter is not a professional grade.

Read the current target, referenced footage and latest clips/visual. Thumbnails cannot establish exposure behavior, flicker or changing skin color across a clip. For matching, select a scene reference and compare subject brightness, neutrals, saturation and temperature across representative frames and neighboring shots.

Use mstudio_read_image(assetId,time) for source frames; with clipId, time is clip-relative and applies trimIn/speed and current grading. Do not request the exact endpoint. Use an in-range time only when a source frame is needed for the requested colour decision. Do not impose a before/after sampling sequence. Clip frames exclude transitions, overlays, subtitles and audio and do not prove continuous playback.

Use mstudio_set_clip_grade with direct parameters for exposure, tonal controls, eight-color HSL, curves and tonal wheels. Set only the required absolute parameters, preserving other values, timing, position, audio, captions and transitions. Use supplied current values and authoritative savedClips. Read only parameters missing for the edit; do not add a visual recheck stage.

Read groups through mstudio_read_clips and returned nextOffset. Known IDs use ids/fields. Batch independent reads and related edits. Choose one principal intention, such as natural product color or cooler atmosphere. Preserve truthful product/skin color and highlight detail; perceived continuity matters more than identical numeric values.

Read [tool limits](color-tools.md) before execution. Distinguish SDR grading from Log/HDR input transforms, RAW recovery, tracked masks and calibrated monitoring. Report concrete missing capabilities without pretending ordinary brightness adjustments supply them.

## Optional restrained live-action look

Use only when the project or reference calls for it. Preserve the actual character's skin tone and the scene's motivated light; do not normalize every face to one beige tone or make all scenes warm. A restrained realist look can use low-to-moderate saturation and contrast, subtle warm/cool separation, smooth highlights and readable shadows. Match subject exposure, skin hue and contrast across related shots while retaining justified day/night and location differences.

Do not lift all blacks until nighttime looks like daylight or remove intentional dramatic contrast. Highlight diffusion, halation and film grain are separate aesthetic choices, not automatic evidence of realism. Apply them only through supported controls when requested; ordinary grading parameters do not recreate physical black-mist filtration. Color correction cannot repair waxy skin geometry, invented pores, identity drift or unnatural acting: route those defects to image or video production with observed evidence. Do not claim an aesthetic result from parameter values alone.
