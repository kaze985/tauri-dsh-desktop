mod prefs;
mod service;
mod state;
mod update;

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use prefs::CloseAction;
use serde::Serialize;
use state::AppState;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent};
use tauri_plugin_dialog::DialogExt;

const SERVICE_URL: &str = "http://127.0.0.1:3080/";

pub enum StartKind {
    Boot,
    Retry,
    Upgrade,
}

#[derive(Clone, Serialize)]
struct ShellStatus {
    phase: &'static str,
    code: Option<&'static str>,
    message: Option<String>,
}

/// Push a shell state to the local frontend page.
fn emit(app: &AppHandle, phase: &'static str, code: Option<&'static str>, message: Option<String>) {
    let payload = ShellStatus {
        phase,
        code,
        message,
    };
    eprintln!(
        "[shell] phase={phase}{}",
        code.map(|c| format!(" code={c}")).unwrap_or_default()
    );
    let _ = app.emit("shell://status", payload);
}

fn pct(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Navigate the main window back to the local shell page with a state hash.
/// In dev this is the Vite dev server; in release it is the bundled assets
/// served on the tauri.localhost origin.
fn navigate_shell(app: &AppHandle, hash: &str) {
    if let Some(w) = app.get_webview_window("main") {
        let url = if cfg!(debug_assertions) {
            app.config()
                .build
                .dev_url
                .clone()
                .map(|d| format!("{d}/#{hash}"))
        } else {
            Some(format!("http://tauri.localhost/#{hash}"))
        };
        if let Some(u) = url.and_then(|s| tauri::Url::parse(&s).ok()) {
            let _ = w.navigate(u);
        }
    }
}

fn navigate_shell_error(app: &AppHandle, code: &str, message: &str) {
    let hash = format!("error?code={}&message={}", pct(code), pct(message));
    navigate_shell(app, &hash);
}

fn show_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// Kill the tracked dsh process tree (idempotent; called on every exit path).
fn shutdown_service(app: &AppHandle) {
    let state = app.state::<AppState>();
    let pid = {
        let mut g = state.pid.lock().unwrap();
        g.take()
    };
    if let Some(pid) = pid {
        service::kill_tree_by_pid(pid);
    }
}

fn exit_app_impl(app: &AppHandle) {
    let state = app.state::<AppState>();
    state.shutting_down.store(true, Ordering::SeqCst);
    shutdown_service(app);
    app.exit(0);
}

/// Full startup sequence: dsh check -> port precheck -> spawn -> readiness
/// poll -> navigate to the service UI. Emits shell states along the way.
pub async fn startup_sequence(app: &AppHandle, _kind: StartKind) {
    emit(app, "loading", None, Some("正在启动 dsh 服务…".to_string()));

    if service::dsh_version().is_none() {
        emit(app, "setup", None, None);
        return;
    }

    if service::port_busy() {
        emit(
            app,
            "error",
            Some("port-occupied"),
            Some("3080 端口已被占用。请先关闭占用该端口的程序，然后重试。".to_string()),
        );
        return;
    }

    let mut child = match service::spawn_dsh() {
        Ok(c) => c,
        Err(e) => {
            emit(
                app,
                "error",
                Some("spawn-failed"),
                Some(format!("无法启动 dsh：{e}")),
            );
            return;
        }
    };

    let deadline = Instant::now() + Duration::from_secs(service::READY_TIMEOUT_SECS);
    let mut ready = false;
    while Instant::now() < deadline {
        if service::port_busy() {
            ready = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(service::POLL_INTERVAL_MS)).await;
    }

    if !ready {
        service::kill_child_tree(&mut child);
        emit(
            app,
            "error",
            Some("startup-timeout"),
            Some("dsh 在 30 秒内未能就绪，请重试。".to_string()),
        );
        return;
    }

    {
        let state = app.state::<AppState>();
        let mut g = state.pid.lock().unwrap();
        *g = Some(child.id());
    }

    emit(app, "ready", None, None);
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.navigate(tauri::Url::parse(SERVICE_URL).unwrap());
    }

    spawn_watchdog(app.clone(), child);
}

/// Watch the dsh process; on unexpected exit (not shutdown, not upgrade),
/// bring the window back to the local stopped page.
fn spawn_watchdog(app: AppHandle, mut child: std::process::Child) {
    let pid = child.id();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            if let Ok(Some(_status)) = child.try_wait() {
                let state = app.state::<AppState>();
                {
                    let mut g = state.pid.lock().unwrap();
                    // Only clear if it is still OUR child: a restart may have
                    // already replaced the pid with the new process.
                    if *g == Some(pid) {
                        *g = None;
                    }
                }
                let shutting = state.shutting_down.load(Ordering::SeqCst);
                let upgrading = state.upgrading.load(Ordering::SeqCst);
                if !shutting && !upgrading {
                    emit(&app, "stopped", None, None);
                    navigate_shell(&app, "stopped");
                }
                break;
            }
        }
    });
}

#[tauri::command]
fn retry_startup(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        shutdown_service(&app);
        startup_sequence(&app, StartKind::Retry).await;
    });
}

#[tauri::command]
fn exit_app(app: AppHandle) {
    exit_app_impl(&app);
}

#[tauri::command]
fn choose_close(app: AppHandle, action: String, remember: bool) {
    let state = app.state::<AppState>();
    let act = match action.as_str() {
        "exit" => CloseAction::Exit,
        "tray" => CloseAction::Tray,
        _ => CloseAction::Ask,
    };
    if remember {
        let mut p = state.prefs.lock().unwrap();
        p.close_action = act;
        prefs::save(&app, &p);
    }
    match act {
        CloseAction::Exit => exit_app_impl(&app),
        CloseAction::Tray => {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.hide();
            }
        }
        CloseAction::Ask => {}
    }
}

#[tauri::command]
fn install_dsh(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        emit(
            &app,
            "loading",
            None,
            Some("正在安装 dsh，可能需要几分钟…".to_string()),
        );
        let ok = service::run_wait("npm", &["i", "-g", "@deepseek-ai/dsh@latest"], 600);
        if ok {
            startup_sequence(&app, StartKind::Retry).await;
        } else {
            emit(
                &app,
                "error",
                Some("install-failed"),
                Some("dsh 安装失败，请检查网络后重试。".to_string()),
            );
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            show_window(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            retry_startup,
            exit_app,
            choose_close,
            install_dsh
        ])
        .setup(|app| {
            // Load persisted close behavior.
            let loaded = prefs::load(app.handle());
            {
                let state = app.state::<AppState>();
                *state.prefs.lock().unwrap() = loaded;
            }

            // Tray: show window / check update / reset close behavior / exit.
            let show_i = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
            let check_i = MenuItem::with_id(app, "check-update", "检查更新", true, None::<&str>)?;
            let reset_i =
                MenuItem::with_id(app, "reset-close", "重置关闭行为", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &check_i, &reset_i, &quit_i])?;
            let tray = TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().expect("window icon").clone())
                .tooltip("DSH Desktop")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_window(app),
                    "quit" => exit_app_impl(app),
                    "check-update" => {
                        let a = app.clone();
                        tauri::async_runtime::spawn(async move {
                            update::check_and_maybe_notify(a, true).await;
                        });
                    }
                    "reset-close" => {
                        let state = app.state::<AppState>();
                        {
                            let mut p = state.prefs.lock().unwrap();
                            p.close_action = CloseAction::Ask;
                            prefs::save(app, &p);
                        }
                        app.dialog()
                            .message("已重置：下次关闭窗口时将再次询问。")
                            .show(|_| {});
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_window(tray.app_handle());
                    }
                })
                .build(app)?;
            // The TrayIcon handle must stay alive for the app's lifetime,
            // otherwise dropping it (end of setup) may remove the icon.
            std::mem::forget(tray);

            // Startup sequence (slight delay lets the shell page register its
            // event listener before the first state is pushed).
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(Duration::from_millis(500)).await;
                startup_sequence(&handle, StartKind::Boot).await;
            });

            // Background update check (auto mode: silent on failure).
            let h2 = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(Duration::from_secs(8)).await;
                update::check_and_maybe_notify(h2, false).await;
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<AppState>();
                if state.shutting_down.load(Ordering::SeqCst) {
                    return;
                }
                api.prevent_close();
                let action = state.prefs.lock().unwrap().close_action;
                match action {
                    CloseAction::Ask => navigate_shell(window.app_handle(), "close-dialog"),
                    CloseAction::Tray => {
                        let _ = window.hide();
                    }
                    CloseAction::Exit => exit_app_impl(window.app_handle()),
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let RunEvent::ExitRequested { .. } = event {
                shutdown_service(app_handle);
            }
        });
}
