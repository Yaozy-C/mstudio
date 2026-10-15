//! Port of DSH 5719020 content.ts requiredImageOffload: count retained
//! occurrences at their represented size, then omit only the oldest excess.
//! Files routes count raw image bytes; other SDK routes reserve 2 MiB for
//! text/structure under the existing 18 MiB attachment request envelope.
use super::{
    Host, media_retention,
    session::{Session, consumed_prefix},
};
use rig_core::message::{DocumentSourceKind, Image, Message, ToolResultContent, UserContent};

const MAX_INLINE_IMAGE_BYTES: usize = 16 * 1024 * 1024;

fn inline_bytes(image: &Image) -> usize {
    match &image.data {
        DocumentSourceKind::Base64(data) => data.len(),
        DocumentSourceKind::Raw(data) => data.len().div_ceil(3).saturating_mul(4),
        DocumentSourceKind::Url(url) if url.starts_with("data:") => {
            url.split_once(',').map_or(0, |(_, data)| data.len())
        }
        // Native localImage, remote URLs and file IDs are references, not inline bytes.
        _ => 0,
    }
}

pub(super) fn images(messages: &[Message]) -> Vec<&Image> {
    let mut images = Vec::new();
    for message in messages {
        if let Message::User { content } = message {
            for part in content {
                match part {
                    UserContent::Image(image) => images.push(image),
                    UserContent::ToolResult(result) => {
                        for part in &result.content {
                            if let ToolResultContent::Image(image) = part {
                                images.push(image);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    images
}

fn required_offload(
    lengths: impl IntoIterator<Item = usize>,
    limit: usize,
    max_images: usize,
) -> usize {
    let lengths: Vec<_> = lengths.into_iter().collect();
    let total = lengths
        .iter()
        .fold(0usize, |total, bytes| total.saturating_add(*bytes));
    let excess = total.saturating_sub(limit);
    let excess_count = lengths.len().saturating_sub(max_images);
    if excess == 0 && excess_count == 0 {
        return 0;
    }
    let mut removed = 0usize;
    for (index, bytes) in lengths.into_iter().enumerate() {
        removed = removed.saturating_add(bytes);
        if removed >= excess && index + 1 >= excess_count {
            return index + 1;
        }
    }
    unreachable!("the full prefix covers the excess")
}

pub fn fit(
    session: &mut Session,
    host: &impl Host,
    profile: &crate::assistant::config::Profile,
) -> Result<usize, String> {
    if let Some(backend) = crate::assistant::uploader::backends::select(profile) {
        let budget = backend.image_budget();
        fit_policy(session, host, budget.bytes, budget.count, true)
    } else {
        fit_policy(session, host, MAX_INLINE_IMAGE_BYTES, usize::MAX, false)
    }
}

#[cfg(test)]
pub(super) fn fit_limit(
    session: &mut Session,
    host: &impl Host,
    limit: usize,
) -> Result<usize, String> {
    fit_policy(session, host, limit, usize::MAX, false)
}

fn fit_policy(
    session: &mut Session,
    host: &impl Host,
    limit: usize,
    max_images: usize,
    files: bool,
) -> Result<usize, String> {
    if host.token().is_cancelled() {
        return Err(crate::app_error::cancelled());
    }
    let count = required_offload(
        images(&session.messages).into_iter().map(|image| {
            if !files {
                return inline_bytes(image);
            }
            match &image.data {
                DocumentSourceKind::Raw(data) => data.len(),
                _ => {
                    let encoded = inline_bytes(image);
                    let padding = match &image.data {
                        DocumentSourceKind::Base64(data) | DocumentSourceKind::Url(data) => {
                            data.bytes().rev().take_while(|c| *c == b'=').count()
                        }
                        _ => 0,
                    };
                    (encoded / 4 * 3).saturating_sub(padding)
                }
            }
        }),
        limit,
        max_images,
    );
    if count == 0 {
        return Ok(0);
    }
    let eligible = images(&session.messages[..consumed_prefix(&session.messages)]).len();
    // DSH may omit any requested input occurrence. Mstudio additionally protects
    // the entire newest batch until a model response; do not silently lose evidence.
    if count > eligible {
        return Err(
            "本轮新增图片超过请求图片预算；尚未送到模型的图片已保留，请减少单批图片或压缩后重试"
                .into(),
        );
    }
    media_retention::offload_oldest_images(session, host, count)
}
