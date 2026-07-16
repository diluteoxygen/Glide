//! System-tray icon + menu. Left-click (or "Show Glide") re-opens the window;
//! "Quit Glide" forces a real exit even when minimize-to-tray is on.

use egui::{Context, ViewportCommand};
use image::{ImageBuffer, Rgba};
use std::sync::{atomic::AtomicBool, OnceLock};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem},
    Icon, TrayIcon, TrayIconBuilder, TrayIconEvent,
};

static CTX: OnceLock<Context> = OnceLock::new();
struct Ids {
    show: MenuId,
    record: MenuId,
    quit: MenuId,
}
static IDS: OnceLock<Ids> = OnceLock::new();

/// Set true by the tray "Quit" menu so the close handler lets the app exit.
pub static FORCE_QUIT: AtomicBool = AtomicBool::new(false);
/// Set true by the tray "Start/Stop" menu; the app drains & toggles recording.
pub static PENDING_RECORD: AtomicBool = AtomicBool::new(false);

pub fn init(ctx: &Context) -> TrayIcon {
    let _ = CTX.set(ctx.clone());

    let menu = Menu::new();
    let show = MenuItem::new("Show Glide", true, None);
    let record = MenuItem::new("Start / Stop recording", true, None);
    let quit = MenuItem::new("Quit Glide", true, None);
    let _ = IDS.set(Ids {
        show: show.id().clone(),
        record: record.id().clone(),
        quit: quit.id().clone(),
    });
    let _ = menu.append(&show);
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&record);
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&quit);

    TrayIconBuilder::new()
        .with_tooltip("Glide")
        .with_icon(make_icon())
        .with_menu(Box::new(menu))
        .build()
        .expect("failed to build tray icon")
}

/// Call each frame from `update` to react to tray interactions.
pub fn pump(ctx: &Context) {
    // Menu selections
    while let Ok(ev) = MenuEvent::receiver().try_recv() {
        if let Some(ids) = IDS.get() {
            if &ev.id == &ids.show {
                show_window();
            } else if &ev.id == &ids.record {
                PENDING_RECORD.store(true, std::sync::atomic::Ordering::SeqCst);
                ctx.request_repaint();
            } else if &ev.id == &ids.quit {
                FORCE_QUIT.store(true, std::sync::atomic::Ordering::SeqCst);
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
        }
    }
    // Icon clicks (left-click reopens)
    while let Ok(ev) = TrayIconEvent::receiver().try_recv() {
        match ev {
            TrayIconEvent::Click { button: tray_icon::MouseButton::Left, button_state: tray_icon::MouseButtonState::Up, .. } =>
            {
                show_window();
            }
            TrayIconEvent::DoubleClick { button: tray_icon::MouseButton::Left, .. } => {
                show_window();
            }
            _ => {}
        }
    }
}

fn show_window() {
    if let Some(ctx) = CTX.get() {
        ctx.send_viewport_cmd(ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(ViewportCommand::Focus);
        ctx.request_repaint();
    }
}

/// 32x32 app-style icon drawn procedurally (no asset file needed).
fn make_icon() -> Icon {
    let s = 32u32;
    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(s, s);
    let r = 9.0f32;
    let cx = (s / 2) as f32;
    let cy = (s / 2) as f32;
    for y in 0..s {
        for x in 0..s {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let inside = rounded_rect_contains(px, py, s as f32, s as f32, r);
            if !inside {
                img.put_pixel(x, y, Rgba([0, 0, 0, 0]));
                continue;
            }
            // diagonal blue->indigo gradient
            let t = (x + y) as f32 / (2 * s) as f32;
            let cr = (10.0 + t * 100.0) as u8;
            let cg = (132.0 - t * 40.0) as u8;
            let cb = (255.0 - t * 5.0) as u8;
            // white aperture dot in the center
            let d = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
            if d < 5.0 {
                img.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            } else if d < 8.5 {
                img.put_pixel(x, y, Rgba([cr, cg, cb, 255]));
            } else {
                img.put_pixel(x, y, Rgba([cr, cg, cb, 255]));
            }
        }
    }
    Icon::from_rgba(img.into_raw(), s, s).expect("bad icon rgba")
}

fn rounded_rect_contains(x: f32, y: f32, w: f32, h: f32, r: f32) -> bool {
    // distance to nearest rounded-rect edge corner
    let dx = (x - r).max(0.0).min(w - r);
    let dy = (y - r).max(0.0).min(h - r);
    (x - dx) * (x - dx) + (y - dy) * (y - dy) <= r * r
}
