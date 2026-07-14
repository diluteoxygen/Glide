use std::f32::consts::PI;

/// A Second Order System (Muratori style) modeling a critically damped spring.
/// Parameters:
/// - `f`: Natural frequency in Hz (speed of the response).
/// - `z`: Damping coefficient (1.0 = critically damped, <1.0 = underdamped/bouncy).
/// - `r`: Initial response (0.0 = gradual, <0.0 = anticipatory, >0.0 = overshoot).
#[derive(Debug, Clone)]
pub struct SecondOrderSystem {
    y: f32,
    yd: f32,
    k1: f32,
    k2: f32,
    k3: f32,
}

impl SecondOrderSystem {
    pub fn new(f: f32, z: f32, r: f32, initial_y: f32) -> Self {
        let k1 = z / (PI * f);
        let k2 = 1.0 / ((2.0 * PI * f) * (2.0 * PI * f));
        let k3 = r * z / (2.0 * PI * f);
        
        Self {
            y: initial_y,
            yd: 0.0,
            k1,
            k2,
            k3,
        }
    }

    /// Updates the system over time `dt` given a target `x` and target velocity `xd`.
    /// Returns the new position.
    pub fn update(&mut self, dt: f32, x: f32, xd: f32) -> f32 {
        if dt <= 0.0 {
            return self.y;
        }

        // To ensure numerical stability for larger dt, we sub-step if necessary.
        let max_dt = 1.0 / 120.0; // Assume 120Hz is safe enough
        let sub_steps = (dt / max_dt).ceil() as u32;
        let dt_step = dt / (sub_steps as f32);

        for _ in 0..sub_steps {
            self.y += dt_step * self.yd;
            self.yd += dt_step * (x + self.k3 * xd - self.y - self.k1 * self.yd) / self.k2;
        }

        self.y
    }

    /// Gets the current position.
    pub fn y(&self) -> f32 {
        self.y
    }
}
