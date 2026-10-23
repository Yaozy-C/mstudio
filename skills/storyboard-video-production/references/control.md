# Choose a controllable production route

Start from shot purpose and what must remain fixed. Routes may be combined; no vendor is the default. References constrain generation but do not lock pixels or geometry. Preserve the specified identity, geometry and action.

| Control need | Possible route | Limit |
|---|---|---|
| Position, scale and composition | Layout sketch, annotated reference or static composite before generation | Layout is not final product restoration |
| Clear start with limited change | Selected production frame plus image-to-video | Motion can still alter parts |
| Existing action with visual changes | Source action clip plus video editing | Product regions can change too |
| Exact visible product details | Retained original product layer with separately generated environment/effects and compositing | A flat original does not solve new views, occlusion or soft-body motion |
| Precise path or parallax | Available 3D/proxy previs and camera/depth references | Requires actual tools/assets; do not invent completed modeling |
| Exact timing, splits or graphics | Deterministic editing, masks and layer animation | Editing cannot hide missing required proof |

Reference the [saved shot design](shot-execution.md); add input roles, chosen route, reason and concrete limits in the existing handoff, not another full camera table. Separate product fact, scene/camera baseline, composition, motion and actual control frames. Use the minimum relevant set and resolve conflicts before submission.

## Submit the temporal storyboard board

The default storyboard route uses one board asset for the selected shot record, with its existing ordered panel map. A single asset may contain several states; it is not one simultaneous scene and not four separate input files.

Use an endpoint that accepts storyboard/shot-planning image references. Use its exposed reference role and the matching purpose: for example, "shot s1 temporal storyboard; TL/TR/BL/BR define opening, entry, contact and ending; one continuous take". In Mstudio use role=reference only when the selected model supports it; the adapter owns native field names. Never put the whole grid into first-frame or last-frame fields. Reference support is not a guarantee of exact timestamp/keyframe adherence.

If the chosen endpoint supports only first/last frames or cannot parse board references, report that specific incompatibility before a paid call. Preserve the chosen board workflow; do not silently crop panels, generate a second control set or switch the user's model/service. An explicitly requested independent control task is a different authorized input use.

Keep original product/identity evidence only where the board's cell resolution or geometry needs it, with separate concrete purposes. Avoid unrelated assets and contradictory states. Carry the same board mapping into the final submitted prompt; saving a correct draft is insufficient if actual input roles differ.

## Preserve motion and editorial intent across routes

For motion-critical work, distinguish a text speed descriptor, a visual motion reference, and an actual trajectory/layout-conditioned model input. Choose according to available capabilities; a generic video reference does not guarantee preserved timing or camera geometry. Use the supplied proxy motion and camera for the assigned execution; return the generated result to the coordinator for review. Do not expect the model to repair a slow or incorrectly staged reference.

A generation group may contain multiple editorial shots only on a route capable of expressing them. If that route cannot express the design, return the required generation split and cut map through coordination before creating different tasks; deterministic assembly belongs to editing. Keep the model's minimum generation length while selecting the needed shorter ranges. Do not change requested montage cuts into continuous orbiting or slow actions merely to match the API duration.
