//! Color representation for PDF graphics state.
//!
//! Supports all PDF color spaces relevant to text extraction.

use std::sync::Arc;

/// Color in a PDF graphics state.
///
/// Covers all PDF color spaces relevant to text extraction.
/// Unsupported color spaces (CalRGB, ICCBased, Pattern) are treated as transparent.
#[derive(Debug, Clone, PartialEq)]
pub enum Color {
    /// DeviceGray color space (0.0–1.0)
    DeviceGray(f32),
    /// DeviceRGB color space (0.0–1.0 each)
    DeviceRGB([f32; 3]),
    /// DeviceCMYK color space (0.0–1.0 each)
    DeviceCMYK([f32; 4]),
    /// Spot color: colorant name and tint (0.0–1.0)
    Spot(Arc<str>, f32),
    /// Unsupported color space (CalRGB, ICCBased, Pattern)
    /// Treated as transparent for text extraction
    Other,
}

impl Color {
    /// Create a new DeviceGray color.
    pub fn gray(value: f32) -> Self {
        Color::DeviceGray(value.clamp(0.0, 1.0))
    }

    /// Create a new DeviceRGB color.
    pub fn rgb(r: f32, g: f32, b: f32) -> Self {
        Color::DeviceRGB([
            r.clamp(0.0, 1.0),
            g.clamp(0.0, 1.0),
            b.clamp(0.0, 1.0),
        ])
    }

    /// Create a new DeviceCMYK color.
    pub fn cmyk(c: f32, m: f32, y: f32, k: f32) -> Self {
        Color::DeviceCMYK([
            c.clamp(0.0, 1.0),
            m.clamp(0.0, 1.0),
            y.clamp(0.0, 1.0),
            k.clamp(0.0, 1.0),
        ])
    }

    /// Create a new Spot color.
    pub fn spot(name: Arc<str>, tint: f32) -> Self {
        Color::Spot(name, tint.clamp(0.0, 1.0))
    }

    /// Convert to CSS hex string if possible.
    ///
    /// Returns `None` for CMYK, Spot, and Other colors.
    pub fn to_css_hex(&self) -> Option<String> {
        match self {
            Color::DeviceGray(v) => {
                let byte = (v * 255.0).round() as u8;
                Some(format!("#{byte:02x}{byte:02x}{byte:02x}"))
            }
            Color::DeviceRGB([r, g, b]) => {
                let rr = (r * 255.0).round() as u8;
                let gg = (g * 255.0).round() as u8;
                let bb = (b * 255.0).round() as u8;
                Some(format!("#{rr:02x}{gg:02x}{bb:02x}"))
            }
            Color::DeviceCMYK([c, m, y, k]) => {
                // Convert CMYK to RGB using standard formula
                // R = 255 × (1−C) × (1−K)
                // G = 255 × (1−M) × (1−K)
                // B = 255 × (1−Y) × (1−K)
                let r = (1.0 - c) * (1.0 - k);
                let g = (1.0 - m) * (1.0 - k);
                let b = (1.0 - y) * (1.0 - k);
                let rr = (r * 255.0).round() as u8;
                let gg = (g * 255.0).round() as u8;
                let bb = (b * 255.0).round() as u8;
                Some(format!("#{rr:02x}{gg:02x}{bb:02x}"))
            }
            Color::Spot(_, _) | Color::Other => None,
        }
    }

    /// Check if this color should be treated as transparent.
    pub fn is_transparent(&self) -> bool {
        matches!(self, Color::Spot(_, _) | Color::Other)
    }
}

impl Default for Color {
    fn default() -> Self {
        // Default to black
        Color::DeviceRGB([0.0, 0.0, 0.0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gray_clamping() {
        assert_eq!(Color::gray(-0.5), Color::DeviceGray(0.0));
        assert_eq!(Color::gray(1.5), Color::DeviceGray(1.0));
    }

    #[test]
    fn test_rgb_clamping() {
        assert_eq!(Color::rgb(-0.5, 0.5, 1.5), Color::DeviceRGB([0.0, 0.5, 1.0]));
    }

    #[test]
    fn test_to_css_hex_gray() {
        assert_eq!(Color::gray(0.5).to_css_hex(), Some("#808080".to_string()));
        assert_eq!(Color::gray(0.0).to_css_hex(), Some("#000000".to_string()));
        assert_eq!(Color::gray(1.0).to_css_hex(), Some("#ffffff".to_string()));
    }

    #[test]
    fn test_to_css_hex_rgb() {
        assert_eq!(Color::rgb(1.0, 0.0, 0.0).to_css_hex(), Some("#ff0000".to_string()));
        assert_eq!(Color::rgb(0.0, 1.0, 0.0).to_css_hex(), Some("#00ff00".to_string()));
        assert_eq!(Color::rgb(0.0, 0.0, 1.0).to_css_hex(), Some("#0000ff".to_string()));
    }

    #[test]
    fn test_to_css_hex_cmyk() {
        // Pure cyan in CMYK should convert to cyan in RGB
        assert_eq!(Color::cmyk(1.0, 0.0, 0.0, 0.0).to_css_hex(), Some("#00ffff".to_string()));
    }

    #[test]
    fn test_spot_is_transparent() {
        let spot = Color::spot(Arc::from("PANTONE-123"), 0.5);
        assert!(spot.is_transparent());
        assert!(spot.to_css_hex().is_none());
    }

    #[test]
    fn test_other_is_transparent() {
        assert!(Color::Other.is_transparent());
        assert!(Color::Other.to_css_hex().is_none());
    }
}
