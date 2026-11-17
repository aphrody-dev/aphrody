//! Standalone GPUI WebGPU Studio template helpers.

pub fn adaptive_column_count(width_px: f32) -> usize {
    if width_px < 600.0 {
        1
    } else if width_px < 1024.0 {
        2
    } else {
        3
    }
}
