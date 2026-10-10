//! Task signals that select the method documents a production task needs.
//!
//! The generation panel has no tool loop, so the entry document cannot send the
//! author to a reference on its own. These tables read the task itself: the
//! bound Skill's own prompt and execution guides are always loaded. Additional
//! local references are selected when the description shows the matching need.
use std::collections::BTreeSet;

pub(super) fn hits(text: &str, keys: &[&str]) -> bool {
    keys.iter().any(|key| text.contains(key))
}

/// Read the task, not a job title, and report which methods it needs.
pub(super) fn signals(description: &str) -> BTreeSet<&'static str> {
    let text = description.to_lowercase();
    let mut found = BTreeSet::new();
    if hits(
        &text,
        &[
            "person",
            "people",
            "man ",
            "woman",
            "coworker",
            "colleague",
            "actor",
            "face",
            "smile",
            "laugh",
            "glance",
            "reaction",
            "dialogue",
            "says",
            "saying",
            "asks",
            "speaks",
            "voice-over",
            "voiceover",
            "人物",
            "演员",
            "同事",
            "主角",
            "女",
            "男",
            "脸",
            "表情",
            "微笑",
            "对视",
            "反应",
            "台词",
            "对话",
            "口播",
        ],
    ) {
        found.insert("performance");
    }
    if hits(
        &text,
        &[
            "second",
            "seconds",
            "timing",
            "pace",
            "rhythm",
            "cut to",
            "hard cut",
            "montage",
            "shot 1",
            "shot 2",
            "秒",
            "节奏",
            "时长",
            "切",
            "分段",
            "镜头一",
            "镜头二",
        ],
    ) {
        found.insert("rhythm");
    }
    if hits(
        &text,
        &[
            "photorealistic",
            "photo-real",
            "realistic",
            "realism",
            "skin",
            "texture",
            "fabric",
            "lighting",
            "daylight",
            "natural light",
            "grain",
            "真实",
            "写实",
            "皮肤",
            "质感",
            "布料",
            "光线",
            "自然光",
            "光照",
        ],
    ) {
        found.insert("appearance");
    }
    if hits(
        &text,
        &[
            "hand", "grip", "grasp", "lift", "lower", "place", "open", "close", "zipper", "zip",
            "lid", "contact", "force", "weight", "carry", "transfer", "fingers", "手", "抓", "握",
            "拿", "放", "开", "拉链", "接触", "受力", "重量", "递", "抬",
        ],
    ) {
        found.insert("action");
    }
    if hits(
        &text,
        &[
            "camera",
            "framing",
            "close-up",
            "wide shot",
            "angle",
            "viewpoint",
            "composition",
            "机位",
            "构图",
            "角度",
            "景别",
            "运镜",
        ],
    ) {
        found.insert("grammar");
    }
    if hits(
        &text,
        &[
            "consistent",
            "consistency",
            "same ",
            "continuity",
            "across shots",
            "identity",
            "一致",
            "连续",
            "同一",
            "身份",
            "保持",
        ],
    ) {
        found.insert("continuity");
    }
    if hits(
        &text,
        &[
            "inspect",
            "check the",
            "review the",
            "result",
            "generated clip",
            "成片",
            "检查",
            "看看",
            "抽帧",
            "结果",
        ],
    ) {
        found.insert("review");
    }
    if hits(
        &text,
        &[
            "storyboard",
            "frame moment",
            "first frame",
            "first-frame",
            "start frame",
            "last frame",
            "last-frame",
            "end frame",
            "keyframe",
            "key frame",
            "control frame",
            "分镜",
            "画格",
            "首帧",
            "起始帧",
            "尾帧",
            "结束帧",
            "关键帧",
            "控制帧",
        ],
    ) {
        found.insert("frames");
    }
    found
}

pub(super) fn routes<'a>(
    kind: &str,
    description: &str,
    owner: &'a str,
) -> Vec<(&'a str, &'static str)> {
    let found = signals(description);
    // Spatial coherence is required even when the brief is attached as context.
    // These are the bound Skill's execution methods, never another role's library.
    let execution = if kind == "video" {
        "references/shot-execution.md"
    } else {
        "references/scene-execution.md"
    };
    let mut routes = vec![(owner, "references/prompt-writing.md"), (owner, execution)];
    if kind == "video" {
        routes.push((owner, "references/control.md"));
    }
    if found.contains("frames") && kind == "image" {
        routes.push((owner, "references/frames.md"));
    }
    if found.contains("review") {
        if kind == "video" {
            routes.push((owner, "references/review.md"));
            routes.push((owner, "references/evidence.md"));
        } else {
            routes.push((owner, "references/frame-checks.md"));
        }
    }
    routes
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn storyboard_and_control_frame_requests_route_to_local_image_frame_methods() {
        for brief in [
            "做分镜图",
            "生成首帧",
            "按参考做尾帧",
            "制作关键帧",
            "a storyboard panel",
            "first-frame image",
            "last frame",
            "a keyframe",
            "control frame",
        ] {
            let methods = routes("image", brief, "storyboard-image-production");
            assert!(
                methods.contains(&("storyboard-image-production", "references/frames.md")),
                "{brief}"
            );
            assert!(
                methods
                    .iter()
                    .all(|(owner, _)| *owner == "storyboard-image-production")
            );
        }
        assert!(
            !routes(
                "image",
                "product cover image",
                "storyboard-image-production"
            )
            .contains(&("storyboard-image-production", "references/frames.md"))
        );
        assert!(
            routes(
                "video",
                "animate this first frame",
                "storyboard-video-production"
            )
            .iter()
            .all(|(owner, _)| *owner == "storyboard-video-production")
        );
    }
}
