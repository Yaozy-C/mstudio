//! Task signals that select the method documents a production task needs.
//!
//! The generation panel has no tool loop, so the entry document cannot send the
//! author to a reference on its own. These tables read the task itself: the
//! bound Skill's own guide is always loaded, and shared method documents are
//! added when the description shows the matching need.
use std::collections::BTreeSet;

const DIRECTION: &str = "creative-ad-director";

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
    if hits(&text, &["storyboard", "frame moment", "分镜", "画格"]) {
        found.insert("frames");
    }
    found
}

pub(super) fn routes(kind: &str, description: &str) -> Vec<(&'static str, &'static str)> {
    let found = signals(description);
    let mut routes: Vec<(&'static str, &'static str)> = Vec::new();
    if kind == "video" {
        routes.push(("product-video-production", "references/prompt-writing.md"));
        routes.push(("product-video-production", "references/control.md"));
    } else {
        routes.push(("image-production", "references/prompt-writing.md"));
    }
    let push = |skill: &'static str, path: &'static str, routes: &mut Vec<_>| {
        if !routes.contains(&(skill, path)) {
            routes.push((skill, path));
        }
    };
    if found.contains("performance") {
        push(
            DIRECTION,
            "references/naturalistic-performance.md",
            &mut routes,
        );
    }
    if found.contains("appearance") {
        push(
            DIRECTION,
            "references/photographic-appearance.md",
            &mut routes,
        );
    }
    if found.contains("rhythm") {
        push(DIRECTION, "references/rhythm.md", &mut routes);
    }
    if found.contains("action") {
        push(DIRECTION, "references/animation-principles.md", &mut routes);
    }
    if found.contains("grammar") {
        push(DIRECTION, "references/cinematography.md", &mut routes);
    }
    if found.contains("continuity") {
        push(DIRECTION, "references/continuity.md", &mut routes);
    }
    if found.contains("frames") && kind == "image" {
        push("image-production", "references/frames.md", &mut routes);
    }
    if found.contains("review") {
        if kind == "video" {
            push(
                "product-video-production",
                "references/review.md",
                &mut routes,
            );
            push(
                "product-video-production",
                "references/evidence.md",
                &mut routes,
            );
        } else {
            push(
                "image-production",
                "references/frame-checks.md",
                &mut routes,
            );
        }
    }
    // A substantial brief that matched no keyword still gets the recommended
    // method set for its kind; a short vague request does not need the library.
    if found.is_empty() && description.chars().count() >= 200 {
        for (skill, path) in recommended(kind) {
            push(skill, path, &mut routes);
        }
    }
    routes
}

pub(super) fn recommended(kind: &str) -> Vec<(&'static str, &'static str)> {
    let mut set = vec![
        (DIRECTION, "references/naturalistic-performance.md"),
        (DIRECTION, "references/photographic-appearance.md"),
        (DIRECTION, "references/animation-principles.md"),
    ];
    if kind == "video" {
        set.push((DIRECTION, "references/rhythm.md"));
    }
    set
}
