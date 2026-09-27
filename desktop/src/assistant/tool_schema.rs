use serde_json::{Value, json};
pub fn schema() -> Value {
    let mut operation = json!({"type":"object","properties":{
        "op":{"type":"string","enum":["add_node","update_node","remove_node","set_brief","append_clip","update_clip"]},
        "id":{"type":"string","description":"节点/片段 ID；add_node 用唯一英文 ID"},
        "kind":{"type":"string","enum":["text","shot","note","asset","plan"]},
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
        "assemble_plan",
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
    properties.insert("sourceOffset".into(), json!({"type":"number","description":"slip_clip：源素材偏移秒数，可正可负，同时移动 trimIn/trimOut，保持时间线位置和时长；源余量不足报错"}));
    properties.insert("ripple".into(), json!({"type":"boolean","description":"retime_clip：保持源区间与起点，按 speed 改变时长；true 顺移同轨原尾点及之后的片段，其他轨道和字幕不动。默认 false"}));
    properties.insert("allowOverlap".into(), json!({"type":"boolean","description":"move_clip/retime_clip 默认拒绝同轨重叠；仅用户明确需要叠加时设 true"}));
    properties.insert("speed".into(), json!({"type":"number","minimum":0.25,"maximum":4,"description":"播放倍率；retime_clip 推荐用于变速，可配合 ripple。move_clip 使用 start（时间线秒数，自动对齐帧）及可选 trackId，不改变源区间"}));
    properties.insert("visual".into(), json!({"type":["object","null"],"description":"update_clip 的非破坏式调色；只覆盖指定参数，保留其余效果；null 清除。由本地 FFmpeg 滤镜执行并同步到 MLT 预览与成片导出，不是生成新视频。参数是绝对值。","additionalProperties":false,"properties":{
        "brightness":{"type":"number","minimum":-0.5,"maximum":0.5,"description":"亮度，默认0，FFmpeg eq"},
        "contrast":{"type":"number","minimum":0.5,"maximum":1.5,"description":"对比度，默认1，FFmpeg eq"},
        "saturation":{"type":"number","minimum":0,"maximum":2,"description":"饱和度，默认1，FFmpeg eq"},
        "temperature":{"type":"number","minimum":-1,"maximum":1,"description":"冷暖，默认0，负冷正暖，FFmpeg colorbalance"},
        "effect":{"type":"string","enum":["none","grayscale","sepia","blur","vignette"],"description":"单个特效；调色时保留现有效果，除非用户要求改变"}
    }}));
    properties["visual"]["properties"]["grade"] = grade_schema();
    properties.insert("fromClipId".into(), json!({"type":"string","description":"set_transition 的前一片段ID，id 为后一片段ID；必须底层画面轨相邻全幅片段"}));
    properties.insert("duration".into(), json!({"type":"number","minimum":0.05,"maximum":3,"description":"转场总时长，不超过任一相邻片段时长，不移动剪辑、音频或字幕"}));
    properties.insert("kind".into(), json!({"type":["string","null"],"enum":["text","shot","note","asset","plan","fade","fadeblack","fadewhite","wipeleft","wiperight","slideleft","slideright","smoothleft","smoothright","circleopen","circleclose","dissolve","custom",null],"description":"set_transition: null 移除；其他为转场类型，缺余量会延展边缘帧，短时优先"}));
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
        "plan".into(),
        json!({"type":"object","properties":{"scriptMode":{"type":"string","enum":["merge","replace"],"description":"默认 merge 按 ID 局部合并；整篇改写必须 replace，script 提供完整段落与顺序，省略的旧段落删除，镜头素材保留"},"removeParagraphIds":{"type":"array","items":{"type":"string"},"description":"merge 模式删除指定段落，保留镜头素材"},"paragraphOrder":{"type":"array","items":{"type":"string"},"description":"merge 模式调整顺序，必须列出操作后的全部段落 ID"},"story":{"type":"string"},"sound":{"type":"string"},"script":{"type":"array","maxItems":200,"description":"merge 按 id 合并，省略内容保留；replace 全篇替换，必须提供完整段落 id/title/action/onScreenText/dialogue/sound/duration；保留对应段落 ID 可维持镜头关联","items":{"type":"object","required":["id"],"properties":{"id":{"type":"string"},"duration":{"type":"number","minimum":0.01,"maximum":3600},"title":{"type":"string"},"action":{"type":"string"},"onScreenText":{"type":"string","description":"画面上实际显示的文字，与台词分开；无则空字符串"},"dialogue":{"type":"string"},"sound":{"type":"string"}}}}}}),
    );
    properties.insert("shot".into(),json!({"type":"object","properties":{"planId":{"type":"string"},"scriptId":{"type":"string","description":"所属脚本段落 ID"},"order":{"type":"integer","minimum":1},"duration":{"type":"number","exclusiveMinimum":0},"dialogue":{"type":"string"},"frames":{"type":"array","maxItems":50,"items":{"type":"object","required":["assetId","title"],"additionalProperties":false,"properties":{"assetId":{"type":"string"},"title":{"type":"string"},"prompt":{"type":"string","maxLength":12000}}}},"framePrompt":{"type":"string","maxLength":12000},"prompt":{"type":"string","maxLength":12000}}}));
    properties.insert("references".into(),json!({"type":"array","maxItems":12,"items":{"type":"object","properties":{"assetId":{"type":"string"},"purpose":{"type":"string"},"role":{"type":"string","enum":["edit","reference","first-frame","last-frame","video-reference"]},"start":{"type":"number"},"end":{"type":"number"}},"required":["assetId","purpose"]}}));
    json!({"type":"object","properties":{
      "action":{"type":"string","enum":["inspect","edit","history","skills","read_skill","models"]},
      "skill":{"type":"string","description":"read_skill 的技能目录 ID，来自 skills 目录"},
      "path":{"type":"string","description":"相对 skill 根目录的 Markdown 路径，默认 SKILL.md；支持链接到其他已启用技能"},
      "section":{"type":"string","enum":["creation","captions","tracks","assets","clips","generation"],"description":"inspect 按需读取的内容区，省略为精简工程摘要"},
      "revision":{"type":"integer","description":"inspect 返回的最新工程 revision；edit 必填"},
      "fields":{"type":"array","items":{"type":"string","enum":["title","shot","shot.order","shot.duration","shots","dialogue","plan","script","framePrompt","prompt","frames","takes","references","assetId","resultAssetId","shotId","start","trimIn","trimOut","speed","trackId","visual","volume","fadeIn","fadeOut","x","y","scale","opacity","transition","name","kind","duration","width","height","text","style","muted","hidden"]},"description":"nodeIds：fields 只返回选定字段，id/kind 始终返回。shot 为基础结构，shot.order/shot.duration 仅取顺序/时长；shots 为方案镜头目录，text 为动作。省略 fields 只返回摘要；text 读取动作正文，script 读取脚本段落摘要；plan 读取完整方案（分页），可用 paragraphIds/scriptFields 限定脚本范围"},
      "ids":{"type":"array","maxItems":12,"items":{"type":"string"},"description":"inspect section=clips/assets/tracks/captions：按精确 ID 读取；fields 选择返回字段"},
      "nodeIds":{"type":"array","maxItems":12,"items":{"type":"string"},"description":"inspect: 批量读取最多 12 个节点；建议配合 fields 选择字段，文字按 textOffset 分页"},
      "paragraphIds":{"type":"array","maxItems":12,"items":{"type":"string"},"description":"inspect fields=script/plan：只读这些脚本段落；不存在的 ID 明确返回"},
      "scriptFields":{"type":"array","items":{"type":"string","enum":["title","duration","action","onScreenText","dialogue","sound"]},"description":"脚本段落字段，id 始终返回。script 默认 title/duration；plan 默认全部。文字使用 textOffset 分页"},
      "taskId":{"type":"string","description":"history：按 taskScope.taskId 筛选任务历史"},
      "taskKey":{"type":"string","description":"inspect section=generation 按任务 ID 查询，包含已隐藏记录"},
      "messageId":{"type":"integer","description":"history: 读取指定历史消息"},
      "textOffset":{"type":"integer","description":"inspect/history 的文字偏移；使用返回的 nextTextOffset"},
      "offset":{"type":"integer","description":"inspect/history 分页偏移，默认 0"},
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
        json!({"type":"number","minimum":-3,"maximum":3,"description":"曝光 EV"}),
    );
    for (key, count, min, max, description) in [
        (
            "hsl",
            8,
            -100,
            100,
            "红橙黄绿青蓝紫洋红，每行[色相偏移,饱和度,明度]；整组覆盖",
        ),
        (
            "curves",
            4,
            0,
            1,
            "总/R/G/B曲线，每行是x=.25,.5,.75的三个输出值，单调递增；默认[.25,.5,.75]",
        ),
        (
            "wheels",
            3,
            -100,
            360,
            "阴影/中间调/高光，每行[色相0–360,饱和度0–100,明度-100–100]；默认全零",
        ),
    ] {
        properties.insert(key.into(), json!({"type":"array","minItems":count,"maxItems":count,"description":description,"items":{"type":"array","minItems":3,"maxItems":3,"items":{"type":"number","minimum":min,"maximum":max}}}));
    }
    json!({"type":["object","null"],"additionalProperties":false,"properties":properties,"description":"可编辑 SDR 调色方案，借鉴 mlight 的明暗/HSL/曲线/分区色轮；只覆盖指定字段，null 清除方案。烘焙为自定义 LUT，原生预览、抽帧、转场和导出共用；不是相机Log/HDR转换。先看帧再调参，修改后同一时间复查。"})
}

fn transition_schema() -> Value {
    let pair = |min: f64, max: f64| json!({"type":"array","minItems":2,"maxItems":2,"items":{"type":"number","minimum":min,"maximum":max}});
    json!({"type":"object","additionalProperties":false,"description":"kind=custom 的可组合设计，其他 kind 不接受。位置与偏移以画幅比例计；首尾自动回到未变换原片，避免接缝跳变。只接受结构化数值，不接受滤镜代码。","properties":{
        "mask":{"type":"string","enum":["uniform","linear","radial"],"description":"整体混合/方向蒙版/径向蒙版，可与运动组合"},
        "angle":{"type":"number","minimum":-180,"maximum":180,"description":"方向蒙版角度，0 从左向右"},
        "center":pair(0.,1.),
        "feather":{"type":"number","minimum":0.001,"maximum":1,"description":"蒙版边缘柔化，默认0.1"},
        "curve":{"type":"array","minItems":2,"maxItems":8,"items":pair(0.,1.),"description":"[时间比例,完成比例]控制点，时间严格递增、完成比例不下降，必须始于[0,0]终于[1,1]；可设计先慢后快等节奏"},
        "outgoingZoom":{"type":"number","minimum":1,"maximum":4,"description":"前镜从1倍推进到此倍率"},
        "incomingZoom":{"type":"number","minimum":1,"maximum":4,"description":"后镜从此倍率回到1倍"},
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
