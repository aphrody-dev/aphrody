//! Deterministic WASI Preview 2 AOT compute kernel for Aphrody.

fn srgb_channel_to_linear(channel: u8) -> f64 {
    let s = f64::from(channel) / 255.0;
    if s <= 0.03928 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

fn relative_luminance(rgb: [u8; 3]) -> f64 {
    0.2126 * srgb_channel_to_linear(rgb[0])
        + 0.7152 * srgb_channel_to_linear(rgb[1])
        + 0.0722 * srgb_channel_to_linear(rgb[2])
}

fn wcag_contrast(fg: [u8; 3], bg: [u8; 3]) -> f64 {
    let l1 = relative_luminance(fg);
    let l2 = relative_luminance(bg);
    let (hi, lo) = if l1 >= l2 { (l1, l2) } else { (l2, l1) };
    (hi + 0.05) / (lo + 0.05)
}

fn spring_step(stiffness: f64, damping_ratio: f64, t_secs: f64) -> f64 {
    if t_secs <= 0.0 {
        return 0.0;
    }
    let omega0 = stiffness.sqrt();
    let decay = (-damping_ratio * omega0 * t_secs).exp();
    1.0 - decay
}

fn main() {
    let surface = [0x0E, 0x14, 0x1B];
    let on_surface = [0xE0, 0xE6, 0xF0];
    let primary = [0x8F, 0xD0, 0xFF];
    let on_primary = [0x00, 0x33, 0x52];

    let surface_contrast = wcag_contrast(on_surface, surface);
    let primary_contrast = wcag_contrast(on_primary, primary);
    let spring_120hz = spring_step(1400.0, 0.9, 1.0 / 120.0);

    println!(
        "{{\"template\":\"wasi-p2-aot-compute\",\"design_source\":\"https://design.google/\",\"surface_contrast\":{surface_contrast:.2},\"primary_contrast\":{primary_contrast:.2},\"spring_120hz\":{spring_120hz:.6}}}"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m3_expressive_tokens_meet_wcag_aa() {
        assert!(wcag_contrast([0xE0, 0xE6, 0xF0], [0x0E, 0x14, 0x1B]) >= 4.5);
        assert!(wcag_contrast([0x00, 0x33, 0x52], [0x8F, 0xD0, 0xFF]) >= 4.5);
    }
}
