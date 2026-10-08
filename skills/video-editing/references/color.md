# Colour grading

Translate the viewing goal into observable colour choices. Correct problems before styling; a fixed filter is not a professional grade.

## Read the footage first

Read the current target, the referenced footage and the latest `clips[].visual` with `mstudio_inspect(section=clips, ids, fields=[visual, assetId, trimIn, trimOut, speed])`, following `nextOffset` for groups. Thumbnails cannot establish exposure behaviour, flicker or changing skin colour across a clip.

Sample source frames with `mstudio_read_image(assetId, clipId, time)`: with `clipId`, `time` is clip-relative and already applies trim, speed and the current grade. Do not request the exact endpoint. Start with the beginning, the middle and near the end, then add samples where light or subjects change, and compare the same times after editing. Clip frames exclude transitions, overlays, captions and audio and do not prove continuous playback. Actual pixels are required before describing the image.

For matching, choose a scene reference and compare subject brightness, neutrals, saturation and temperature across representative frames and neighbouring shots.

## Grade

Set the grade through `mstudio_update_clip(id, visual={grade:{...}})` for exposure, tonal controls, eight-colour HSL, curves and tonal wheels; read [tool limits](color-tools.md) before execution. Set only the required absolute parameters and preserve the other values, timing, position, audio, captions and transitions. Verify `savedClips`, inspect only missing or conflicting parameters, then recheck the graded pixels at matching times. A saved parameter does not prove the render was viewed.

Choose one principal intention, such as natural product colour or a cooler atmosphere. Preserve truthful product and skin colour and highlight detail; perceived continuity matters more than identical numbers. Distinguish SDR display grading from Log or HDR input transforms, RAW recovery, tracked masks and calibrated monitoring, and report a missing capability instead of pretending ordinary adjustments supply it.

## Worked example: matching two desk clips

Clip c7 (asset a31, trimIn 0.2, trimOut 3.6) follows c6 (asset a28) at the same desk. Frames read at c7 times 0.3, 1.5 and 3.0 and at c6 times 0.5 and 2.5 show c7 roughly half a stop darker, with a green cast on the neutral wall and the blue fabric reading slightly cyan. The bag's blue must stay the product's blue.

```text
mstudio_update_clip(id=c7, visual={grade:{exposure:0.45, tint:-8}})
```

Only `exposure` and `tint` are sent, so the other grade values stay as they were. After `savedClips` confirms, reread c7 at 0.3, 1.5 and 3.0: the wall neutrals now sit with c6, and the fabric blue matches a28 without an HSL change. If the fabric had drifted, the next step would be the blue row of `hsl` only, not a global saturation change. The join itself is checked by [transitions](transitions.md); the full export still needs playback for flicker.

## Optional restrained live-action look

Use only when the project or the reference calls for it. Keep each character's skin tone and the scene's motivated light; do not normalise every face to one beige tone or make every scene warm. A restrained realist look can use low-to-moderate saturation and contrast, subtle warm and cool separation, smooth highlights and readable shadows. Match subject exposure, skin hue and contrast across related shots while keeping justified day, night and location differences.

Do not lift all blacks until night looks like day or remove intentional dramatic contrast. Highlight diffusion, halation and film grain are separate aesthetic choices, not evidence of realism, and ordinary grading parameters do not recreate physical filtration. Colour correction cannot repair waxy skin, invented pores, identity drift or unnatural acting: route those to image or video production with the observed evidence.
