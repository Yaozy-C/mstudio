# Action chains and deterministic editing

Follow the handed-off shots and the [continuity record](../../creative-ad-director/references/continuity.md); do not replace the story or the claim.

## Action units

A generation segment carries one action chain the model can express; split by real complexity. For an important contact identify source, grasp, support, path, crossed boundary, release and destination in the underlying action. This is a causality check, not a requirement to show every phase at full length; the director's chosen proof or montage decides which phases stay visible. Material-appropriate soft deformation is allowed without changing product structure or function.

Resolve a risky contact through a readable view and a compatible start, not a negative-word list. Inspect before, during and after the contact plus the necessary continuous footage. Occlusion is not automatically penetration, but it cannot conceal required proof. A single long prompt is an input format, not a continuity guarantee; if a requested single generation fails with pauses, teleportation or penetration, revise the staging or the allowed cuts instead of appending "fluent" and "realistic".

## Time and sound

Apply the three clocks from [rhythm](../../creative-ad-director/references/rhythm.md): keep the model's minimum generation length without imposing it on editorial shots. Record the generation group or shot ID and the intended film intervals before submission; add the actual asset IDs and the inspected source in and out afterwards. Each internal cut stays explicit. If the model cannot create the planned internal sequence, obtain the minimum missing supported clips rather than weakening the edit.

Translate the selected timeline into output frames: editing controls exact cuts, sequential reveals, masks, splits, captions and accents; generation supplies usable handles, not frame precision. Select effective source ranges, remove purposeless waits and keep the reading time for contact, release and results. Speed must not hide errors.

Choose tempo from content and sound, not a fixed BPM. Arrange information, action and reveal accents, then create or use authorised sound. Temporary original beats are allowed with source records, never claimed as real product-test recordings or testimony. Check onsets, tails, levels, clipping and silence.

For duration compression, prioritise information and cut selection. For an explicit whole-film speed, apply the requested multiplier and synchronise tracks per [duration and speed feedback](../../creative-ad-director/references/rhythm.md#duration-and-speed-feedback). Keep the established language and the supported sales claims; do not invent offers.

## Select only valid chains

Map the [shot execution evidence](../../creative-ad-director/references/continuity.md#shot-execution-evidence) to shot ID, source, in and out, observed event, keep or trim or discard or repair, and film range. Identify the actual internal cuts, not one pass per task ID.

Check the selected chain from the object's first appearance, not only from its final result. An intentional omission between compatible states is not a missing action; an impossible support or contact transition is. Do not remove cuts just to show every preparatory movement. Any essential contradiction in source, support, path, contact or destination makes that range unusable; keep a valid tail only if it can join a correct start. If trimming still leaves missing causality, produce the minimum missing event within authorisation; music and transitions cannot hide it.

Compare adjacent selections for purposeful progression, revelation, result or performance; different generation IDs do not justify redundant shots. Select the action before allocating final time, then recheck the joins and the sequence.

## Tools

Use `mstudio_slip_clip(id, sourceOffset)` to change which action phase a clip shows, `mstudio_retime_clip(id, speed, ripple)` for pace, `mstudio_move_clip(id, start, trackId)` for placement, and `mstudio_update_clip(id, trimIn, trimOut, ...)` for source ranges and visual settings. Captions and tracks have their own add, update and remove tools. Verify `savedClips` in the receipt, then recheck the affected frames and joins.
