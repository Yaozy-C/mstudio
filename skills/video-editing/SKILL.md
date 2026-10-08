---
name: video-editing
description: Assemble generated media in Mstudio - select usable source ranges, set pace and sound, grade colour and design joins, each judged from actual frames, playback and listening.
---

# Editing and finishing

Assemble accepted media into the final viewing experience. Editing decides what the audience sees and for how long; it cannot repair an essential event that was never captured or make an untrue performance true. Read [CORE.md](CORE.md) together with this file; every step below assumes its rules.

## Workflow

1. **Recover the plan and the material.** Read the handed-off shots with their intended film ranges, the user-locked output specifications, and the current clips, tracks, captions and audio with `mstudio_inspect(section=clips)` and the related sections. Distinguish locked specifications from provisional durations and from the actual generated media.
2. **Select only valid chains.** Map each planned shot to real source ranges from inspected frames per [Editing](references/editing.md), keeping only ranges whose source, support, path, contact and destination hold. Record the keep, trim, discard or repair decision per range.
3. **Set pace.** Decide how much screen time each beat earns and what to remove per [Rhythm](../creative-ad-director/references/rhythm.md); do not pad to a provisional length. For speed, set an explicitly requested absolute speed with `mstudio_retime_clip` and multiply the current speed only for an explicit relative request; for a whole-film retime, update related positions, captions and fades together while preserving source ranges.
4. **Place sound and captions.** Arrange information, action and reveal accents, then add authorised sound and captions with the caption and track tools. Check onsets, tails, levels, clipping and silence.
5. **Grade.** Correct before styling per [Colour grading](references/color.md), which includes a worked example, within the [grading tool limits](references/color-tools.md): sample source frames, grade, then recheck graded pixels at the same times.
6. **Design joins.** Decide cut or transition per [Transitions](references/transitions.md), which includes a worked example, and the [transition engine limits](references/transition-tools.md), then inspect the actual composite.
7. **Verify and report.** Confirm edits from the complete receipts, play back what the tools can play, and say which claims remain unverified: playback for motion, listening for audio.

## Read when

| The task has to decide | Read |
|---|---|
| Which source intervals are usable, where to cut, and how sound lands on the cut | [Editing](references/editing.md) |
| **How much screen time each beat earns and what to remove** | **[Rhythm](../creative-ad-director/references/rhythm.md)** |
| Exposure, colour balance, shot matching and a restrained look | [Colour grading](references/color.md), [grading tool limits](references/color-tools.md) |
| Whether to cut or add a transition, and how to build the join | [Transitions](references/transitions.md), [transition engine limits](references/transition-tools.md) |
| The state relationships a cut has to preserve | [Continuity](../creative-ad-director/references/continuity.md) |
| What a clip can support being claimed about it | [Inspection evidence](../product-video-production/references/evidence.md) |

## Evidence for every judgement

Each decision needs its own evidence: source frames for colour and joins, actual composited frames for transitions, playback for continuous motion, listening for audio. Thumbnails, saved parameter values, receipts and task status are not visual acceptance. Say which of these you actually have.

## Failure signatures

| You notice | Do instead |
|---|---|
| Clips are being padded to reach a provisional length | Allocate by information; remove rather than stretch, and report the real total |
| A dissolve is being added to hide a bad action match | Slip, retime or trim first; an effect hides nothing |
| A grade is judged from thumbnails or from the saved parameters | Read frames at the same clip times before and after |
| A missing contact is being covered with music or a speed change | Route it to production with the frame evidence; editing shapes valid material only |
| A relative multiplier is being applied to an absolute speed request, or the reverse | Absolute sets the value; relative multiplies the current speed; say which you did |
| The report says "seamless" or "in sync" without playback or listening | State which joins have composite frames, which motion has playback, which audio was heard |
| A local edit moved, muted or retimed something unrelated | Restore it; preserve every unrelated track, caption and audio element |

## Report

Clip IDs with source in and out, placement and speed as saved; the grade and join parameters confirmed by `savedClips`; the frames and composites inspected with their times; what still needs playback or listening.

## Boundaries

A cut, a grade or a transition cannot repair missing action, invented geometry, identity drift, waxy skin or unnatural acting. Route those to production or direction with the observed evidence instead of masking them with effects, speed or a unified filter.
