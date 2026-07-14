use input_hooks::OtfInputEvent;

pub const DEFAULT_ZOOM: f32 = 1.6;
pub const MIN_ZOOM: f32 = 1.0;
pub const MAX_ZOOM: f32 = 4.0;
pub const SCROLL_INCREMENT: f32 = 0.1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CameraState {
    Idle,
    Zoomed { level: f32 },
}

impl CameraState {
    pub fn new() -> Self {
        CameraState::Idle
    }

    pub fn apply_event(&mut self, event: OtfInputEvent) {
        match event {
            OtfInputEvent::DoubleTapShift => {
                match self {
                    CameraState::Idle => {
                        *self = CameraState::Zoomed { level: DEFAULT_ZOOM };
                    }
                    CameraState::Zoomed { .. } => {
                        // Reset to default
                        *self = CameraState::Zoomed { level: DEFAULT_ZOOM };
                    }
                }
            }
            OtfInputEvent::DoubleTapCtrl => {
                *self = CameraState::Idle;
            }
            OtfInputEvent::ShiftScroll(delta) => {
                // delta is typically 120 or -120, let's normalize to signs
                let steps = (delta as f32) / 120.0;
                
                match self {
                    CameraState::Idle => {
                        if steps > 0.0 {
                            let mut new_level = 1.0 + steps * SCROLL_INCREMENT;
                            new_level = new_level.clamp(MIN_ZOOM, MAX_ZOOM);
                            *self = CameraState::Zoomed { level: new_level };
                        }
                    }
                    CameraState::Zoomed { level } => {
                        let mut new_level = *level + steps * SCROLL_INCREMENT;
                        new_level = new_level.clamp(MIN_ZOOM, MAX_ZOOM);
                        
                        // "Scrolling back down through ~1.0x exits to idle"
                        // Since MIN_ZOOM is 1.0, if we hit 1.0 exactly or try to go below, we can exit to Idle.
                        if new_level <= 1.001 { // small epsilon
                            *self = CameraState::Idle;
                        } else {
                            *self = CameraState::Zoomed { level: new_level };
                        }
                    }
                }
            }
            OtfInputEvent::CursorMoved(_, _) => {
                // State machine itself doesn't change on cursor move, just the engine's target.
            }
        }
    }
}
