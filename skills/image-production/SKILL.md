---
name: image-production
description: Produce still images in Mstudio: write one-moment image prompts, execute designed frames, prepare reusable white-background reference assets and inspect actual pixels.
---

# Image production

Produce the requested still image and the reusable references it depends on. Preserve the chosen concept, the selected identity and verified product geometry. Loading this Skill grants no extra tools or permissions: saving a prompt is not a generated image, and a generated image is not an inspected one.

## Read for the task at hand

| The task has to decide | Read |
|---|---|
| How to write one prompt for one visible moment | [Image prompt writing](references/prompt-writing.md) |
| How to execute a designed shot as a readable frame, and how dependent frames reuse an anchor | [Storyboard frames](references/frames.md) |
| Whether a shared identity reference is missing, and how to design a white-background asset | [Reference assets](references/reference-assets.md) |
| How to inspect actual pixels for viewpoint, contact, part count and cross-frame state | [Frame inspection](references/frame-checks.md) |
| Staging, attention centre or spatial relationships that need redesigning | [Shot grammar](../creative-ad-director/references/cinematography.md) |
| Photographic people, skin, light and material appearance | [Photographic appearance](../creative-ad-director/references/photographic-appearance.md) |

Read [core](CORE.md) for the short rules.

## Bind every input to a concrete purpose

Each supplied image is one of: edit target, identity or wardrobe reference, product geometry and material evidence, composition or state reference, or motion reference. Name that use plus the subject it supplies in `references[].purpose` — for example "the lead's face and hair identity", "the opened main compartment as state reference", "customer's own product front panel for geometry" — never a bare label such as "content reference". `role` selects the technical input mode and `purpose` describes the visual use; they are separate fields and both must be correct.

Purposes carry identity assignments. When a scene has more than one recurring person or object, bind each input to the named subject, and keep that binding consistent through the prompt text and any later segment. An image simply labelled "reference" cannot hold a character's identity.

## What a reference does and does not transfer

A white-background identity asset fixes who the subject is. It does not fix pose, camera, background or lighting for a scene frame: do not inherit its standing pose or studio light into a scene, and do not restate them as if they were requested.

A reference of an opened bag, a mid-action state or a detail close-up supplies structure and state. It is not a first frame, and it does not license a prompt to repeat an event that is already complete inside it.

## Boundaries

Script content, shot design and video generation belong to other Skills. Fix execution deviations locally; return design conflicts with evidence. Static frames never establish motion, speed, sound or playback.
