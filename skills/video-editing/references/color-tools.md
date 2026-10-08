# Grading decisions and tool limits

Use `mstudio_update_clip(id, visual={grade:{...}})` for new work. `exposure` spans plus or minus 3 EV. `temperature`, `tint`, `contrast`, `saturation`, `vibrance`, `highlights`, `shadows`, `whites`, `blacks` and `balance` span plus or minus 100. The grade also holds eight-colour HSL, master and R, G, B curves, and shadow, midtone and highlight wheels. Only supplied fields change; `grade:null` clears the custom grade only. The source is never redrawn; one cached LUT is shared by frame extraction, native preview, transitions and export.

- `hsl`: eight rows in the order red, orange, yellow, green, cyan, blue, purple, magenta; each row is `[hueOffset, saturation, lightness]`.
- `curves`: four rows for master, R, G, B; each row holds the three y values at x = 0.25, 0.5 and 0.75, in 0 to 1 and non-decreasing. These are fixed-point piecewise-linear curves, not free Bezier curves.
- `wheels`: three rows for shadows, midtones, highlights; each row is `[hue 0 to 360, saturation 0 to 100, lightness -100 to 100]`.

Arrays replace the whole group, so read the existing values first. The legacy `brightness`, `contrast`, `saturation`, `temperature` and `effect` fields still run after the custom grade in old projects; avoid an accidental double exposure or contrast adjustment. This is creative SDR display grading, not Log or HDR conversion, RAW development, full colour management or tracked masking. Lowering exposure cannot recover clipped source detail.

## Choose from observed problems

- Correction: assess black and white points and neutrals while preserving intentional lighting. Global brightening lifts blacks without solving a backlit subject.
- Matching: compare subject luminance and hue around cuts against a reference. Indoor and outdoor scenes need not share one white balance.
- Style: choose a colour tendency and contrast relationship while keeping product and skin readable. Monochrome or vintage looks are deliberate choices; vignettes are not automatic.
- Verification: inspect highlight and shadow clipping, colour casts, cut-to-cut brightness changes and compression banding. An unseen final render stays unchecked.

Example: for a slightly cooler image with preserved product colour, compare the neutrals and the product, adjust `temperature` modestly, and compensate the affected HSL ranges only if needed. HSL selects colours throughout the frame, not a semantic product mask; values depend on the actual frames.
