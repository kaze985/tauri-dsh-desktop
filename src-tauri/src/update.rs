//! Version management: registry check and user-confirmed upgrade.

use std::cmp::Ordering;

use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
use tokio::sync::oneshot;

use crate::service;

/// Parse a dsh version string like "0.1.0-rc.7" into a comparable tuple.
/// The fourth element is true for a release and false for a prerelease;
/// per semver, a release sorts after prereleases of the same core version.
fn parse(s: &str) -> Option<(u64, u64, u64, bool, u64)> {
    let s = s.trim().trim_start_matches('v');
    let (core, pre) = match s.split_once('-') {
        Some((c, p)) => (c, Some(p)),
        None => (s, None),
    };
    let mut it = core.split('.');
    let maj: u64 = it.next()?.parse().ok()?;
    let min: u64 = it.next()?.parse().ok()?;
    let pat: u64 = it.next()?.parse().ok()?;
    let (is_release, pre_num) = match pre {
        None => (true, 0),
        Some(p) => {
            let digits: String = p.chars().filter(|c| c.is_ascii_digit()).collect();
            (false, digits.parse().ok()?)
        }
    };
    Some((maj, min, pat, is_release, pre_num))
}

pub fn compare(a: &str, b: &str) -> Ordering {
    match (parse(a), parse(b)) {
        (Some(x), Some(y)) => x.cmp(&y),
        _ => a.trim().cmp(b.trim()),
    }
}

/// Latest version published to the npm registry (None on any failure).
pub fn registry_latest() -> Option<String> {
    service::run_out("npm", &["view", "@deepseek-ai/dsh", "version"], 30)
}

pub fn installed_version() -> Option<String> {
    service::dsh_version()
}

/// Notify only when a newer version exists; silent on failure (auto mode),
/// or report the outcome (manual mode, invoked from the tray menu).
pub async fn check_and_maybe_notify(app: AppHandle, manual: bool) {
    let Some(installed) = installed_version() else {
        if manual {
            show_info(&app, "无法读取当前安装的 dsh 版本。");
        }
        return;
    };
    let Some(latest) = registry_latest() else {
        if manual {
            show_info(&app, "检查失败：无法访问 npm registry，请检查网络。");
        }
        return;
    };
    match compare(&installed, &latest) {
        Ordering::Less => {
            let (tx, rx) = oneshot::channel();
            app.dialog()
                .message(format!(
                    "发现新版本：{installed} → {latest}\n\n升级将重启 dsh 服务，进行中的会话会中断。"
                ))
                .title("DSH Desktop 更新")
                .buttons(MessageDialogButtons::OkCancelCustom(
                    "立即升级".into(),
                    "暂不".into(),
                ))
                .show(move |ok| {
                    let _ = tx.send(ok);
                });
            if rx.await.unwrap_or(false) {
                upgrade(&app, &latest).await;
            }
        }
        Ordering::Equal | Ordering::Greater => {
            if manual {
                show_info(&app, &format!("已是最新版本（{installed}）。"));
            }
        }
    }
}

fn show_info(app: &AppHandle, text: &str) {
    app.dialog().message(text).show(|_| {});
}

/// User-confirmed upgrade: install the target version, then restart the
/// service so the new version takes effect.
pub async fn upgrade(app: &AppHandle, target: &str) {
    let state = app.state::<crate::state::AppState>();
    state
        .upgrading
        .store(true, std::sync::atomic::Ordering::SeqCst);

    crate::emit(
        app,
        "loading",
        None,
        Some("正在升级 dsh，请稍候…".to_string()),
    );
    crate::navigate_shell(app, "upgrading");

    let ok = service::run_wait(
        "npm",
        &["i", "-g", &format!("@deepseek-ai/dsh@{target}")],
        600,
    );

    if !ok {
        state
            .upgrading
            .store(false, std::sync::atomic::Ordering::SeqCst);
        crate::navigate_shell_error(
            app,
            "upgrade-failed",
            &format!("dsh 升级到 {target} 失败，请检查网络后重试。"),
        );
        return;
    }

    // The old dsh process still holds port 3080: kill its whole tree first so
    // the restart below can rebind. Keep the upgrading flag set while the
    // watchdog may observe the old child's exit, so it does not flip to the
    // stopped page mid-restart; clear it only after the new service is up.
    crate::shutdown_service(app);
    crate::startup_sequence(app, crate::StartKind::Upgrade).await;
    state
        .upgrading
        .store(false, std::sync::atomic::Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_rc_versions() {
        assert_eq!(compare("0.1.0-rc.7", "0.1.0-rc.8"), Ordering::Less);
        assert_eq!(compare("0.1.0-rc.8", "0.1.0-rc.7"), Ordering::Greater);
        assert_eq!(compare("0.1.0-rc.7", "0.1.0-rc.7"), Ordering::Equal);
    }

    #[test]
    fn release_beats_prerelease() {
        assert_eq!(compare("0.1.0-rc.8", "0.1.0"), Ordering::Less);
        assert_eq!(compare("0.1.0", "0.1.0-rc.8"), Ordering::Greater);
    }

    #[test]
    fn handles_v_prefix() {
        assert_eq!(compare("v0.1.0-rc.7", "0.1.0-rc.8"), Ordering::Less);
    }
}
