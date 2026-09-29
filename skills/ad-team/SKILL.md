---
name: ad-team
description: Coordinate Mstudio script, storyboard, image, media and editing roles with scoped assignments and verifiable handoffs.
---

# Production coordination

The short [CORE.md](CORE.md) is automatically loaded from the database. Read this detailed document only when the current task needs its methods.

The coordinator maintains the current goal, facts, decisions, scope and handoff. Users need not relay messages between roles. Application configuration determines role permissions; this skill does not add tools or create Codex chats/processes.

## Assign scoped work

Use mstudio_delegate(agentId, task) when a specialist is needed. Pass the current request, actual target IDs and editable fields, preserved intent, verified evidence/source, concrete problem and completion conditions. Label old designs and diagnoses as background, not user decisions. Default to an independent spawn. To continue an existing specialist session, use mstudio_send_message(agentId=childId, message=task) only when canContinue=true; select mode=continuable when initially delegating work that will need follow-ups. A oneShot session cannot resume: spawn a new task with existing findings and IDs. fork creates a new child seeded only with the parent session's completed history; it does not restore a previous specialist session. A simple task calls only necessary roles. Do not claim delegation occurred before it succeeds.

Local edits do not add full-film review, other shots or downstream production. Read wider context only for a concrete dependency without expanding writes. After a specialist saves and verifies the requested change, check completion and continue only within authorization. Historical pending work is not a new assignment.

| Responsibility | Owner and rules |
|---|---|
| Story and script | Writer using [scriptwriting](../ad-script/SKILL.md) |
| Camera, blocking and continuity | Director using [storyboard direction](../product-storyboard/SKILL.md) |
| Shared white-background identity assets | Asset specialist using [asset preparation](../asset-preparation/SKILL.md) |
| Static frames and image inputs | Artist using [storyboard art](../storyboard-art/SKILL.md) |
| Video prompt, mode, reference roles and parameters | Media producer using [production](../product-video-production/SKILL.md) |
| Footage selection, timeline and sound | Editor using production and rhythm references |

The coordinator does not fill in specialists' professional decisions or prescribe technical downgrades. Specialists resolve routine choices themselves; cross-role conflicts return with evidence, without another approval layer. Existing shot text is not automatically a user lock. Preserve accepted viewing effects while allowing assistant-proposed geometry/timing to change. Ordinary references and first/last-frame controls are not interchangeable.

## Continue and complete

Use [project state](../product-storyboard/references/project-state.md) when needed. New work reuses facts, not old stories or approvals. Existing scripts go directly to direction. Frame generation follows the current image request; text-only edits do not generate media. Production owns each task ID and checks status before retrying. Do not expand dependent material while its critical relationship is known to fail.

Read groups of needed nodes with nodeIds/fields and reuse available results. After failure, distinguish saved work from incomplete work, diagnose and fix the cause rather than dispatching the unchanged assignment repeatedly. Two diagnosed attempts without progress on the same defect require a route change or a concrete blocker report; budget/attempt limits still apply.

When an artist is waiting for a critical anchor, preserve that dependency in the handoff. Resume inspection when the real asset is ready before requesting dependent expansion; a queued task or child completion is not visual acceptance. Do not instruct the artist to finish the whole batch by bypassing its unresolved structural check.

Read the current specialist roster and effective tool permissions to determine whether independent review is available. Delegate review only when it serves the current authorized scope; do not add a mandatory review round. If the requested role is unavailable, report the specific limitation and continue supported checks. Executing roles inspect actual outputs within their capabilities. Child completion is not proof of media quality. Report pending, awaiting confirmation and unchecked states accurately.

## Match the deliverable to the request

A prompt edit returns the saved task/draft, not compulsory generation or editing. Direction returns saved shot design, necessary frame moments, constraints and unknowns using the shared shot rules. Art returns real asset IDs and frame observations. Production returns real results, selected ranges and keep/discard/repair decisions. Editing returns the actual timeline, selection reasons and audiovisual changes. Self-checks identify shot/asset, observed evidence and unchecked scope.

Use mstudio_read_image(assetId) for current images and mstudio_reopen_image(imageId) only for unloaded historical references. Never infer actual output from prompt text or guessed IDs. Maintain one current project representation, not competing copies. Shared memory writes require the current session's effective memory-write permission, enabled project memory and autoUpdate. Store only new, corrected or revoked lasting user decisions, quoting the original current user request as evidence; delegated messages are not user evidence. Do not infer permission from a role name. Scripts, shots and generation state remain in the project.

Before commissioning shared assets, have asset-designer read the scoped storyboard and audit current assets/records/tasks. Only elements repeated across distinct shots with a real consistency need qualify for automatic asset preparation. Match consumers to reusable, unchecked, pending and missing references; commission only concrete gaps, not every item in every shot. If no gap exists, continue with existing assets. Single-shot details and same-shot A/B continuity stay with the artist. Missing shared asset dependencies precede their consuming frames: inspect the real result before handing off asset IDs, roles and locks. Independent work can continue while waiting; queued jobs are not ready inputs.

Detailed role scope and execution methods: [role methods](references/role-methods.md). Read only when relevant.
