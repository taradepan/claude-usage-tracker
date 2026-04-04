use crate::format::{format_time_until, reset_suffix, usage_bar};
use crate::types::UiUpdate;
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::{Icon, TrayIconBuilder};

enum UserEvent {
    MenuEvent(MenuEvent),
    TrayIconEvent(()),
}

struct MenuItems {
    session_label: MenuItem,
    session_bar: MenuItem,
    weekly_label: MenuItem,
    extra_label: MenuItem,
    plan_label: MenuItem,
    refresh: MenuItem,
    open_usage: MenuItem,
    quit: MenuItem,
}

fn load_icon() -> Icon {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));

    let candidates = [
        exe_dir.as_ref().map(|d| d.join("icon.png")),
        Some(std::path::PathBuf::from("icon.png")),
    ];

    for candidate in candidates.iter().flatten() {
        if candidate.exists()
            && let Ok(data) = std::fs::read(candidate)
            && let Ok(img) = image::load_from_memory(&data)
        {
            let rgba = img.to_rgba8();
            let (w, h) = (rgba.width(), rgba.height());
            if let Ok(icon) = Icon::from_rgba(rgba.into_raw(), w, h) {
                return icon;
            }
        }
    }

    // Minimal 1x1 transparent fallback
    Icon::from_rgba(vec![0, 0, 0, 0], 1, 1).unwrap()
}

fn build_menu() -> (Menu, MenuItems) {
    let session_label = MenuItem::new("Session (5h): loading...", true, None);
    let session_bar = MenuItem::new("", true, None);
    let weekly_label = MenuItem::new("Weekly (7d): loading...", true, None);
    let extra_label = MenuItem::new("", true, None);
    let plan_label = MenuItem::new("", true, None);
    let refresh = MenuItem::new("Refresh Now", true, None);
    let open_usage = MenuItem::new("Open Usage Page", true, None);
    let quit = MenuItem::new("Quit", true, None);

    let menu = Menu::new();
    let _ = menu.append(&session_label);
    let _ = menu.append(&session_bar);
    let _ = menu.append(&weekly_label);
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&extra_label);
    let _ = menu.append(&plan_label);
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&refresh);
    let _ = menu.append(&open_usage);
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&quit);

    (
        menu,
        MenuItems {
            session_label,
            session_bar,
            weekly_label,
            extra_label,
            plan_label,
            refresh,
            open_usage,
            quit,
        },
    )
}

pub fn run(rx: Receiver<UiUpdate>, refresh_trigger: Arc<AtomicBool>) {
    use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};

    let mut event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    event_loop.set_activation_policy(ActivationPolicy::Accessory);

    // Forward menu and tray events into the event loop
    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::MenuEvent(event));
    }));

    let proxy = event_loop.create_proxy();
    tray_icon::TrayIconEvent::set_event_handler(Some(move |_event| {
        let _ = proxy.send_event(UserEvent::TrayIconEvent(()));
    }));

    let icon = load_icon();
    let (menu, items) = build_menu();

    let mut _tray_icon: Option<tray_icon::TrayIcon> = None;

    event_loop.run(move |event, _event_loop, control_flow| {
        *control_flow = ControlFlow::WaitUntil(
            std::time::Instant::now() + std::time::Duration::from_millis(200),
        );

        match event {
            Event::NewEvents(StartCause::Init) => {
                // Create tray icon during event loop initialization
                _tray_icon = Some(
                    TrayIconBuilder::new()
                        .with_icon(icon.clone())
                        .with_icon_as_template(true)
                        .with_menu(Box::new(menu.clone()))
                        .with_tooltip("Claude Usage")
                        .with_title(" ...")
                        .build()
                        .expect("failed to create tray icon"),
                );

                // Wake up the CFRunLoop so the icon appears immediately
                #[cfg(target_os = "macos")]
                {
                    use objc2_core_foundation::CFRunLoop;
                    let rl = CFRunLoop::main().unwrap();
                    rl.wake_up();
                }
            }

            Event::UserEvent(UserEvent::MenuEvent(event)) => {
                if event.id == *items.refresh.id() {
                    refresh_trigger.store(true, Ordering::Relaxed);
                    if let Some(tray) = &_tray_icon {
                        tray.set_title(Some(" ..."));
                    }
                    items.session_label.set_text("Session (5h): loading...");
                    items.session_bar.set_text("");
                    items.weekly_label.set_text("Weekly (7d): loading...");
                    items.extra_label.set_text("");
                    items.plan_label.set_text("");
                } else if event.id == *items.open_usage.id() {
                    let _ = std::process::Command::new("open")
                        .arg("https://claude.ai/settings/usage")
                        .spawn();
                } else if event.id == *items.quit.id() {
                    std::process::exit(0);
                }
            }

            // On each tick (every 200ms), check for UI updates from poller
            Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                while let Ok(update) = rx.try_recv() {
                    if let Some(tray) = &_tray_icon {
                        apply_update(tray, &items, &update);
                    }
                }
            }

            _ => {}
        }
    });
}

fn set_error(tray: &tray_icon::TrayIcon, items: &MenuItems, title: &str, detail: &str, bar: &str) {
    tray.set_title(Some(title));
    items.session_label.set_text(detail);
    items.session_bar.set_text(bar);
}

fn apply_update(tray: &tray_icon::TrayIcon, items: &MenuItems, update: &UiUpdate) {
    match update {
        UiUpdate::UsageData { usage, plan } => {
            let (title, session_text, bar_text) = match &usage.five_hour {
                Some(fh) if fh.utilization.is_some() => {
                    let pct = fh.utilization.unwrap();
                    let rpct = pct.round() as u64;
                    let reset = fh.resets_at.as_deref().unwrap_or("");
                    let reset_time = format_time_until(reset);
                    let suffix = if reset_time.is_empty() {
                        String::new()
                    } else {
                        format!("  |  resets in {}", reset_time)
                    };
                    let title = if reset_time.is_empty() {
                        format!(" {}%", rpct)
                    } else {
                        format!(" {}%  {}", rpct, reset_time)
                    };
                    let session = format!("Session (5h): {}%{}", rpct, suffix);
                    let bar = format!("  {}", usage_bar(pct, 20));
                    (title, session, bar)
                }
                _ => (
                    " OK".to_string(),
                    "Session (5h): no limit".to_string(),
                    String::new(),
                ),
            };

            let weekly_text = match &usage.seven_day {
                Some(sd) if sd.utilization.is_some() => {
                    let pct = sd.utilization.unwrap().round() as u64;
                    let reset = sd.resets_at.as_deref().unwrap_or("");
                    format!("Weekly (7d): {}%{}", pct, reset_suffix(reset))
                }
                _ => "Weekly (7d): no limit".to_string(),
            };

            let extra_text = match &usage.extra_usage {
                Some(ex) if ex.is_enabled && ex.used_credits.is_some() => {
                    let used = ex.used_credits.unwrap();
                    match ex.monthly_limit {
                        Some(limit) => format!("Extra usage: ${:.2} / ${:.2}", used, limit),
                        None => format!("Extra usage: ${:.2} (no cap)", used),
                    }
                }
                _ => String::new(),
            };

            let plan_text = if plan.is_empty() {
                String::new()
            } else {
                format!("Plan: {}", plan)
            };

            tray.set_title(Some(&title));
            items.session_label.set_text(&session_text);
            items.session_bar.set_text(&bar_text);
            items.weekly_label.set_text(&weekly_text);
            items.extra_label.set_text(&extra_text);
            items.plan_label.set_text(&plan_text);
        }
        UiUpdate::NotSignedIn => {
            tray.set_title(Some(" --"));
            items.session_label.set_text("Not signed in to Claude Code");
            items.session_bar.set_text("Run `claude` in terminal to authenticate");
            items.weekly_label.set_text("");
            items.extra_label.set_text("");
            items.plan_label.set_text("");
        }
        UiUpdate::TokenExpired => {
            tray.set_title(Some(" exp"));
            items.session_label.set_text("OAuth token expired");
            items.session_bar.set_text("Run `claude` to re-authenticate");
            items.weekly_label.set_text("");
            items.extra_label.set_text("");
            items.plan_label.set_text("");
        }
        UiUpdate::RateLimited { retry_in_secs } => {
            let wait = if *retry_in_secs >= 60 {
                format!("{}m", retry_in_secs / 60)
            } else {
                format!("{}s", retry_in_secs)
            };
            set_error(tray, items, " 429", "Rate limited", &format!("Retrying in {}...", wait));
        }
        UiUpdate::AuthError => {
            set_error(tray, items, " err", "Error: authentication failed", "Will retry next poll");
        }
        UiUpdate::NetworkError(msg) => {
            set_error(tray, items, " err", &format!("Error: {}", msg), "Will retry next poll");
        }
        UiUpdate::HttpError(code) => {
            set_error(tray, items, " err", &format!("Error: HTTP {}", code), "Will retry next poll");
        }
    }
}
