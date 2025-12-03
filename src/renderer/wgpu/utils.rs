use wgpu_glyph::ab_glyph::{FontArc, Font, ScaleFont};

pub fn calculate_gutter_width(font: &FontArc, font_scale: &f32, max_line: usize) -> f32 {
    let max_line_str = max_line.to_string();
    let scaled_font = font.as_scaled(*font_scale);
    let mut width = 0.0;
    for ch in max_line_str.chars() {
        width += scaled_font.h_advance(scaled_font.glyph_id(ch));
    }
    width + 20.0
}

pub fn status_bar_height() -> f32 {
    let padding = 8.0;
    return 30.0 + 26.0 + (padding * 2.0)
}

pub fn hex_to_wgpu_color(hex: &str) -> wgpu::Color {
    let (r8, g8, b8) = parse_hex(hex);

    let r = srgb_to_linear(r8 as f32 / 255.0);
    let g = srgb_to_linear(g8 as f32 / 255.0);
    let b = srgb_to_linear(b8 as f32 / 255.0);

    wgpu::Color {
        r: r as f64,
        g: g as f64,
        b: b as f64,
        a: 1.0,
    }
}

fn parse_hex(hex: &str) -> (u8, u8, u8) {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap();
    (r, g, b)
}

pub fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}
