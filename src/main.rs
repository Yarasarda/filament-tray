#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    collections::HashSet,
    fs,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::Duration,
};

use serde::{Deserialize, Serialize};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, PhysicalPosition, State, WebviewWindow,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_notification::NotificationExt;

#[derive(Clone, Serialize, Deserialize)]
struct Config {
    spoolman_url: String,
    threshold_g: f64,
    interval_s: u64,
    #[serde(default)]
    autostart_prompted: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            spoolman_url: "http://192.168.1.123:7912".into(),
            threshold_g: 100.0,
            interval_s: 30,
            autostart_prompted: false,
        }
    }
}

#[derive(Clone, Serialize)]
struct Slot {
    slot: u8,
    spool_id: i64,
    name: String,
    material: String,
    vendor: String,
    color: String,
    remaining: f64,
    initial: f64,
    percent: f64,
}

#[derive(Clone, Serialize)]
struct Snapshot {
    slots: Vec<Slot>,
    error: Option<String>,
}

struct AppState {
    config: Mutex<Config>,
    notified: Mutex<HashSet<i64>>,
    tray_hover: AtomicBool,
    popup_hover: AtomicBool,
}

fn config_path(app: &AppHandle) -> Option<std::path::PathBuf> {
    let dir = app.path().app_config_dir().ok()?;
    fs::create_dir_all(&dir).ok()?;
    Some(dir.join("config.json"))
}

fn load_config(app: &AppHandle) -> Config {
    config_path(app)
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// "AMS 1", "Slot 2", "Yuva 3", "4" -> 1..=4
fn parse_slot(loc: &str) -> Option<u8> {
    let digits: String = loc.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.parse::<u8>().ok().filter(|n| (1..=4).contains(n))
}

async fn fetch_slots(cfg: &Config) -> Result<Vec<Slot>, String> {
    let url = format!("{}/api/v1/spool", cfg.spoolman_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|e| e.to_string())?;
    let spools: Vec<serde_json::Value> = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Spoolman'a ulaşılamadı: {e}"))?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let mut slots: Vec<Slot> = spools
        .iter()
        .filter(|s| !s["archived"].as_bool().unwrap_or(false))
        .filter_map(|s| {
            let slot = parse_slot(s["location"].as_str()?)?;
            let f = &s["filament"];
            let initial = s["initial_weight"]
                .as_f64()
                .or_else(|| f["weight"].as_f64())
                .unwrap_or(0.0);
            let remaining = s["remaining_weight"].as_f64().unwrap_or(0.0);
            let percent = if initial > 0.0 {
                (remaining / initial * 100.0).clamp(0.0, 100.0)
            } else {
                0.0
            };
            Some(Slot {
                slot,
                spool_id: s["id"].as_i64()?,
                name: f["name"].as_str().unwrap_or("?").to_string(),
                material: f["material"].as_str().unwrap_or("").to_string(),
                vendor: f["vendor"]["name"].as_str().unwrap_or("").to_string(),
                color: f["color_hex"].as_str().unwrap_or("888888").to_string(),
                remaining,
                initial,
                percent,
            })
        })
        .collect();
    slots.sort_by_key(|s| s.slot);
    Ok(slots)
}

fn tooltip_text(snap: &Snapshot) -> String {
    if let Some(e) = &snap.error {
        return format!("Filament: {e}");
    }
    if snap.slots.is_empty() {
        return "Filament: AMS yuvalarında makara yok".into();
    }
    snap.slots
        .iter()
        .map(|s| format!("{}: {} {:.0} g", s.slot, s.name, s.remaining))
        .collect::<Vec<_>>()
        .join("\n")
}

async fn refresh(app: &AppHandle) -> Snapshot {
    let state: State<AppState> = app.state();
    let cfg = state.config.lock().unwrap().clone();
    let snap = match fetch_slots(&cfg).await {
        Ok(slots) => Snapshot { slots, error: None },
        Err(e) => Snapshot { slots: vec![], error: Some(e) },
    };

    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(tooltip_text(&snap)));
    }

    {
        let mut notified = state.notified.lock().unwrap();
        for s in &snap.slots {
            if s.remaining < cfg.threshold_g {
                if notified.insert(s.spool_id) {
                    let _ = app
                        .notification()
                        .builder()
                        .title("Filament azalıyor")
                        .body(format!(
                            "AMS {}: {} sadece {:.0} g kaldı",
                            s.slot, s.name, s.remaining
                        ))
                        .show();
                }
            } else {
                notified.remove(&s.spool_id);
            }
        }
    }

    let _ = app.emit("snapshot", snap.clone());
    snap
}

#[tauri::command]
async fn get_snapshot(app: AppHandle) -> Snapshot {
    refresh(&app).await
}

#[tauri::command]
fn get_config(state: State<AppState>) -> Config {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
async fn set_config(app: AppHandle, state: State<'_, AppState>, config: Config) -> Result<(), String> {
    *state.config.lock().unwrap() = config.clone();
    if let Some(p) = config_path(&app) {
        let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
        fs::write(p, json).map_err(|e| e.to_string())?;
    }
    refresh(&app).await;
    Ok(())
}

fn place_near(window: &WebviewWindow, click: PhysicalPosition<f64>) {
    let Ok(size) = window.outer_size() else { return };
    let (w, h) = (size.width as f64, size.height as f64);
    let mut x = click.x - w / 2.0;
    // Menü çubuğu üstteyse (macOS) aşağı, görev çubuğu altta ise yukarı aç.
    let mut y = if click.y < 200.0 { click.y + 12.0 } else { click.y - h - 12.0 };
    if let Ok(Some(mon)) = window.current_monitor() {
        let mp = mon.position();
        let ms = mon.size();
        x = x.clamp(mp.x as f64, (mp.x as f64 + ms.width as f64 - w).max(mp.x as f64));
        y = y.clamp(mp.y as f64, (mp.y as f64 + ms.height as f64 - h).max(mp.y as f64));
    }
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

/// Odağı çalmadan popup'ı göster (hover için).
fn show_popup(app: &AppHandle, click: Option<PhysicalPosition<f64>>) {
    let Some(w) = app.get_webview_window("main") else { return };
    if !w.is_visible().unwrap_or(false) {
        if let Some(p) = click {
            place_near(&w, p);
        }
        let _ = w.show();
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        refresh(&app).await;
    });
}

/// Fare ne ikonun ne de popup'ın üstündeyse kısa gecikmeyle gizle.
fn schedule_hide(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(350)).await;
        let st: State<AppState> = app.state();
        if !st.tray_hover.load(Ordering::Relaxed) && !st.popup_hover.load(Ordering::Relaxed) {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.hide();
            }
        }
    });
}

#[tauri::command]
fn set_popup_hover(app: AppHandle, state: State<AppState>, inside: bool) {
    state.popup_hover.store(inside, Ordering::Relaxed);
    if !inside {
        schedule_hide(&app);
    }
}

#[tauri::command]
fn is_autostart_enabled(app: AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
fn enable_autostart(app: AppHandle) -> Result<(), String> {
    app.autolaunch().enable().map_err(|e| e.to_string())
}

#[tauri::command]
fn disable_autostart(app: AppHandle) -> Result<(), String> {
    app.autolaunch().disable().map_err(|e| e.to_string())
}

fn toggle_window(app: &AppHandle, click: Option<PhysicalPosition<f64>>) {
    let Some(w) = app.get_webview_window("main") else { return };
    if w.is_visible().unwrap_or(false) {
        let _ = w.hide();
    } else {
        if let Some(p) = click {
            place_near(&w, p);
        }
        let _ = w.show();
        let _ = w.set_focus();
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            refresh(&app).await;
        });
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            get_config,
            set_config,
            set_popup_hover,
            is_autostart_enabled,
            enable_autostart,
            disable_autostart,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            app.manage(AppState {
                config: Mutex::new(load_config(&handle)),
                notified: Mutex::new(HashSet::new()),
                tray_hover: AtomicBool::new(false),
                popup_hover: AtomicBool::new(false),
            });

            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let show = MenuItem::with_id(app, "show", "Göster", true, None::<&str>)?;
            let reload = MenuItem::with_id(app, "refresh", "Yenile", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Çık", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &reload, &quit])?;

            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Filament Tray")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => toggle_window(app, None),
                    "refresh" => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            refresh(&app).await;
                        });
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    let app = tray.app_handle();
                    let st: State<AppState> = app.state();
                    match event {
                        // Windows / macOS: fare ikonun üstüne gelince göster.
                        TrayIconEvent::Enter { position, .. } => {
                            st.tray_hover.store(true, Ordering::Relaxed);
                            show_popup(app, Some(position));
                        }
                        TrayIconEvent::Leave { .. } => {
                            st.tray_hover.store(false, Ordering::Relaxed);
                            schedule_hide(app);
                        }
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            position,
                            ..
                        } => toggle_window(app, Some(position)),
                        _ => {}
                    }
                })
                .build(app)?;

            let show_on_start = !load_config(&handle).autostart_prompted;
            if let Some(w) = app.get_webview_window("main") {
                if show_on_start {
                    let _ = w.show();
                    let _ = w.center();
                    let _ = w.set_focus();
                }
                let w2 = w.clone();
                w.on_window_event(move |e| {
                    if let tauri::WindowEvent::Focused(false) = e {
                        let _ = w2.hide();
                    }
                });
            }

            // Arka plan döngüsü: tooltip + bildirim
            tauri::async_runtime::spawn(async move {
                loop {
                    refresh(&handle).await;
                    let secs = {
                        let st: State<AppState> = handle.state();
                        let s = st.config.lock().unwrap().interval_s;
                        s.max(5)
                    };
                    tokio::time::sleep(Duration::from_secs(secs)).await;
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("tauri çalıştırılamadı");
}
