# Inspection evidence

For each conclusion record the shot ID, the asset ID, the inspected object, the time range or image region, the actual observation, and pass, fail or unchecked, in the existing handoff or reply. Prompts, metadata and completion status are not visual evidence.

- Static: original product parts, frame state, camera, and agreement with the plan.
- Motion: source, initial support, path, decisive contact, destination, counts and positions.
- Sequence: viewing changes, repetition, recognition time, speed and the actual cuts.
- Sound: the range actually listened to, sync, material and level. No audio access means unchecked.
- Technical: only specifications returned by tools or actually inspected; technical validity is not content validity.

## Example record

```text
shot s2 / asset a31 (6.0 s generated)
0.0-2.1 s, frames 0.3 / 1.2 / 2.0: slider travels the full zipper, lid lifts on the rear seam, both pulls present at the open end - pass
2.1-3.6 s, frames 2.4 / 2.9 / 3.4: container rises through the open top with visible clearance, lands beside the bag, hand stays until the base is flat - pass
3.6-6.0 s: hand withdraws, idle hold - usable as handle only
front zip pocket: closed and present in every sampled frame - pass
zipper speed and hand release timing: needs playback - unchecked
zipper and contact sounds: no listening access - unchecked
```

Reinspect the affected areas and joins after a repair, a retime or a new version. Never relabel a known essential failure as unchecked to pass it. Report capability gaps precisely. Project fields and messages carry these records; no separate review file is required.
