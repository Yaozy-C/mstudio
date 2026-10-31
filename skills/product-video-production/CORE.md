# Core: media production

Start from the selected shot's current complete text and dialogue, using linked story context where needed. Understand its audience-facing purpose, essential change and narrative result. Ground production in the saved approved design and latest explicit user changes; a coordinator summary or old prompt does not replace them. Preserve the event's meaning when choosing inputs and splitting the scene into single shots.

Realize the selected creative event through its action sequence, physical causes and intended result. Derive action timing, force, pauses and editorial exit from that purpose; keep rigid parts rigid, let soft goods deform, show support and keep the label readable. Read [product motion realism](references/motion-realism.md) when the product drifts, a hand mutates or the clip reads as a synthetic turntable.

Inspect relevant material to decide what it supplies, what should vary and which supported route can realize the shot within scope. Planning media need not be uploaded or determine final appearance. Explain concrete missing-input needs through the existing handoff, and carry useful observations and production reasons forward when work changes hands.

Preserve the chosen product, characters and scene direction with real references. Use concrete image asset IDs in task references; earlier submitted inputs remain historical evidence, not current truth. Do not turn an ordinary scene into glossy commercial or cinematic imagery without that direction. Preserve physical support, contact and action continuity, and use the selected model's supported parameters.

Video production owns product fidelity through motion and cuts: use the original evidence, preserve visible construction and materials, and repair observed drift within authorization. Execute the director's camera, lens/distance, focus and staging decisions without forcing identical product views. Changes of projection, occlusion and physically motivated deformation are compatible with identity. Return an unsupported essential mechanism or an actual design conflict; do not send ordinary generated component errors back as a request to redesign the shots.

Give every referenced input a concrete purpose that names the subject and the visual use, and keep that assignment stable across segments. `role` is the technical input mode; `purpose` is the visual use.

Create **one video generation task per single continuous shot**. A shot is one camera setup and one continuous action; its `shot.duration` is the intended screen duration. Do not group adjacent shots into one task. If a shot is shorter than the model minimum, generate at the minimum and trim; do not pad, stretch or merge. If a saved shot contains multiple cuts or an internal time map, return it to direction for splitting before generating. The prompt describes one action and one camera behavior; internal cut points belong to editing, not to a single-shot prompt.

Describe the visible change from the actual starting state through the decisive event to the ending state. Keep subject movement, camera movement and focus changes distinct; explain relevant light, material and sound changes through their causes. Stable properties remain stable. Use only factors needed by the selected shot, without adding motion or effects to satisfy a template.

Write the complete physical causal chain into the actual prompt, including necessary intermediate contact, support, grip and release states. Preserve structure while allowing physically required pose and deformation changes. Editorial omission requires a designed cut with compatible states; a continuous shot cannot skip a necessary transition. Remove only idle time or repetition that changes no physical state.

Preserve exact required dialogue and voice-over in the actual prompt when the selected video model generates audio, including language, speaker delivery and timing relative to action. A missing standalone audio tool or visible speaker does not remove that capability. Keep subtitle compositing separate from speech generation; captions and empty audio tracks cannot fulfill an audible deliverable. Use a separately selected audio route or report a concrete endpoint limitation without silently dropping speech.

Report actual submission, completion and import states without implying visual approval. Detailed prompt, control and generation rules are listed in [SKILL.md](SKILL.md).
