# Action, cutting and sound rhythm

Use for shot timing, production handoff, or feedback such as soft, dragging or flat. Choose the experience the film needs; fast cutting is not a default. [Shot grammar](cinematography.md) owns camera and space; [animation principles](animation-principles.md) own poses, effort and inertia.

## Three clocks

Keep these separate in the shot plan:

- Action time: when subjects move, make contact and change state.
- Generation time: the clip requested from the model and, after generation, the actual source timestamps. The selected model's minimum clip length applies here. A Mstudio shot record may package several editorial shots; `shot.duration` is that group's generation span.
- Film time: where retained material appears and how long the audience sees it. A short editorial shot may use only part of a longer generated clip, so generation totals need not equal the runtime.

Plan the viewing sequence first, then group compatible events by the selected model's actual durations and multi-shot ability. Keep intended internal cuts explicit; never smooth them into one continuous camera move to fill a group. If internal cuts are unreliable, generate separate supported clips and select shorter ranges. Never request a sub-minimum generation length and never stretch a brief action to occupy the minimum. Example: a minimum-length source may contribute 0.8 seconds of a completed placement; the rest of the clip is not owed screen time.

## Information before time

For a dense short-form request, list the distinct things the viewer must understand and the visible evidence for each. Keep purposeful emotion, setup and payoff; do not force a selling point into every shot. For each item write the event, the decisive visible moment, the omittable process and the intended film interval; only then choose framing and reading time. Never start by dividing the runtime into equal slots.

Putting different objects into the same container repeatedly communicates one idea. Add a view or event only when it reveals a distinct relationship the product supports: arrangement, access, loaded form, context of use. More cuts, objects or labels are not more content. When the user wants more content in a locked duration, restructure and compress rather than shortening the deliverable or lengthening the film.

## Select phases, shape pace

Causality must hold in the underlying action; not every phase must appear in the edit. Distinguish a continuous proof or tutorial from an edited montage and keep the full process only where the request or the evidence needs it. Otherwise omit redundant reaches, empty-hand returns, repeated transport and idle endings while keeping enough contact, state change and result to understand what happened. Cuts may enter mid-action and leave before the subject stops; outgoing and incoming state and attention still match.

For each retained event choose the useful entry, the decisive change and the exit. A placement keeps alignment, contact and the settled result while skipping most transport; passage through an opening stays visible when it is the evidence. Brief shots and longer recognition holds follow content, not a universal duration or cut count.

When a result feels slow, separate slow physical motion, camera-relative motion, a late action start, redundant information and an overlong hold, and repair that cause. Use speed changes and pauses around events, not a global easing or acceleration. Music cannot repair missing force or contact; retiming cannot hide a failed action.

## Sound in the rough cut

Use the necessary temporary sound and music in the first authorised rough cut. Let music, action sound and ambience take turns leading; loudness and density alone do not make impact. An in-shot action may land on a beat without a cut. Choose synchrony, anticipation, delay or cross-beat timing by the scene while keeping essential contact and results audible.

## Verify and iterate

For uncertain pacing, compare two materially different rough cuts of the core section from existing footage under the same content and duration constraints; a clear small fix can be made directly. A comparison does not authorise new generation or concept changes.

Use the available viewing modes: silent playback for waiting, floating, broken momentum and reading time; audio only for dynamics, space and crowded accents; both together for correspondence, contrast and payoff. Record timecodes, observations and changes. Missing contact needs material repair; sync problems need alignment; redundant holds need trimming; overload needs restructured information.

## Duration and speed feedback

Record whether a time is a maximum, a user-locked total or your own estimate. Do not fill a maximum, do not freeze an estimate into a constraint. Added content should add progression, evidence or expectation, not repeated motion; if the core content cannot fit a locked length, explain the concrete conflict.

Locate the drag before changing time: a late start, slow action, repeated information, an empty tail and insufficient recognition time need different fixes. Do not shrink every shot proportionally by default.

For an explicit whole-film multiplier r, set each duration to original divided by r, synchronise audio and preserve pitch where required, and check captions and effects. An absolute speed is different from multiplying the current speed again. A one-time multiplier is not a permanent default; a simultaneous unchanged-total requirement is a conflict to resolve with the user.

Verify a timed edit, not a list of intended beats: retained events, film intervals, source mapping and the locked total must agree. Continuous playback establishes pacing; stills only reveal wrong states or cuts. Without playback or audio access, report that status as unverified.
