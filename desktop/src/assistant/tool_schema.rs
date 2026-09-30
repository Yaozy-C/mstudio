use serde_json::{Value, json};
pub fn schema() -> Value {
    let mut operation = json!({"type":"object","properties":{
        "op":{"type":"string","enum":["add_node","update_node","remove_node","set_brief","append_clip","update_clip"]},
        "id":{"type":"string","description":"Node/clip ID; add_node requires a unique ASCII ID."},
        "kind":{"type":"string","enum":["text","shot","note","asset","screenplay"]},
        "title":{"type":"string"},"text":{"type":"string"},"assetId":{"type":"string"},
        "x":{"type":"number"},"y":{"type":"number"},
        "trimIn":{"type":"number"},"trimOut":{"type":"number"},"speed":{"type":"number"},"volume":{"type":"number"}
      },"required":["op"]});
    for name in [
        "move_clip",
        "retime_clip",
        "slip_clip",
        "set_transition",
        "choose_take",
        "assemble_screenplay",
        "request_generation",
        "update_generation",
        "regenerate_generation",
        "set_creation",
        "set_references",
        "add_track",
        "update_track",
        "add_caption",
        "update_caption",
        "remove_caption",
        "remove_clip",
    ] {
        operation["properties"]["op"]["enum"]
            .as_array_mut()
            .unwrap()
            .push(json!(name));
    }
    let properties = operation["properties"].as_object_mut().unwrap();
    properties.insert("referenceMode".into(), json!({"type":"string","enum":["upsert","remove","replace"],"description":"set_references: prefer upsert to add or update by assetId while preserving unrelated references; remove unlinks assetIds without deleting media; replace explicitly replaces the entire list (legacy default). References do not create canvas cards."}));
    properties.insert("assetIds".into(), json!({"type":"array","items":{"type":"string"},"description":"set_references with referenceMode=remove: asset IDs to unlink from this node only."}));
    properties.insert("name".into(), json!({"type":"string"}));
    properties.insert("description".into(), json!({"type":"string"}));
    properties.insert("sourceOffset".into(), json!({"type":"number","description":"slip_clip: signed source offset in seconds. Shift trimIn/trimOut together, preserving timeline position and duration. Insufficient source handles fail."}));
    properties.insert("ripple".into(), json!({"type":"boolean","description":"retime_clip: preserve source range and start; speed changes duration. true shifts same-track clips at/after the original end; other tracks and captions remain unchanged. Default false."}));
    properties.insert("allowOverlap".into(), json!({"type":"boolean","description":"move_clip/retime_clip reject same-track overlap by default; set true only for explicitly requested layering."}));
    properties.insert("speed".into(), json!({"type":"number","minimum":0.25,"maximum":4,"description":"Absolute playback multiplier. Prefer retime_clip for speed changes, optionally with ripple. move_clip uses frame-aligned timeline start seconds and optional trackId without changing source range."}));
    properties.insert("visual".into(), json!({"type":["object","null"],"description":"Non-destructive update_clip color settings: merge specified absolute values, preserve other effects; null clears. Local FFmpeg filters feed GES preview and export; this does not generate new media.","additionalProperties":false,"properties":{
        "brightness":{"type":"number","minimum":-0.5,"maximum":0.5,"description":"Brightness, default 0, FFmpeg eq."},
        "contrast":{"type":"number","minimum":0.5,"maximum":1.5,"description":"Contrast, default 1, FFmpeg eq."},
        "saturation":{"type":"number","minimum":0,"maximum":2,"description":"Saturation, default 1, FFmpeg eq."},
        "temperature":{"type":"number","minimum":-1,"maximum":1,"description":"Temperature, default 0; negative cools, positive warms, FFmpeg colorbalance."},
        "effect":{"type":"string","enum":["none","grayscale","sepia","blur","vignette"],"description":"One effect; preserve the existing effect while grading unless asked to change it."}
    }}));
    properties["visual"]["properties"]["grade"] = grade_schema();
    properties.insert("fromClipId".into(), json!({"type":"string","description":"set_transition outgoing clip ID; id is the incoming clip. Requires adjacent full-frame clips on the base visual track."}));
    properties.insert("duration".into(), json!({"type":"number","minimum":0.05,"maximum":3,"description":"Total transition duration, no longer than either adjacent clip; does not move clips, audio or captions."}));
    properties.insert("kind".into(), json!({"type":["string","null"],"enum":["text","shot","note","asset","screenplay","fade","fadeblack","fadewhite","wipeleft","wiperight","slideleft","slideright","smoothleft","smoothright","circleopen","circleclose","dissolve","custom",null],"description":"set_transition: null removes; otherwise select the transition type. Missing handles use extended edge frames; prefer short transitions."}));
    properties.insert("design".into(), transition_schema());
    for name in ["start", "end", "scale", "opacity", "fadeIn", "fadeOut"] {
        properties.insert(name.into(), json!({"type":"number"}));
    }
    for name in [
        "trackId",
        "resultAssetId",
        "mediaModelId",
        "canvasTaskKey",
        "taskKey",
        "intent",
        "essential",
        "preserve",
    ] {
        properties.insert(name.into(), json!({"type":"string"}));
    }
    properties.insert("generationPurpose".into(), json!({"type":"string","enum":["asset"],"description":"request_generation: asset creates a standalone reference-image task without a shot link or inherited current frame. Explicit references required; [] is allowed."}));
    properties.insert("mediaModelId".into(), json!({"type":"string","description":"For a model named in chat, use mstudio_models to resolve the exact catalog ID. Clarify ambiguous names rather than guessing a provider. Omission keeps the current selection; do not override a conflicting explicit selection."}));
    for name in ["muted", "hidden"] {
        properties.insert(name.into(), json!({"type":"boolean"}));
    }
    properties.insert(
        "mediaKind".into(),
        json!({"type":"string","enum":["image","video"]}),
    );
    properties.insert(
        "mode".into(),
        json!({"type":"string","enum":["single","ends","multi","mixed"]}),
    );
    properties.insert(
        "trackKind".into(),
        json!({"type":"string","enum":["video","audio"]}),
    );
    properties.insert(
        "stage".into(),
        json!({"type":"string","enum":["planning","production","editing"]}),
    );
    properties.insert(
        "screenplay".into(),
        json!({"type":"object","additionalProperties":false,"properties":{"scriptMode":{"type":"string","enum":["merge","replace"],"description":"Default merge updates by ID. Full rewrites require replace and the complete ordered script; omitted old paragraphs are removed, shot media retained."},"removeParagraphIds":{"type":"array","items":{"type":"string"},"description":"Delete these paragraphs in merge mode; retain shot media."},"paragraphOrder":{"type":"array","items":{"type":"string"},"description":"Reorder in merge mode; list every paragraph ID remaining after the operation."},"script":{"type":"array","maxItems":200,"description":"merge updates by id and preserves omissions; replace requires the complete script with id/title/action/onScreenText/dialogue/sound/duration. Preserve corresponding IDs to retain shot links.","items":{"type":"object","required":["id"],"properties":{"id":{"type":"string"},"duration":{"type":"number","minimum":0.01,"maximum":3600},"title":{"type":"string"},"action":{"type":"string"},"onScreenText":{"type":"string","description":"Text actually displayed on screen, separate from speech; empty string when absent."},"dialogue":{"type":"string"},"sound":{"type":"string"}}}}}}),
    );
    properties.insert("shot".into(),json!({"type":"object","properties":{"screenplayId":{"type":"string"},"scriptId":{"type":"string","description":"Parent script paragraph ID."},"order":{"type":"integer","minimum":1},"duration":{"type":"number","exclusiveMinimum":0},"dialogue":{"type":"string"},"frames":{"type":"array","maxItems":50,"items":{"type":"object","required":["assetId","title"],"additionalProperties":false,"properties":{"assetId":{"type":"string"},"title":{"type":"string"},"prompt":{"type":"string","maxLength":12000}}}},"framePrompt":{"type":"string","maxLength":12000},"prompt":{"type":"string","maxLength":12000}}}));
    properties.insert("references".into(),json!({"type":"array","maxItems":12,"items":{"type":"object","properties":{"assetId":{"type":"string"},"purpose":{"type":"string"},"role":{"type":"string","enum":["edit","reference","first-frame","last-frame","video-reference"]},"start":{"type":"number"},"end":{"type":"number"}},"required":["assetId","purpose"]}}));
    json!({"type":"object","properties":{
      "action":{"type":"string","enum":["inspect","edit","history","skills","read_skill","models"]},
      "skill":{"type":"string","description":"read_skill directory ID from the skills catalog."},
      "path":{"type":"string","description":"Markdown path relative to the Skill root; default SKILL.md. Supports permitted cross-Skill links."},
      "section":{"type":"string","enum":["creation","captions","tracks","assets","clips","generation"],"description":"inspect section to read; omission returns a compact project summary."},
      "revision":{"type":"integer","description":"Latest project revision returned by inspect; required for edit."},
      "fields":{"type":"array","items":{"type":"string","enum":["title","shot","shot.order","shot.duration","shots","dialogue","screenplay","script","framePrompt","prompt","frames","takes","references","assetId","resultAssetId","shotId","start","trimIn","trimOut","speed","trackId","visual","volume","fadeIn","fadeOut","x","y","scale","opacity","transition","name","kind","duration","width","height","text","style","muted","hidden","status","targetNodeId","resultAssetIds","error","trackingPaused","turnId","modelId","ownerId","inputs","parameters","generationPurpose"]},"description":"With nodeIds, return selected fields plus id/kind. shot is basic structure; shot.order/shot.duration select order/timing; shots is a screenplay shot directory; text is action/staging. Omission returns summaries. script gives paragraph summaries; screenplay gives the full paginated script. Narrow with paragraphIds/scriptFields."},
      "ids":{"type":"array","maxItems":12,"items":{"type":"string"},"description":"inspect section=clips/assets/tracks/captions: exact IDs; fields selects returned fields."},
      "nodeIds":{"type":"array","maxItems":12,"items":{"type":"string"},"description":"inspect up to 12 nodes; select needed fields and paginate text with textOffset."},
      "paragraphIds":{"type":"array","maxItems":12,"items":{"type":"string"},"description":"inspect fields=script/screenplay: only these paragraphs; missing IDs are reported."},
      "scriptFields":{"type":"array","items":{"type":"string","enum":["title","duration","action","onScreenText","dialogue","sound"]},"description":"Paragraph fields; id always returned. script defaults to title/duration; screenplay defaults to all. Paginate text with textOffset."},
      "taskId":{"type":"string","description":"history: filter task history by taskScope.taskId."},
      "turnId":{"type":"string","description":"inspect section=generation: filter by durable batch/turn ID."},
      "status":{"type":"string","description":"inspect section=generation: filter status, e.g. FAILED, READY, COMPLETED; batch reports whole-batch statistics."},
      "taskKey":{"type":"string","description":"inspect section=generation: exact task ID, including hidden records."},
      "messageId":{"type":"integer","description":"history: read this historical message."},
      "textOffset":{"type":"integer","description":"inspect/history text offset; use returned nextTextOffset."},
      "mediaModelId":{"type":"string","description":"models: selected generation model ID; returns model-specific prompt rules."},
      "offset":{"type":"integer","description":"inspect/history page offset, default 0."},
      "operations":{"type":"array","maxItems":30,"items":operation}
    },"required":["action"]})
}

fn grade_schema() -> Value {
    let mut properties = serde_json::Map::new();
    for key in [
        "temperature",
        "tint",
        "contrast",
        "saturation",
        "vibrance",
        "shadows",
        "highlights",
        "whites",
        "blacks",
        "balance",
    ] {
        properties.insert(
            key.into(),
            json!({"type":"number","minimum":-100,"maximum":100}),
        );
    }
    properties.insert(
        "exposure".into(),
        json!({"type":"number","minimum":-3,"maximum":3,"description":"Exposure in EV."}),
    );
    for (key, count, min, max, description) in [
        (
            "hsl",
            8,
            -100,
            100,
            "Red/orange/yellow/green/cyan/blue/purple/magenta; each row [hue shift,saturation,lightness]; replaces the whole group.",
        ),
        (
            "curves",
            4,
            0,
            1,
            "Master/R/G/B curves; each row contains outputs at x=.25,.5,.75, monotonically increasing; default [.25,.5,.75].",
        ),
        (
            "wheels",
            3,
            -100,
            360,
            "Shadows/midtones/highlights; each row [hue 0-360,saturation 0-100,lightness -100-100]; default all zeros.",
        ),
    ] {
        properties.insert(key.into(), json!({"type":"array","minItems":count,"maxItems":count,"description":description,"items":{"type":"array","minItems":3,"maxItems":3,"items":{"type":"number","minimum":min,"maximum":max}}}));
    }
    json!({"type":["object","null"],"additionalProperties":false,"properties":properties,"description":"Editable SDR grade inspired by mlight tonal/HSL/curve/wheel controls. Merge specified fields; null clears. Baked as a shared LUT for native preview, frame reads, transitions and export; not a camera Log/HDR transform. Inspect pixels before adjustment and recheck the same times afterward."})
}

fn transition_schema() -> Value {
    let pair = |min: f64, max: f64| json!({"type":"array","minItems":2,"maxItems":2,"items":{"type":"number","minimum":min,"maximum":max}});
    json!({"type":"object","additionalProperties":false,"description":"Composable design for kind=custom only. Positions/offsets are frame fractions; endpoints return to untransformed footage to avoid jumps. Structured numbers only, not filter code.","properties":{
        "mask":{"type":"string","enum":["uniform","linear","radial"],"description":"Overall blend, directional mask or radial mask; can combine with motion."},
        "angle":{"type":"number","minimum":-180,"maximum":180,"description":"Directional mask angle; 0 moves left to right."},
        "center":pair(0.,1.),
        "feather":{"type":"number","minimum":0.001,"maximum":1,"description":"Mask feathering, default 0.1."},
        "curve":{"type":"array","minItems":2,"maxItems":8,"items":pair(0.,1.),"description":"[time fraction,completion fraction] control points. Strictly increasing time, nondecreasing completion; starts [0,0], ends [1,1]. Supports custom pacing."},
        "outgoingZoom":{"type":"number","minimum":1,"maximum":4,"description":"Outgoing shot zooms from 1 to this scale."},
        "incomingZoom":{"type":"number","minimum":1,"maximum":4,"description":"Incoming shot zooms from this scale back to 1."},
        "outgoingOffset":pair(-1.,1.),"incomingOffset":pair(-1.,1.)
    }})
}

/// Advertise only installed capabilities; dispatch still checks authorization.
pub fn for_profile(profile: &super::profiles::AgentProfile) -> Value {
    let mut schema = schema();
    schema["properties"]["action"]["enum"]
        .as_array_mut()
        .unwrap()
        .retain(|v| super::profiles::allows(profile, v.as_str().unwrap_or("")));
    if !super::profiles::allows(profile, "edit") {
        let p = schema["properties"].as_object_mut().unwrap();
        p.remove("operations");
        p.remove("revision");
    } else {
        schema["properties"]["operations"]["items"]["properties"]["op"]["enum"]
            .as_array_mut()
            .unwrap()
            .retain(|op| super::permissions::allows_operation(profile, op.as_str().unwrap_or("")));
        if !profile
            .tool_ids
            .iter()
            .any(|id| id == "project-production" || id == "project-edit")
        {
            schema["properties"]["operations"]["items"]["properties"]["mediaKind"]["enum"] =
                json!(["image"]);
        }
    }
    if super::profiles::allows(profile, "edit")
        && let Some(shot) =
            schema["properties"]["operations"]["items"]["properties"]["shot"].as_object_mut()
    {
        shot.get_mut("properties")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .retain(|key, _| super::permissions::allows_shot_field(profile, key));
        shot.insert("additionalProperties".into(), json!(false));
    }
    schema
}
