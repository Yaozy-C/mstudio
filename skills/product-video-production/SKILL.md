---
name: product-video-production
description: Plan media inputs, diagnose generated action, select usable footage and edit timelines in Mstudio.
---

# Media production and editing

Use the supplied shot design and [core](CORE.md). Production owns video implementation; the artist owns image prompts and stills; the editor owns source selection and audiovisual assembly. Consult the method relevant to the current work instead of following every phase.

## Production decisions

Use the selected model's current supplied rules and capability schema. Query model guidance only if missing or changed. Distinguish ordinary appearance/composition references from actual first/last-frame controls. Verify that the input's geometry and starting support can produce the intended event; calling a middle-state image 'not the first frame' does not remove the conflict.

Apply [controlled generation](references/generation.md) for segmentation, reference selection and a specific production problem. The assigned video-prompt guide converts the design into final text. Keep user-locked aspect, resolution and duration in supported parameters; describe an unsupported requirement explicitly. Generated duration and edited shot duration can differ.

Judge action from its source and initial support through path/contact to destination. A correct ending cannot excuse a missing causal opening. For a faulty transfer, use a compatible input or the smallest useful split/cut consistent with the intended expression. Keep accepted ranges when possible; do not repeat contradictory references with longer negations. Return spatial design conflicts to direction with evidence.

## Source selection and timeline

Read [motion and editing](references/motion-and-editing.md) for usable source intervals, decisive action and sound synchronization. Use [rhythm](../creative-ad-director/references/rhythm.md) to distinguish startup delay, slow movement, repeated information, idle endings and insufficient recognition time. Remove redundant coverage instead of padding to provisional timing.

trimIn/trimOut are source time, start is film time, and duration is (trimOut-trimIn)/speed. Set absolute speed directly; multiply current speed only for an explicit relative request. Whole-film retiming includes related track positions/speeds, caption times and fades while preserving source ranges. Local edits preserve unrelated tracks and accepted content.

Use move_clip for position, slip_clip for source phase, and retime_clip for speed. Same-track ripple does not synchronize other audio/subtitles. Respect source handles and tool limits without silent clamping. Make timing edits before rechecking transitions or grading. For an animatic, existing still holds can test cuts and recognition time, not actual motion speed.

## Inspect and deliver

Apply [output review](references/review-and-delivery.md) and [evidence boundaries](references/evidence-contract.md) to the relevant output. Handoff includes shot IDs, actual assets, usable source ranges, observed events and keep/trim/discard/repair decisions. Missing viewpoints or essential actions remain production gaps; trimming, captions or speed cannot conceal them.

Return the saved prompt, media or timeline requested, with remaining dependencies. Parameters, extracted stills, playback and listening each support different claims; identify the evidence actually available.
