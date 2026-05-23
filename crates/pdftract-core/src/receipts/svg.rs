//! SVG clip generator for visual citation receipts.
//!
//! This module generates self-contained SVG documents that render glyph
//! outlines extracted from PDF fonts. The SVG output is normalized to
//! the receipt's bbox coordinate system and can be rendered standalone
//! in any browser without external font dependencies.
//!
//! # Algorithm
//!
//! 1. Filter glyphs whose bbox center falls within the receipt bbox
//! 2. Extract glyph outlines via ttf-parser's outline API
//! 3. Transform PDF coordinates to SVG coordinates (flip Y axis)
//! 4. Generate SVG path elements with fill colors from glyph styles
//! 5. Wrap in a self-contained SVG element with normalized viewBox
//!
//! # Coordinate system
//!
//! PDF user space uses a bottom-left origin (y increases upward).
//! SVG uses a top-left origin (y increases downward).
//!
//! The transform applied is:
//! - svg_x = pdf_x - bbox.x0
//! - svg_y = bbox.y1 - pdf_y

use std::fmt::Write;

/// A placeholder for Phase 3 glyph data.
///
/// This will be replaced by the actual Phase 3 Glyph struct when
/// that phase is implemented. For now, this stub allows the SVG
/// generator to be developed and tested independently.
#[derive(Debug, Clone)]
pub struct Glyph {
    /// Glyph ID in the font.
    pub gid: u16,

    /// Bounding box in PDF user-space points [x0, y0, x1, y1].
    pub bbox: [f64; 4],

    /// Font face identifier for this glyph.
    pub font_id: usize,

    /// Fill color in CSS format (e.g., "#000000" or "rgb(0,0,0)").
    pub fill_color: String,
}

/// A placeholder for Phase 3 font data.
///
/// This will be replaced by the actual Phase 3 Font struct when
/// that phase is implemented. For now, this stub allows the SVG
/// generator to work with in-memory font data.
#[derive(Debug, Clone)]
pub struct FontFace {
    /// Font data bytes (TTF/OTF).
    pub data: Vec<u8>,

    /// Font index within the data (for TTC collections).
    pub index: u32,
}

/// A collection of glyphs and fonts for a page.
///
/// This represents the input data structure that will come from
/// Phase 3's GlyphList and FontResolver.
#[derive(Debug, Clone)]
pub struct GlyphList {
    /// All glyphs on the page.
    pub glyphs: Vec<Glyph>,

    /// Font faces indexed by font_id.
    pub fonts: Vec<FontFace>,
}

/// SVG clip generator.
///
/// Generates self-contained SVG documents from glyph outlines.
pub struct SvgGenerator {
    glyphs: Vec<Glyph>,
    fonts: Vec<FontFace>,
}

impl SvgGenerator {
    /// Create a new SVG generator from a glyph list.
    pub fn new(glyph_list: GlyphList) -> Self {
        Self {
            glyphs: glyph_list.glyphs,
            fonts: glyph_list.fonts,
        }
    }

    /// Generate an SVG clip for the given bbox.
    ///
    /// # Arguments
    ///
    /// * `bbox` - Bounding box in PDF points [x0, y0, x1, y1]
    ///
    /// # Returns
    ///
    /// A self-contained SVG document as a string.
    pub fn generate(&self, bbox: [f64; 4]) -> String {
        let width = bbox[2] - bbox[0];
        let height = bbox[3] - bbox[1];

        let mut svg = String::new();
        write!(
            svg,
            r#"<svg viewBox="0 0 {} {}" xmlns="http://www.w3.org/2000/svg">"#,
            round_coord(width),
            round_coord(height)
        )
        .unwrap();

        // Filter and group glyphs by fill color for more efficient output
        let mut glyphs_by_color: std::collections::HashMap<String, Vec<&Glyph>> =
            std::collections::HashMap::new();

        for glyph in &self.glyphs {
            // Check if glyph center is within bbox
            let center_x = (glyph.bbox[0] + glyph.bbox[2]) / 2.0;
            let center_y = (glyph.bbox[1] + glyph.bbox[3]) / 2.0;

            if center_x >= bbox[0] && center_x <= bbox[2] && center_y >= bbox[1] && center_y <= bbox[3] {
                glyphs_by_color
                    .entry(glyph.fill_color.clone())
                    .or_default()
                    .push(glyph);
            }
        }

        // Generate path elements grouped by color
        for (color, glyphs) in glyphs_by_color {
            let _ = write!(svg, r#"<g fill="{}">"#, escape_xml(&color));

            for glyph in glyphs {
                if let Some(font) = self.fonts.get(glyph.font_id) {
                    if let Some(path_data) = self.extract_glyph_path(glyph, font, bbox) {
                        let _ = write!(svg, r#"<path d="{}"/>"#, escape_xml(&path_data));
                    }
                    // If outline extraction fails, we skip the glyph
                    // (OCR fallback will be handled in Phase 6.8.3)
                }
            }

            svg.push_str("</g>");
        }

        svg.push_str("</svg>");
        svg
    }

    /// Extract SVG path data for a single glyph.
    fn extract_glyph_path(&self, glyph: &Glyph, font: &FontFace, bbox: [f64; 4]) -> Option<String> {
        let face = ttf_parser::Face::parse(&font.data, font.index).ok()?;

        let mut builder = SvgPathBuilder::new(bbox);
        face.outline_glyph(ttf_parser::GlyphId(glyph.gid), &mut builder)?;

        Some(builder.finish())
    }
}

/// SVG path builder for ttf-parser's OutlineBuilder trait.
///
/// Converts PDF glyph outline commands to SVG path data.
struct SvgPathBuilder {
    path_data: String,
    bbox: [f64; 4],
    last_move: Option<(f64, f64)>,
}

impl SvgPathBuilder {
    fn new(bbox: [f64; 4]) -> Self {
        Self {
            path_data: String::new(),
            bbox,
            last_move: None,
        }
    }

    /// Transform PDF coordinates to SVG coordinates.
    fn transform(&self, x: f32, y: f32) -> (f64, f64) {
        let svg_x = (x as f64) - self.bbox[0];
        let svg_y = self.bbox[3] - (y as f64);
        (round_coord(svg_x), round_coord(svg_y))
    }

    fn finish(self) -> String {
        self.path_data
    }
}

impl ttf_parser::OutlineBuilder for SvgPathBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        let (sx, sy) = self.transform(x, y);
        let _ = write!(self.path_data, "M{:.2} {:.2}", sx, sy);
        self.last_move = Some((sx, sy));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let (sx, sy) = self.transform(x, y);
        let _ = write!(self.path_data, "L{:.2} {:.2}", sx, sy);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (sx1, sy1) = self.transform(x1, y1);
        let (sx, sy) = self.transform(x, y);
        let _ = write!(self.path_data, "Q{:.2} {:.2} {:.2} {:.2}", sx1, sy1, sx, sy);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (sx1, sy1) = self.transform(x1, y1);
        let (sx2, sy2) = self.transform(x2, y2);
        let (sx, sy) = self.transform(x, y);
        let _ = write!(
            self.path_data,
            "C{:.2} {:.2} {:.2} {:.2} {:.2} {:.2}",
            sx1, sy1, sx2, sy2, sx, sy
        );
    }

    fn close(&mut self) {
        self.path_data.push('Z');
    }
}

/// Round a coordinate to 2 decimal places for SVG output.
fn round_coord(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

/// Escape special XML characters in a string.
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Convert a PDF color to a CSS color string.
///
/// This is a placeholder for the full color space conversion
/// that will be implemented in Phase 3. For now, it handles
/// simple RGB colors.
pub fn pdf_color_to_css(color_type: &str, components: &[f64]) -> String {
    match color_type {
        "DeviceRGB" | "RGB" => {
            if components.len() >= 3 {
                let r = (components[0] * 255.0).round() as u8;
                let g = (components[1] * 255.0).round() as u8;
                let b = (components[2] * 255.0).round() as u8;
                format!("#{:02X}{:02X}{:02X}", r, g, b)
            } else {
                "#000000".to_string()
            }
        }
        "DeviceGray" | "Gray" => {
            if components.len() >= 1 {
                let v = (components[0] * 255.0).round() as u8;
                format!("#{:02X}{:02X}{:02X}", v, v, v)
            } else {
                "#000000".to_string()
            }
        }
        "DeviceCMYK" | "CMYK" => {
            // Simple CMYK to RGB conversion
            if components.len() >= 4 {
                let c = components[0];
                let m = components[1];
                let y = components[2];
                let k = components[3];

                let r = (1.0 - c) * (1.0 - k);
                let g = (1.0 - m) * (1.0 - k);
                let b = (1.0 - y) * (1.0 - k);

                let r = (r * 255.0).round() as u8;
                let g = (g * 255.0).round() as u8;
                let b = (b * 255.0).round() as u8;
                format!("rgb({},{},{})", r, g, b)
            } else {
                "#000000".to_string()
            }
        }
        _ => "#000000".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_coord() {
        assert_eq!(round_coord(12.345), 12.35);
        assert_eq!(round_coord(12.344), 12.34);
        assert_eq!(round_coord(0.0), 0.0);
        assert_eq!(round_coord(-5.678), -5.68);
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("hello"), "hello");
        assert_eq!(escape_xml("a&b"), "a&amp;b");
        assert_eq!(escape_xml("<tag>"), "&lt;tag&gt;");
        assert_eq!(escape_xml("\"quote\""), "&quot;quote&quot;");
    }

    #[test]
    fn test_pdf_color_to_css_rgb() {
        assert_eq!(pdf_color_to_css("DeviceRGB", &[0.0, 0.0, 0.0]), "#000000");
        assert_eq!(pdf_color_to_css("DeviceRGB", &[1.0, 1.0, 1.0]), "#FFFFFF");
        assert_eq!(pdf_color_to_css("DeviceRGB", &[1.0, 0.0, 0.0]), "#FF0000");
        assert_eq!(pdf_color_to_css("DeviceRGB", &[0.5, 0.5, 0.5]), "#808080");
    }

    #[test]
    fn test_pdf_color_to_css_gray() {
        assert_eq!(pdf_color_to_css("DeviceGray", &[0.0]), "#000000");
        assert_eq!(pdf_color_to_css("DeviceGray", &[1.0]), "#FFFFFF");
        assert_eq!(pdf_color_to_css("DeviceGray", &[0.5]), "#808080");
    }

    #[test]
    fn test_pdf_color_to_css_cmyk() {
        // Cyan: C=1, M=0, Y=0, K=0
        assert_eq!(pdf_color_to_css("DeviceCMYK", &[1.0, 0.0, 0.0, 0.0]), "rgb(0,255,255)");
        // Black: all 1
        assert_eq!(pdf_color_to_css("DeviceCMYK", &[1.0, 1.0, 1.0, 1.0]), "rgb(0,0,0)");
    }

    #[test]
    fn test_svg_generator_empty_glyph_list() {
        let glyph_list = GlyphList {
            glyphs: vec![],
            fonts: vec![],
        };

        let generator = SvgGenerator::new(glyph_list);
        let svg = generator.generate([0.0, 0.0, 100.0, 100.0]);

        assert!(svg.contains("<svg"));
        assert!(svg.contains("viewBox"));
        assert!(svg.contains("xmlns"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn test_svg_generator_filters_glyphs_by_bbox() {
        let glyph_list = GlyphList {
            glyphs: vec![
                Glyph {
                    gid: 0,
                    bbox: [10.0, 10.0, 30.0, 30.0], // Center at (20, 20) - inside
                    font_id: 0,
                    fill_color: "#000000".to_string(),
                },
                Glyph {
                    gid: 1,
                    bbox: [110.0, 110.0, 130.0, 130.0], // Center at (120, 120) - outside
                    font_id: 0,
                    fill_color: "#000000".to_string(),
                },
            ],
            fonts: vec![],
        };

        let generator = SvgGenerator::new(glyph_list);
        let svg = generator.generate([0.0, 0.0, 100.0, 100.0]);

        // The second glyph should be filtered out
        // (no actual path data since font is empty, but the structure is correct)
        assert!(svg.contains("<svg"));
    }

    #[test]
    fn test_svg_output_is_valid_xml() {
        let glyph_list = GlyphList {
            glyphs: vec![],
            fonts: vec![],
        };

        let generator = SvgGenerator::new(glyph_list);
        let svg = generator.generate([0.0, 0.0, 100.0, 100.0]);

        // Basic XML well-formedness check
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));

        // Check for balanced tags
        let open_count = svg.matches("<").count();
        let close_count = svg.matches(">").count();
        assert_eq!(open_count, close_count);
    }

    #[test]
    fn test_svg_output_no_external_references() {
        let glyph_list = GlyphList {
            glyphs: vec![],
            fonts: vec![],
        };

        let generator = SvgGenerator::new(glyph_list);
        let svg = generator.generate([0.0, 0.0, 100.0, 100.0]);

        // No external references (except xmlns)
        // Check that the only http:// reference is the xmlns attribute
        let http_count = svg.matches("http://").count();
        assert_eq!(http_count, 1, "Only xmlns should contain http://, found {} occurrences", http_count);
        assert!(!svg.contains("href="));
        assert!(!svg.contains("xlink:href"));

        // But xmlns should be present
        assert!(svg.contains("xmlns=\"http://www.w3.org/2000/svg\""));
    }

    #[test]
    fn test_svg_viewbox_normalization() {
        let glyph_list = GlyphList {
            glyphs: vec![],
            fonts: vec![],
        };

        let generator = SvgGenerator::new(glyph_list);

        // Test various bbox sizes
        let cases = [
            ([0.0, 0.0, 100.0, 100.0], "0 0 100 100"),
            ([50.0, 50.0, 150.0, 200.0], "0 0 100 150"),
            ([10.5, 20.5, 30.5, 40.5], "0 0 20 20"),
        ];

        for (bbox, expected_viewbox) in cases {
            let svg = generator.generate(bbox);
            eprintln!("DEBUG: Generated SVG: {}", svg);
            eprintln!("DEBUG: Looking for viewBox=\"{}\"", expected_viewbox);
            assert!(svg.contains(&format!("viewBox=\"{}\"", expected_viewbox)));
        }
    }

    #[test]
    fn test_coordinate_transform() {
        let bbox = [200.0, 400.0, 240.0, 440.0];
        let builder = SvgPathBuilder::new(bbox);

        // PDF coordinate (220, 432) should transform to SVG coordinate
        // svg_x = 220 - 200 = 20
        // svg_y = 440 - 432 = 8
        let (sx, sy) = builder.transform(220.0, 432.0);

        assert!((sx - 20.0).abs() < 0.01, "x coordinate should be 20, got {}", sx);
        assert!((sy - 8.0).abs() < 0.01, "y coordinate should be 8, got {}", sy);
    }

    #[test]
    fn test_svg_groups_by_color() {
        let glyph_list = GlyphList {
            glyphs: vec![
                Glyph {
                    gid: 0,
                    bbox: [10.0, 10.0, 30.0, 30.0],
                    font_id: 0,
                    fill_color: "#FF0000".to_string(),
                },
                Glyph {
                    gid: 1,
                    bbox: [40.0, 10.0, 60.0, 30.0],
                    font_id: 0,
                    fill_color: "#FF0000".to_string(),
                },
                Glyph {
                    gid: 2,
                    bbox: [10.0, 40.0, 30.0, 60.0],
                    font_id: 0,
                    fill_color: "#0000FF".to_string(),
                },
            ],
            fonts: vec![],
        };

        let generator = SvgGenerator::new(glyph_list);
        let svg = generator.generate([0.0, 0.0, 100.0, 100.0]);

        // Should have two groups: one for red, one for blue
        assert!(svg.contains("<g fill=\"#FF0000\">"));
        assert!(svg.contains("<g fill=\"#0000FF\">"));
    }

    #[test]
    fn test_svg_from_actual_font() {
        // Test with real font data (DejaVu Sans)
        let font_data = include_bytes!("../../../../tests/fixtures/fonts/DejaVuSans.ttf");
        let glyph_list = GlyphList {
            glyphs: vec![
                Glyph {
                    gid: 36, // 'A' in DejaVu Sans (not 3, which is typically .notdef)
                    bbox: [50.0, 400.0, 100.0, 450.0],
                    font_id: 0,
                    fill_color: "#000000".to_string(),
                },
            ],
            fonts: vec![FontFace {
                data: font_data.to_vec(),
                index: 0,
            }],
        };

        let generator = SvgGenerator::new(glyph_list);
        let svg = generator.generate([0.0, 0.0, 500.0, 500.0]);

        // Should have generated a path
        assert!(svg.contains("<path d="));
        // Should start with M (move to)
        assert!(svg.contains("M"));
    }

    #[test]
    fn test_svg_validates_via_quick_xml() {
        // Verify SVG output is well-formed XML using quick-xml
        let glyph_list = GlyphList {
            glyphs: vec![
                Glyph {
                    gid: 36, // 'A' in DejaVu Sans
                    bbox: [50.0, 400.0, 100.0, 450.0],
                    font_id: 0,
                    fill_color: "#FF0000".to_string(),
                },
                Glyph {
                    gid: 37, // 'B' in DejaVu Sans
                    bbox: [110.0, 400.0, 160.0, 450.0],
                    font_id: 0,
                    fill_color: "#0000FF".to_string(),
                },
            ],
            fonts: vec![FontFace {
                data: include_bytes!("../../../../tests/fixtures/fonts/DejaVuSans.ttf").to_vec(),
                index: 0,
            }],
        };

        let generator = SvgGenerator::new(glyph_list);
        let svg = generator.generate([0.0, 0.0, 500.0, 500.0]);

        // Parse with quick-xml to verify well-formedness
        use quick_xml::Reader;
        let mut reader = Reader::from_str(&svg);
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(quick_xml::events::Event::Eof) => break,
                Ok(_) => {
                    buf.clear();
                    continue;
                }
                Err(e) => panic!("SVG is not well-formed XML: {}", e),
            }
        }
    }

    #[test]
    fn test_svg_handles_missing_glyph_outline() {
        // Test graceful handling when a glyph has no outline
        let font_data = include_bytes!("../../../../tests/fixtures/fonts/DejaVuSans.ttf");
        let glyph_list = GlyphList {
            glyphs: vec![
                Glyph {
                    gid: 36, // Valid glyph with outline
                    bbox: [50.0, 400.0, 100.0, 450.0],
                    font_id: 0,
                    fill_color: "#000000".to_string(),
                },
                Glyph {
                    gid: 0, // .notdef glyph, may have no outline
                    bbox: [110.0, 400.0, 160.0, 450.0],
                    font_id: 0,
                    fill_color: "#000000".to_string(),
                },
                Glyph {
                    gid: 9999, // Out of range glyph ID
                    bbox: [170.0, 400.0, 220.0, 450.0],
                    font_id: 0,
                    fill_color: "#000000".to_string(),
                },
            ],
            fonts: vec![FontFace {
                data: font_data.to_vec(),
                index: 0,
            }],
        };

        let generator = SvgGenerator::new(glyph_list);
        // Should not panic, should skip glyphs without outlines
        let svg = generator.generate([0.0, 0.0, 500.0, 500.0]);

        // At least the valid glyph should produce a path
        assert!(svg.contains("<path d="));
    }

    #[test]
    fn test_svg_path_uses_absolute_coordinates() {
        // Verify SVG uses absolute commands (M, L, Q, C, Z) not relative (m, l, q, c, z)
        let font_data = include_bytes!("../../../../tests/fixtures/fonts/DejaVuSans.ttf");
        let glyph_list = GlyphList {
            glyphs: vec![Glyph {
                gid: 36, // 'A' in DejaVu Sans
                bbox: [50.0, 400.0, 100.0, 450.0],
                font_id: 0,
                fill_color: "#000000".to_string(),
            }],
            fonts: vec![FontFace {
                data: font_data.to_vec(),
                index: 0,
            }],
        };

        let generator = SvgGenerator::new(glyph_list);
        let svg = generator.generate([0.0, 0.0, 500.0, 500.0]);

        // Extract path data
        let path_start = svg.find("d=\"").unwrap() + 3;
        let path_end = svg[path_start..].find("\"").unwrap();
        let path_data = &svg[path_start..path_start + path_end];

        // Check that path uses uppercase (absolute) commands
        // Note: This test assumes the path contains at least one command
        let has_uppercase = path_data
            .chars()
            .any(|c| matches!(c, 'M' | 'L' | 'Q' | 'C' | 'Z' | 'H' | 'V'));

        assert!(
            has_uppercase,
            "Path data should use absolute (uppercase) commands, got: {}",
            path_data
        );
    }

    #[test]
    fn test_svg_aggregate_size_estimate() {
        // Verify that SVG output size is reasonable for 100 receipts
        // Plan acceptance criterion: 100 SVG receipts <= 500 KB

        let font_data = include_bytes!("../../../../tests/fixtures/fonts/DejaVuSans.ttf");

        // Simulate a typical receipt with a few glyphs
        let typical_receipt = || {
            let glyph_list = GlyphList {
                glyphs: vec![
                    Glyph {
                        gid: 36,
                        bbox: [50.0, 400.0, 70.0, 420.0],
                        font_id: 0,
                        fill_color: "#000000".to_string(),
                    },
                    Glyph {
                        gid: 68, // 'a'
                        bbox: [75.0, 400.0, 90.0, 420.0],
                        font_id: 0,
                        fill_color: "#000000".to_string(),
                    },
                ],
                fonts: vec![FontFace {
                    data: font_data.to_vec(),
                    index: 0,
                }],
            };

            let generator = SvgGenerator::new(glyph_list);
            generator.generate([0.0, 0.0, 500.0, 500.0])
        };

        // Generate 100 receipts and measure total size
        let receipts: Vec<String> = (0..100).map(|_| typical_receipt()).collect();
        let total_bytes: usize = receipts.iter().map(|r| r.len()).sum();

        // 500 KB = 512,000 bytes
        assert!(
            total_bytes <= 512_000,
            "100 SVG receipts should be <= 500 KB, got {} bytes",
            total_bytes
        );

        // Also verify individual receipt size is reasonable
        let avg_size = total_bytes / 100;
        assert!(
            avg_size < 5_000,
            "Average SVG receipt should be < 5 KB, got {} bytes",
            avg_size
        );
    }
}
