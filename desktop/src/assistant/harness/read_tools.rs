//! Object-specific model interfaces. The internal project reader stays private.
use rig_core::completion::ToolDefinition;
use serde_json::{Value, json};

const SPECS: &[(&str, &str, &[&str], &[&str])] = &[
    (
        "project",
        "Read project metadata and a paginated node directory. Reuse the supplied snapshot when sufficient. Object contents have dedicated read tools.",
        &["offset"],
        &[],
    ),
    (
        "creation",
        "Read the project brief, creative intent, essential requirements and preserved decisions.",
        &["textOffset"],
        &[],
    ),
    (
        "shots",
        "Read exact shot IDs. Field names are direct: order, duration, frames, prompt. Returned items use those same names, without a shot wrapper. Default reads omit generation prompts. Select prompt/framePrompt only for an explicit prompt edit, unchanged retry or generation investigation. offset paginates frames/takes/references; textOffset paginates long text.",
        &["ids", "fields", "offset", "textOffset"],
        &["ids"],
    ),
    (
        "screenplay",
        "Read the screenplay node identified by id, optionally filtering paragraphIds and paragraph fields. Omitted fields return full paragraphs. Returned items contain script directly. Follow nextScriptOffset and each text's nextTextOffset.",
        &["id", "paragraphIds", "fields", "offset", "textOffset"],
        &["id"],
    ),
    (
        "nodes",
        "Read exact text, note or asset-node IDs. Shots and screenplays have dedicated readers. These are canvas node IDs; media assets use read_assets.",
        &["ids", "fields", "offset", "textOffset"],
        &["ids"],
    ),
    (
        "assets",
        "List confirmed shot assets (characters, locations, props) and their selected reference versions. These are the reusable production references for shot design and generation. Exact IDs can also retrieve known storyboard/video outputs. Materials and assets have independent memberships: a source can remain a material while also being a confirmed shot asset. Raw materials without a confirmed asset cannot be shot-generation references. Descriptions express design intent, not verified pixels. Follow description.nextTextOffset with textOffset. Full provenance fields=[source] is only for prompt editing, unchanged retries or investigation.",
        &["ids", "fields", "offset", "textOffset"],
        &[],
    ),
    (
        "materials",
        "Read source materials for creative development and screenwriting: product facts, brand documents, source photos and reference films. Available when the material-read tool is assigned (creative, assets and writer by default). Research membership survives establishing a confirmed shot asset; the same media may appear in both libraries. The assets specialist may use them to establish a named character/location/prop asset; that asset must be confirmed before a shot can use it. For shot generation, require confirmed asset membership; a source may already share the same ID with a confirmed asset. Collection alone does not confirm a candidate. Exact ids and fields=[description] narrow this read.",
        &["ids", "fields", "offset", "textOffset"],
        &[],
    ),
    (
        "concepts",
        "Read the project concept board. Each card contains title, oneLine, participant, want, ritual, evidence, reframe, risk, optional cost and status. Read titles/status first, then exact cards by id.",
        &["ids", "fields", "offset", "textOffset"],
        &[],
    ),
    (
        "clips",
        "Read timeline clips by exact IDs or list a page. Fields are direct clip properties; times are seconds.",
        &["ids", "fields", "offset"],
        &[],
    ),
    (
        "tracks",
        "Read timeline tracks by exact IDs or list a page.",
        &["ids", "fields", "offset"],
        &[],
    ),
    (
        "captions",
        "Read captions by exact IDs or list a page; textOffset paginates caption text.",
        &["ids", "fields", "offset", "textOffset"],
        &[],
    ),
    (
        "generation",
        "Read generation tasks by taskKey or filter turnId/status. Archived tasks return only compact historical facts in listings, even with fields; supply taskKey to retrieve their full record when needed. Archiving does not authorize resubmission. Default reads omit prompt; select fields=[prompt] only for an explicit prompt edit, unchanged retry or generation investigation. Status and continuation are always returned. Follow nextOffset. Use await_generation for backend work instead of repeated status reads. Submission is not completion.",
        &[
            "taskKey",
            "turnId",
            "status",
            "fields",
            "offset",
            "textOffset",
        ],
        &[],
    ),
];
const SHOTS: &[&str] = &[
    "title",
    "text",
    "screenplayId",
    "scriptId",
    "order",
    "duration",
    "dialogue",
    "framePrompt",
    "prompt",
    "frames",
    "takes",
    "references",
    "scriptChanged",
    "visualChanged",
    "promptStale",
];
fn fields(kind: &str) -> &'static [&'static str] {
    match kind {
        "shots" => SHOTS,
        "screenplay" => &[
            "title",
            "duration",
            "action",
            "onScreenText",
            "dialogue",
            "sound",
        ],
        "nodes" => &["title", "text", "assetId", "resultAssetId", "references"],
        "assets" | "materials" => &[
            "name",
            "description",
            "kind",
            "usage",
            "duration",
            "width",
            "height",
            "imageId",
            "imageVersionId",
            "imageVersionCount",
            "inMaterials",
            "assetStatus",
            "source",
        ],
        "clips" => &[
            "assetId",
            "shotId",
            "start",
            "trimIn",
            "trimOut",
            "speed",
            "trackId",
            "visual",
            "volume",
            "fadeIn",
            "fadeOut",
            "x",
            "y",
            "scale",
            "opacity",
            "transition",
        ],
        "tracks" => &["title", "kind", "muted", "hidden"],
        "captions" => &["text", "start", "end", "style"],
        "concepts" => &[
            "title",
            "oneLine",
            "truth",
            "brandIdea",
            "participant",
            "want",
            "ritual",
            "evidence",
            "reframe",
            "risk",
            "cost",
            "status",
        ],
        "generation" => &[
            "title",
            "status",
            "targetNodeId",
            "resultAssetId",
            "resultAssetIds",
            "error",
            "trackingPaused",
            "turnId",
            "modelId",
            "ownerId",
            "kind",
            "inputs",
            "parameters",
            "generationPurpose",
            "prompt",
        ],
        _ => &[],
    }
}
pub fn definitions() -> Vec<ToolDefinition> {
    let schema = crate::assistant::tool_schema::schema();
    SPECS.iter().map(|(kind, description, keys, required)| {
        let properties: serde_json::Map<_, _> = keys.iter().map(|key| {
            let value = match *key {
                "fields" => json!({"type":"array","minItems":1,"maxItems":fields(kind).len(),"items":{"type":"string","enum":fields(kind)},"description":"Select direct properties of this object; no dotted paths."}),
                "ids" | "paragraphIds" => json!({"type":"array","minItems":1,"maxItems":12,"items":{"type":"string","minLength":1,"maxLength":100}}),
                "id" | "taskKey" | "turnId" => json!({"type":"string","minLength":1,"maxLength":300}),
                "offset" | "textOffset" => json!({"type":"integer","minimum":0}),
                "status" => { let mut value = schema["properties"]["status"].clone(); value["description"] = json!("Filter tasks by exact status."); value },
                _ => schema["properties"][*key].clone(),
            };
            ((*key).into(), value)
        }).collect();
        ToolDefinition {name:format!("mstudio_read_{kind}").try_into().expect("nonempty tool name"),description:(*description).into(),parameters:json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})}
    }).collect()
}
pub fn is_read(name: &str) -> bool {
    SPECS
        .iter()
        .any(|(kind, _, _, _)| name == format!("mstudio_read_{kind}"))
}
/// Only exact advertised names are mapped; no aliases or argument repair.
pub fn decode(name: &str, mut args: Value) -> Option<Value> {
    let kind = name.strip_prefix("mstudio_read_")?;
    if !SPECS.iter().any(|(item, _, _, _)| *item == kind) {
        return None;
    }
    let object = args.as_object_mut()?;
    object.insert("action".into(), json!("inspect"));
    match kind {
        "shots" | "nodes" => {
            let ids = object.remove("ids")?;
            object.insert("nodeIds".into(), ids);
            let requested = object.get("fields").cloned().unwrap_or_else(|| {
                json!(
                    fields(kind)
                        .iter()
                        .filter(|f| !["prompt", "framePrompt"].contains(f))
                        .collect::<Vec<_>>()
                )
            });
            let internal: Vec<_> = requested
                .as_array()?
                .iter()
                .map(|field| match field.as_str()? {
                    "order" => Some(json!("shot.order")),
                    "duration" if kind == "shots" => Some(json!("shot.duration")),
                    "screenplayId" | "scriptId" | "scriptChanged" | "visualChanged"
                    | "promptStale" => Some(json!("shot")),
                    _ => Some(field.clone()),
                })
                .collect::<Option<Vec<_>>>()?;
            object.insert("fields".into(), json!(internal));
        }
        "screenplay" => {
            let id = object.remove("id")?;
            object.insert("nodeIds".into(), json!([id]));
            if let Some(fields) = object.remove("fields") {
                object.insert("scriptFields".into(), fields);
            }
            object.insert("fields".into(), json!(["title", "screenplay"]));
        }
        "project" => {}
        "creation" => {
            object.insert("section".into(), json!("creation"));
        }
        "materials" => {
            object.insert("section".into(), json!("assets"));
            object.insert("usage".into(), json!("material"));
        }
        _ => {
            object.insert("section".into(), json!(kind));
            if kind == "assets" && !object.contains_key("fields") && !object.contains_key("ids") {
                object.insert(
                    "fields".into(),
                    json!([
                        "name",
                        "description",
                        "kind",
                        "usage",
                        "inMaterials",
                        "assetStatus",
                        "duration",
                        "width",
                        "height"
                    ]),
                );
            }
        }
    }
    Some(args)
}
pub fn result(name: &str, mut value: Value) -> Value {
    if value.get("error").is_some() {
        return value;
    }
    let kind = name.trim_start_matches("mstudio_read_");
    if matches!(kind, "shots" | "screenplay" | "nodes") {
        let details = value["details"].as_array().cloned().unwrap_or_default();
        let mut invalid = Vec::new();
        let items: Vec<_> = details
            .into_iter()
            .filter_map(|mut item| {
                let valid = match kind {
                    "shots" => item["kind"] == "shot",
                    "screenplay" => item["kind"] == "screenplay",
                    _ => !matches!(item["kind"].as_str(), Some("shot" | "screenplay")),
                };
                if !valid {
                    invalid.push(item["id"].clone());
                    return None;
                }
                let nested = item
                    .as_object_mut()
                    .unwrap()
                    .remove(if kind == "screenplay" {
                        "screenplay"
                    } else {
                        "shot"
                    });
                let obj = item.as_object_mut().unwrap();
                obj.remove("omittedFields");
                obj.remove("shots");
                obj.remove("nextShotOffset");
                if let Some(Value::Object(fields)) = nested {
                    obj.extend(fields);
                }
                Some(item)
            })
            .collect();
        return json!({"items":items,"missingIds":value["missingNodeIds"],"invalidKindIds":invalid});
    }
    if kind == "project" {
        let obj = value.as_object_mut().unwrap();
        for key in [
            "details",
            "readDetails",
            "missingNodeIds",
            "assets",
            "clips",
            "tracks",
        ] {
            obj.remove(key);
        }
    }
    value
}
#[cfg(test)]
mod tests;
