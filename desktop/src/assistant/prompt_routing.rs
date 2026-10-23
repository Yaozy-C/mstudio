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
    if hits(
        &text,
        &[
            "person",
            "people",
            "portrait",
            "face",
            "skin",
            "character",
            "woman",
            "man",
            "child",
            "hand",
            "hands",
            "actor",
            "actress",
            "identity",
            "人物",
            "人像",
            "肖像",
            "皮肤",
            "脸",
            "角色",
            "模特",
            "手部",
            "手指",
            "女孩",
            "女人",
            "男人",
            "老人",
            "小孩",
            "演员",
        ],
    ) {
        found.insert("people");
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
        routes.push((owner, "references/motion-realism.md"));
    } else {
        routes.push((owner, "references/photographic-direction.md"));
        if found.contains("people") {
            routes.push((owner, "references/skin-realism.md"));
        }
    }
    if owner == "image-production" {
        routes.push((owner, "references/reference-assets.md"));
    }
    if found.contains("frames") && owner == "storyboard-image-production" {
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

    #[test]
    fn photographic_and_motion_methods_route_by_task_and_people_signal() {
        let portrait = routes("image", "portrait of a woman in a room", "image-production");
        assert!(portrait.contains(&("image-production", "references/photographic-direction.md")));
        assert!(portrait.contains(&("image-production", "references/skin-realism.md")));

        let product = routes("image", "product still on a table", "image-production");
        assert!(product.contains(&("image-production", "references/photographic-direction.md")));
        assert!(!product.contains(&("image-production", "references/skin-realism.md")));

        let video = routes("video", "a hand opens the bag", "product-video-production");
        assert!(video.contains(&("product-video-production", "references/motion-realism.md")));

        let board = routes(
            "image",
            "storyboard frame with a person",
            "storyboard-image-production",
        );
        assert!(board.contains(&(
            "storyboard-image-production",
            "references/photographic-direction.md"
        )));
        assert!(board.contains(&("storyboard-image-production", "references/skin-realism.md")));
    }
}
