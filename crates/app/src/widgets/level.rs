//! Animated microphone input meter.

use crate::theme;
use egui::{Pos2, Rect, Sense, Ui, Vec2};

pub fn show(ui: &mut Ui, active: bool) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(150.0, 22.0), Sense::hover());
    let n = 20usize;
    let gap = 2.0;
    let bw = (rect.width() / n as f32 - gap).max(2.0);
    let id = ui.id().with("lvl");

    let t = ui.input(|i| i.time) as f32;
    let target = if active { 0.25 + 0.65 * ((t * 6.0).sin() * 0.5 + 0.5).min(1.0) } else { 0.05 };
    let cur = ui.ctx().data(|d| d.get_temp::<f32>(id)).unwrap_or(0.05);
    let next = cur + (target - cur) * 0.25;
    ui.ctx().data_mut(|d| d.insert_temp(id, next));
    if active {
        ui.ctx().request_repaint();
    }

    for i in 0..n {
        let frac = i as f32 / n as f32;
        let on = frac < next;
        let col = if frac > 0.8 { theme::RED } else if frac > 0.6 { theme::ORANGE } else { theme::GREEN };
        let h = (frac * 0.9 + 0.1) * rect.height();
        let x = rect.left() + i as f32 * (bw + gap);
        let r = Rect::from_min_size(Pos2::new(x, rect.bottom() - h), Vec2::new(bw, h));
        ui.painter().rect_filled(r, egui::Rounding::same(1.0), if on { col } else { theme::CONTROL });
    }
}
