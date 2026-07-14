pub const SHADER_WGSL: &str = include_str!("shader.wgsl");

/// Calculates the transformation matrix for the vertex shader given target geometry.
pub fn calculate_transform(target_x: f32, target_y: f32, zoom: f32, width: u32, height: u32) -> [f32; 16] {
    // Convert to normalized coordinates (0 to 1)
    let nx = target_x / width as f32;
    let ny = target_y / height as f32;
    
    // Unscaled geometry pos corresponding to target
    let pos_x = nx * 2.0 - 1.0;
    let pos_y = -(ny * 2.0 - 1.0); // Y is flipped in clip space
    
    let mut tx = -pos_x * zoom;
    let mut ty = -pos_y * zoom;
    
    // Clamp to avoid showing black borders
    let max_t = zoom - 1.0;
    if max_t > 0.0 {
        tx = tx.clamp(-max_t, max_t);
        ty = ty.clamp(-max_t, max_t);
    } else {
        tx = 0.0;
        ty = 0.0;
    }

    [
        zoom, 0.0,  0.0, 0.0,
        0.0,  zoom, 0.0, 0.0,
        0.0,  0.0,  1.0, 0.0,
        tx,   ty,   0.0, 1.0,
    ]
}
