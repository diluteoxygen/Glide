//! The Raw/Live pill, now with the selected-segment fill *sliding*
//! between positions instead of snapping instantly — this was flagged as
//! feeling static/lifeless. Uses `ctx.animate_bool` to get an eased 0..1
//! progress value and lerps the highlight rect's x position from it, the
//! same pattern used for the settings-panel expand animation in `app.rs`.

use crate::theme;
use egui::{Color32, CornerRadius, Sense, Stroke, Ui, Vec2};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RecordMode {
    Raw,
    Live,
}

impl RecordMode {
    fn label(self) -> &'static str {
        match self {
            RecordMode::Raw => "Raw",
            RecordMode::Live => "Live",
        }
    }
}

const SEGMENT_SIZE: Vec2 = Vec2::new(52.0, 28.0);

/// Draws the pill and returns `Some(new_mode)` if the user clicked a
/// different segment than `current`.
pub fn show(ui: &mut Ui, current: RecordMode) -> Option<RecordMode> {
    let options = [RecordMode::Raw, RecordMode::Live];
    let total_size = Vec2::new(SEGMENT_SIZE.x * options.len() as f32 + 4.0, SEGMENT_SIZE.y + 4.0);

    let (rect, _response) = ui.allocate_exact_size(total_size, Sense::hover());

    // 0.0 = Raw selected, 1.0 = Live selected. `animate_bool` gives us an
    // eased transition for free — no manual easing curve needed.
    let anim_id = ui.id().with("record_mode_slide");
    let progress = ui.ctx().animate_bool(anim_id, current == RecordMode::Live);
    if progress > 0.0001 && progress < 0.9999 {
        ui.ctx().request_repaint();
    }

    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same((total_size.y / 2.0) as u8), theme::BG_CONTROL);

    // Single sliding highlight, not a per-segment fill toggle.
    let highlight_x = rect.left() + 2.0 + progress * SEGMENT_SIZE.x;
    let highlight_rect = egui::Rect::from_min_size(
        egui::pos2(highlight_x, rect.top() + 2.0),
        SEGMENT_SIZE,
    );
    painter.rect_filled(highlight_rect, CornerRadius::same((SEGMENT_SIZE.y / 2.0) as u8), theme::ACCENT);

    let mut clicked: Option<RecordMode> = None;
    for (i, opt) in options.iter().enumerate() {
        let seg_rect = egui::Rect::from_min_size(
            rect.min + Vec2::new(2.0 + i as f32 * SEGMENT_SIZE.x, 2.0),
            SEGMENT_SIZE,
        );
        let seg_response =
            ui.interact(seg_rect, ui.id().with(("record_mode_segment", i)), Sense::click());

        if seg_response.hovered() && *opt != current {
            ui.painter().rect_stroke(
                seg_rect,
                CornerRadius::same((SEGMENT_SIZE.y / 2.0) as u8),
                Stroke::new(1.0_f32, theme::BORDER),
                egui::StrokeKind::Inside,
            );
        }

        // Text color also crossfades with the slide instead of snapping —
        // avoids a frame where text and highlight are visually out of sync.
        // `lerp_to_gamma` exists on egui's `Color32` in current versions;
        // if it's been renamed/moved by the time you build, swap in a
        // manual per-channel lerp — the animation logic doesn't change.
        let selected_amount = if *opt == RecordMode::Live { progress } else { 1.0 - progress };
        let text_color = theme::TEXT_ON_UNSELECTED_SEGMENT.lerp_to_gamma(Color32::WHITE, selected_amount);

        ui.painter().text(
            seg_rect.center(),
            egui::Align2::CENTER_CENTER,
            opt.label(),
            egui::FontId::proportional(13.0),
            text_color,
        );

        if seg_response.clicked() {
            clicked = Some(*opt);
        }
    }

    clicked
}
