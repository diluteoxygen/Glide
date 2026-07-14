use crate::event_log::{LogEntry, MouseEvent};
use super::spring::SecondOrderSystem;

pub struct VirtualCamera {
    x_spring: SecondOrderSystem,
    y_spring: SecondOrderSystem,
    zoom_spring: SecondOrderSystem,
    
    // Config
    lookahead_us: u64,
    idle_timeout_us: u64,
    active_zoom: f32,
    idle_zoom: f32,
}

impl VirtualCamera {
    pub fn new(initial_x: f32, initial_y: f32) -> Self {
        Self {
            // Position: f=1.5Hz, z=1.0, r=-0.5 (anticipatory)
            x_spring: SecondOrderSystem::new(1.5, 1.0, -0.5, initial_x),
            y_spring: SecondOrderSystem::new(1.5, 1.0, -0.5, initial_y),
            // Zoom: f=0.6Hz, z=1.0, r=0.0 (no anticipation)
            zoom_spring: SecondOrderSystem::new(0.6, 1.0, 0.0, 1.0),
            
            lookahead_us: 300_000,
            idle_timeout_us: 1_500_000,
            active_zoom: 1.5,
            idle_zoom: 1.0,
        }
    }

    /// Determines the target (x, y, zoom) for the camera based on the event log.
    pub fn compute_target(&self, time_us: u64, events: &[LogEntry]) -> (f32, f32, f32) {
        let target_time_us = time_us.saturating_add(self.lookahead_us);
        
        let mut target_x = self.x_spring.y();
        let mut target_y = self.y_spring.y();
        
        // Find the cursor position at `target_time_us`
        for entry in events {
            if entry.t > target_time_us {
                break;
            }
            if let MouseEvent::Move { x, y } = &entry.event {
                target_x = *x as f32;
                target_y = *y as f32;
            }
        }
        
        // Compute zoom target
        let window_start = time_us.saturating_sub(self.idle_timeout_us);
        let mut is_active = false;
        
        for entry in events {
            if entry.t > target_time_us {
                break;
            }
            if entry.t >= window_start {
                is_active = true;
            }
        }
        
        let target_zoom = if is_active { self.active_zoom } else { self.idle_zoom };
        
        (target_x, target_y, target_zoom)
    }

    /// Steps the physics simulation forward by `dt_seconds` towards the targets.
    /// Returns the current (x, y, zoom).
    pub fn step(&mut self, dt_seconds: f32, target_x: f32, target_y: f32, target_zoom: f32) -> (f32, f32, f32) {
        let cur_x = self.x_spring.update(dt_seconds, target_x, 0.0);
        let cur_y = self.y_spring.update(dt_seconds, target_y, 0.0);
        let cur_zoom = self.zoom_spring.update(dt_seconds, target_zoom, 0.0);
        (cur_x, cur_y, cur_zoom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_idle_zoom() {
        let camera = VirtualCamera::new(0.0, 0.0);
        
        let mut events = vec![];
        
        let (_, _, tz) = camera.compute_target(0, &events);
        assert_eq!(tz, 1.0);
        
        events.push(LogEntry {
            t: 1_000_000,
            event: MouseEvent::Click { x: 10, y: 10, button: "left".into(), action: "down".into() }
        });
        
        let (_, _, tz) = camera.compute_target(800_000, &events);
        assert_eq!(tz, 1.5);
        
        let (_, _, tz) = camera.compute_target(3_000_000, &events);
        assert_eq!(tz, 1.0);
    }
}
