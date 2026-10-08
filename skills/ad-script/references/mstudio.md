# Saving and handoff

Tools and permissions come from the host. Do not assume delegation or write access; when a tool is missing, deliver the text in chat and say what could not be saved.

## Save with mstudio_update_screenplay

Chat drafts stay in chat. When the user asks for a save, write `screenplay.script` paragraphs with `mstudio_update_screenplay`:

- `id`: keep the existing paragraph ID so its shots stay linked; new paragraphs get new IDs.
- `title`: a short event or segment name.
- `action`: the events actually visible, plus the viewpoint, reveal or transition that makes the segment worth watching. No research notes, no model parameters.
- `onScreenText`: the exact on-screen text, empty when there is none.
- `dialogue`: the exact spoken words or voice-over, empty when there is none. Never write "subtitle: ..." here in place of the on-screen text field.
- `sound`: the action sounds, ambience, music or silence that do work, and where they land.
- `duration`: planned seconds. The total must respect a locked length or cap; mark provisional timing as provisional.

`scriptMode=merge` (the default) updates by ID and preserves omitted paragraphs; use `removeParagraphIds` to delete and `paragraphOrder` with all remaining IDs to reorder. `scriptMode=replace` with the complete array is only for an explicitly rewritten scope. Report a save only after the receipt succeeds.

## Worked example: two paragraphs in field form

An illustration of field discipline for an office lunch-bag piece, not a story to reuse. Each field holds only what belongs to it.

```text
id: p2  title: Where are we eating
action: The lead sets the closed blue lunch bag on the desk; its base lands flat before the hand lets go. The coworker leans in and looks at the bag, not at the lead.
onScreenText: (empty)
dialogue: Coworker: "Where are we eating today?"
sound: One soft padded thud as the base meets the desk. Office room tone; no music yet.
duration: 3

id: p3  title: My place
action: One hand steadies the bag while the other draws the slider along the top zipper; the lid lifts on its rear seam and a closed lunch container sits inside with clearance around it.
onScreenText: (empty)
dialogue: Lead: "My place."
sound: Zipper travel, then the fabric lid settling; the line lands just after the lid opens.
duration: 3
```

A weak version of p3 would read: "She opens her amazing bag and shows off her healthy lunch (subtitle: My place)". It mixes the on-screen text into dialogue, replaces visible action with adjectives, and leaves the zipper, the lid and the container for someone else to invent.

## Handoff to direction

Give the screenplay ID, the core thing worth watching, the events and sound relationships that must survive, the locked constraints, the provisional timing, and the product conditions still to confirm. The director may redistribute provisional timing but must not dissolve the content into a part-by-part tour.

A model's minimum clip length is not the length of a viewing beat; a storyboard frame, an edit shot and a generation task are different things. Writing a script does not authorise generating media.
