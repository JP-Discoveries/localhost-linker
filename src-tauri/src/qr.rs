//! QR code rendering as an inline SVG string the popup can drop straight into the DOM.

use qrcode::render::svg;
use qrcode::{EcLevel, QrCode};

pub fn svg_for(url: &str) -> String {
    match QrCode::with_error_correction_level(url.as_bytes(), EcLevel::M) {
        Ok(code) => code
            .render::<svg::Color>()
            .min_dimensions(220, 220)
            .quiet_zone(true)
            .dark_color(svg::Color("#000000"))
            .light_color(svg::Color("#ffffff"))
            .build(),
        Err(_) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn renders_svg() {
        let out = super::svg_for("http://192.168.1.5:3000");
        assert!(out.contains("<svg"), "got: {}", &out[..out.len().min(80)]);
    }
}
