# Grading decisions and tool limits

Use update_clip.visual.grade for new work. Exposure spans plus/minus 3 EV. Temperature, tint, contrast, saturation, vibrance, highlights, shadows, whites, blacks and tonal balance span plus/minus 100. The tool includes eight-color HSL, master/R/G/B curves and shadow/midtone/highlight wheels. Only supplied fields change; grade:null clears custom grading only. The source is not redrawn; a cached 33-cubed LUT is shared by frame extraction, native preview, transitions and export.

HSL order is red, orange, yellow, green, cyan, blue, purple, magenta; each row is [hueOffset,saturation,lightness]. Curves are master/R/G/B with three y values at x=.25/.5/.75, in 0–1 and nondecreasing. Wheels are shadows/midtones/highlights with [hue 0–360,saturation 0–100,lightness -100–100]. Arrays replace the whole group, so read existing values first. Curves are fixed-point piecewise-linear, not free Bezier curves.

Legacy brightness, contrast, saturation, temperature and effect still run after the custom grade for old projects. Avoid accidental double exposure/contrast adjustments. This is creative SDR display-RGB grading, not camera Log/HDR input conversion, RAW development, full color management or tracked masking. Lowering exposure cannot recover already clipped source detail.

## Choose from observed problems

- Correction: assess black/white points and neutrals while preserving intentional lighting. Global brightening can lift blacks without solving a backlit subject.
- Matching: compare subject luminance/hue around cuts against a reference. Indoor/outdoor scenes need not share identical white balance.
- Style: choose a color tendency and contrast relationship while retaining product/skin readability. Monochrome or vintage looks are deliberate choices; vignettes are not automatic.
- Verification: inspect highlight/shadow clipping, color casts, cut-to-cut brightness changes and compression banding. Unseen final renders remain unchecked.

For a slightly cooler image with preserved product color, compare neutrals and the product, adjust temperature modestly, and compensate affected color ranges only if needed. HSL selects colors throughout the image, not a semantic product mask. Values depend on the actual frames.
