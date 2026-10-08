# Mstudio integration

Read this only for screenwriting tasks in Mstudio. Tools and permissions are governed by the host; do not assume network access, delegation or write access.

## Context and research

Read the relevant original product images, facts, user requirements, existing scripts and references. For a full rewrite, understand the complete intent; for a local change, read only the relevant passages and the necessary context. The handoff must carry the finished-film language, market, character/text/voice-over constraints and hard duration at all times.

Mstudio has no network search capability. Analyse only material accessible within the project; for video, images or audio, report only the evidence the actual tools can read. When material is insufficient, write a provisional draft from known facts, mark the creative assumptions, do not wait for an external Agent to go online, and do not invent market validation.

## Saving

Chat drafts stay in the chat. When the user asks for a save, use the currently available tools such as mstudio_update_screenplay to write screenplay.script:

- id: keep the corresponding segment ID and the existing shot associations.
- title: a short event/segment name.
- action: the events actually visible, plus the viewpoint/revelation/transition that decides what makes it worth watching; do not put research analysis or model parameters here.
- onScreenText: the exact on-screen text, empty if there is none.
- dialogue: the exact dialogue/voice-over, empty if there is no spoken part; do not use "subtitle: ..." in place of the on-screen text field.
- sound: action sounds, ambience, music/pauses and the landing point that do work.
- duration: seconds; the total must respect the locked length or cap, and provisional timing must be marked as such.

Follow the actual schema. A partial merge keeps the other content; a full replacement is used only for an explicitly rewritten scope. Report a save only after the authoritative write receipt succeeds.

## Handoff

Give the document ID, the core thing worth watching, the events/audio-visual relationships that must not be lost, the locked constraints, the provisional realisation and the product conditions still to be confirmed. The director may redistribute provisional timing but must not dissolve the content into a part-by-part tour.

A model's minimum generation length does not equal the length of a viewing beat; a storyboard, an edit shot and a generation task are different things. Writing a script does not authorise generating media.

