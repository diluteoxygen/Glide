//! Equal-width segmented control with a sliding highlight.
//! Returns Some(i) when the user picks a different segment.

use crate::theme;
use egui::{CursorIcon, FontId, Pos2, Rect, Rounding, Sense, Stroke, Ui};

pub fn show(ui: &mut Ui, id: &str, labels: &[&str], current: usize) -> Option<usize> {
    let pad = 2.0;
    let h = 26.0;
    let seg_w = 58.0;
    let n = labels.len().max(1);
    let total = egui::Vec2::new(seg_w * n as f32 + pad * 2.0, h + pad * 2.0);

    let (rect, _resp) = ui.allocate_exact_size(total, Sense::hover());
    let p = ui.painter();

    p.rect_filled(rect, Rounding::same(total.y / 2.0), theme::CONTROL);

    // smoothed animated index
    let aid = ui.id().with(id);
    let dt = ui.input(|i| i.stable_dt).clamp(0.001, 0.05);
    let target = current as f32;
    let cur = ui.ctx().data(|d| d.get_temp::<f32>(aid)).unwrap_or(target);
    let next = cur + (target - cur) * (1.0_f32 - (-16.0_f32 * dt).exp());
    ui.ctx().data_mut(|d| d.insert_temp(aid, next));
    if (next - target).abs() > 0.002 {
        ui.ctx().request_repaint();
    }

    // sliding white thumb
    let hl = Rect::from_min_size(
        Pos2::new(rect.left() + pad + next * seg_w, rect.top() + pad),
        egui::Vec2::new(seg_w, h),
    );
    p.rect_filled(hl, Rounding::same(h / 2.0), theme::CARD);
    p.rect_stroke(hl, Rounding::same(h / 2.0), Stroke::new(0.5, theme::HAIR));

    let mut picked = None;
    for (i, label) in labels.iter().enumerate() {
        let sr = Rect::from_min_size(
            Pos2::new(rect.left() + pad + i as f32 * seg_w, rect.top() + pad),
            egui::Vec2::new(seg_w, h),
        );
        let r = ui.interact(sr, ui.id().with((id, i)), Sense::click());
        let sel = i == current;
        let color = if sel { theme::TEXT1 } else { theme::TEXT2 };
        p.text(sr.center(), egui::Align2::CENTER_CENTER, label, FontId::proportional(12.5), color);
        if r.clicked() {
            picked = Some(i);
        }
        r.on_hover_cursor(CursorIcon::PointingHand);
    }
    picked
}
