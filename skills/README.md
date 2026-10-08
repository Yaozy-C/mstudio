# Creative Skill library

This directory ships the five creative Skills that Mstudio Agents read at runtime. Every Markdown file inside a Skill folder is copied into the project database on first launch and shown to the model on request, so each file is written for the model that will apply it, not for a reader who already knows the product.

## How the files are used

- The model receives only the catalog (id, name, description). It reads `SKILL.md` with `mstudio_read_skill` when a task needs the method, and reads a reference only to settle a concrete question.
- `SKILL.md` is the entry: what the Skill delivers, the working steps, when to open which reference, and where the Skill stops.
- `CORE.md` holds the invariants that must hold for every task of that Skill. The quick generation panel loads `SKILL.md`, `CORE.md` and the references selected by `desktop/src/assistant/prompt_routing.rs` in one block, so these three layers must read well together and must not repeat each other.
- `references/` holds one method per file. Names are referenced from code and tests (`prompt_routing.rs`, `prompt_guidance_tests.rs`); rename only together with those files.
- Cross-Skill links are relative paths such as `../creative-ad-director/references/rhythm.md`. Reading through a link is allowed when the linked Skill is enabled or is a declared dependency in `desktop/src/assistant/skills.rs`.

## Authoring rules

- English only (`scripts/check_skills.py` rejects CJK characters). The model answers the user in the user's language regardless.
- Name tools by their real names: `mstudio_update_screenplay`, `mstudio_update_shot`, `mstudio_set_image_prompt`, `mstudio_set_video_prompt`, `mstudio_set_shot_frames`, `mstudio_upsert_references`, `mstudio_generate_image`, `mstudio_generate_reference_image`, `mstudio_generate_video`, `mstudio_update_clip`, `mstudio_set_transition`, `mstudio_slip_clip`, `mstudio_retime_clip`, `mstudio_move_clip`, `mstudio_inspect`, `mstudio_read_image`, `mstudio_await_generation`. Do not describe field schemas in detail; the tool definitions are injected and the host validates them.
- Keep provenance, dates, URLs and maintainer notes out of the model-facing files. Mstudio has no network access, and a link the model cannot open is noise in its context. Record sources in this file instead.
- Prefer a step, a decision table or a short example over an aphorism. Every rule should tell the model what to do and what to check.
- Keep the two sentences that `desktop/src/assistant/prompt_guidance_tests.rs` asserts in `product-video-production`: the heading "Physical causality is not a full operating sequence" and the phrase "concrete purpose".
- Run `python3 scripts/check_skills.py` after editing; it validates folders, frontmatter, links and anchors.
- Shipped text reaches an existing installation only when `desktop/src/assistant/skills/catalog.rs` bumps its marker. Without a bump, new installs get the new text and existing databases keep what they have.

## The example thread

The worked examples across the five Skills follow one fictional piece: an office lunch-bag ad in which the lead opens the bag and lifts out a container. Script paragraphs p2 and p3 (`ad-script/references/mstudio.md`), shot record 2 (`creative-ad-director/references/shot-design.md`), frame f1 (`image-production/references/prompt-writing.md`), generation group 2 (`product-video-production/references/prompt-writing.md`), the evidence record (`product-video-production/references/evidence.md`) and the grade and joins (`video-editing/references/color.md`, `transitions.md`) describe the same handoff chain. Keep new examples on this thread so a reader can follow one piece from script to export, and keep every example labelled as an illustration, never as a story to reuse.

## Evaluating a Skill change

An internal review can only test whether a draft is expressible, coherent, factual and honest about its assumptions. It cannot predict views or conversions, and a model scoring its own output proves nothing.

To compare an old and a new version, run a blind comparison on real briefs across several products and content modes: keep facts, requirements and resources identical, hide which version is which, interleave the order, and let readers say where they kept watching, what they remember, what product value they understood and which part they would cut, before they state a preference. Allow both versions to be bad. After a piece ships, look separately at retention, product understanding and clicks, and record differences in placement, audience, duration, offer and production before drawing any conclusion from one piece.

## Sources

The methods are adapted from public material; none of it was copied, and none of it validates these Skills or any particular video. Dates are when the source was last read by a maintainer.

Screenwriting and research (read 2026-10-07):

- TikTok, "Introducing Symphony Agent" (2026-06-22) and the Top Ads one-pager: evidence-led creation and the limits of per-second attention curves.
- TikTok, "Spark Ads Creative Playbook" (2024-03-04): natural expression, character-product interaction and evergreen forms over forced hooks.
- Chris Kocek for Contagious, "How to spot a true, transformative insight" (2023-09-26): an observation is not yet an insight.
- Andrew Stanton, TED 2012: make the audience care, promise something worth continuing for.
- Harmon Brothers interviews and Writers Room page: develop the expression from the communication difficulty; separate the choice of claim from copy polish.
- Kantar, "Beyond viewability"; Thinkbox / Neuro-Insight, "Creative Drivers of Effectiveness" (2023-06-21); System1 / WPP Media / TikTok, "The Creator Effectiveness Playbook": check attention, brand memory and purchase understanding separately.
- Doshi and Hauser, Science Advances 2024: AI assistance raised individual creativity ratings while reducing diversity across works; compare candidates at the content level.

Direction and animation:

- Frank Thomas and Ollie Johnston, "The Illusion of Life"; Disney Animation and Adobe summaries of the twelve principles.
- Khan Academy / Pixar in a Box, film grammar; Columbia film glossary and continuity notes; StudioBinder on the 30-degree rule.
- Higgsfield Academy, "AI ad in 3 steps" and the public tutorial on shot lists and in-shot performance.
- TikTok Creative Codes; marketingskills hook-system notes; Krea cut-architecture notes.

Editing, colour and transitions (read 2026-09-27):

- Blackmagic Design, DaVinci Resolve colour training and product documentation.
- FFmpeg filter documentation, including `xfade` and `perspective` sources; Shotcut brightness filter and keyboard shortcut documentation.
- CapCut help on transitions and dissolves.
