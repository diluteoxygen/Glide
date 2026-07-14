use crate::state_machine::CameraState;
use camera_core::SecondOrderSystem;
use input_hooks::OtfInputEvent;
use crossbeam_channel::Receiver;
use std::time::Instant;

pub struct OtfCameraEngine {
    pub state: CameraState,
    x_spring: SecondOrderSystem,
    y_spring: SecondOrderSystem,
    zoom_spring: SecondOrderSystem,
    
    pub last_cursor_x: f32,
    pub last_cursor_y: f32,
    
    frame_width: f32,
    frame_height: f32,
    
    last_tick: Instant,
}

impl OtfCameraEngine {
    pub fn new(frame_width: f32, frame_height: f32) -> Self {
        let center_x = frame_width / 2.0;
        let center_y = frame_height / 2.0;
        
        Self {
            state: CameraState::Idle,
            // Tuning constants:
            // f (speed): 1.5Hz for pos, 1.2Hz for zoom (slightly faster than Stage 6's 0.6Hz)
            // z (damping): 1.0 for pos, 1.0 for zoom (critically damped)
            // r (anticipation): 0.0 for all, since we don't have lookahead, negative r (anticipatory) would just overshoot randomly
            x_spring: SecondOrderSystem::new(1.5, 1.0, 0.0, center_x),
            y_spring: SecondOrderSystem::new(1.5, 1.0, 0.0, center_y),
            zoom_spring: SecondOrderSystem::new(1.2, 1.0, 0.0, 1.0),
            
            last_cursor_x: center_x,
            last_cursor_y: center_y,
            
            frame_width,
            frame_height,
            
            last_tick: Instant::now(),
        }
    }

    pub fn tick(&mut self, event_rx: &Receiver<OtfInputEvent>) -> (f32, f32, f32) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f32();
        self.last_tick = now;

        // Drain all pending events
        let prev_state = self.state;
        while let Ok(event) = event_rx.try_recv() {
            if let OtfInputEvent::CursorMoved(x, y) = event {
                self.last_cursor_x = x as f32;
                self.last_cursor_y = y as f32;
            }
            self.state.apply_event(event);
        }

        if self.state != prev_state {
            match self.state {
                CameraState::Idle => tracing::info!("OTF State Changed: Idle (Zoom 1.0x)"),
                CameraState::Zoomed { level } => tracing::info!("OTF State Changed: Zoomed to {:.2}x", level),
            }
        }

        // Determine targets based on state
        let (target_x, target_y, target_zoom) = match self.state {
            CameraState::Idle => (self.frame_width / 2.0, self.frame_height / 2.0, 1.0),
            CameraState::Zoomed { level } => (self.last_cursor_x, self.last_cursor_y, level),
        };

        // Update springs
        let cur_x = self.x_spring.update(dt, target_x, 0.0);
        let cur_y = self.y_spring.update(dt, target_y, 0.0);
        let cur_zoom = self.zoom_spring.update(dt, target_zoom, 0.0);

        (cur_x, cur_y, cur_zoom)
    }
}
