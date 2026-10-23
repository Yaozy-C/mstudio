use super::context;
use crate::assistant::config::Profile;
use anyhow::{Result, ensure};

pub fn validate_policy(profile: &Profile) -> Result<()> {
    let output = profile.limits.max_output_tokens;
    ensure!(
        profile.adapter != "codex" || output.is_none(),
        "Codex 的输出上限由本机服务管理"
    );
    ensure!(
        output.is_none_or(|n| (1..=2_000_000).contains(&n)),
        "输出上限须为 1 到 2000000 token"
    );
    ensure!(
        profile
            .limits
            .compaction_headroom
            .is_none_or(|n| n <= 2_000_000),
        "压缩余量须在 0 到 2000000 token 之间"
    );
    ensure!(
        profile.limits.context_window_mode != Some(context::Mode::Manual)
            || profile.context_window.is_some(),
        "请填写手动上下文窗口，或切换为自动获取"
    );
    if let Some(maximum) = context::automatic(profile).and_then(|limits| limits.max_output_tokens) {
        ensure!(
            output.is_none_or(|n| n <= maximum),
            "输出上限超过此模型公布的输出能力"
        );
    }
    if let Some(window) = profile.context_window() {
        ensure!(
            context::threshold(profile, window).is_some(),
            "上下文窗口不足以容纳输出预留和压缩余量，请调整这三项设置"
        );
    }
    Ok(())
}

/// Metadata discovery may start with unknown limits; model execution may not.
pub fn validate_execution(profile: &Profile) -> Result<()> {
    validate_policy(profile)?;
    ensure!(
        profile.adapter == "codex" || profile.context_window().is_some(),
        "未获取到此模型的上下文容量，请在模型中心刷新容量或手动填写上下文窗口"
    );
    ensure!(
        profile.adapter != "anthropic-native" || context::output_cap(profile).is_some(),
        "此 Anthropic 型号未公布默认输出上限，请手动填写输出上限"
    );
    Ok(())
}
