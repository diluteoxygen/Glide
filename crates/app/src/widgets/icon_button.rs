//! Toolbar icon button: pointer cursor, hover/active background, tooltip, and
//! crucially an *image* glyph (not selectable text) — fixes the
//! "users can select the icons" bug.

use crate::icons;
use crate::theme;
use egui::{CursorIcon, Response, Sense, Stroke, Ui, Vec2, Widget};

pub struct IconButton {
    pub icon: &'static str,
    pub size: f32,
    pub active: bool,
    pub danger: bool,
    pub tooltip: Option<&'static str>,
}

impl Default for IconButton {
    fn default() -> Self {
        Self { icon: "info", size: 18.0, active: false, danger: false, tooltip: None }
    }
}

impl Widget for IconButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let dim = 34.0;
        let (rect, mut resp) = ui.allocate_exact_size(Vec2::splat(dim), Sense::click());
        let p = ui.painter();

        let (bg, stroke, fg) = if self.active {
            let base = if self.danger { theme::RED_SOFT } else { theme::ACCENT_SOFT };
            let col = if self.danger { theme::RED } else { theme::ACCENT };
            (base, Stroke::new(0.5, col), col)
        } else if resp.hovered() {
            (theme::CONTROL.gamma_multiply(0.8), Stroke::new(0.5, theme::HAIR), theme::TEXT1)
        } else {
            (egui::Color32::TRANSPARENT, Stroke::NONE, theme::TEXT1)
        };

        let inner = rect.shrink(1.0);
        if bg != egui::Color32::TRANSPARENT {
            p.rect_filled(inner, egui::Rounding::same(9.0), bg);
        }
        if stroke != Stroke::NONE {
            p.rect_stroke(inner, egui::Rounding::same(9.0), stroke);
        }

        // image glyph centered (not selectable text)
        let _ = ui.put(
            egui::Rect::from_center_size(rect.center(), Vec2::splat(self.size)),
            icons::image(self.icon, self.size, fg),
        );

        resp = resp.on_hover_cursor(CursorIcon::PointingHand);
        if let Some(t) = self.tooltip {
            resp = resp.on_hover_text(t);
        }
        resp
    }
}
