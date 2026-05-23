use std::env;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build/std14-metrics.json");

    let out_dir = env::var("OUT_DIR").unwrap();
    let metrics_path = Path::new("build/std14-metrics.json");

    let json_content = fs::read_to_string(metrics_path)
        .expect("Failed to read std14-metrics.json");

    let data: serde_json::Value = serde_json::from_str(&json_content)
        .expect("Failed to parse std14-metrics.json");

    let fonts = data["fonts"].as_object()
        .expect("fonts object missing");

    let mut metrics_structs = String::new();

    for (font_name, font_data) in fonts {
        let font_ident = font_name.replace("-", "_");
        let weights = font_data["weights"].as_array()
            .expect("weights array missing");

        let weights_array: Vec<String> = weights.iter()
            .map(|v| v.as_u64().unwrap_or(0).to_string())
            .collect();

        let font_bbox = font_data["font_bbox"].as_array()
            .expect("font_bbox array missing");
        let font_bbox: Vec<String> = font_bbox.iter()
            .map(|v| v.as_i64().unwrap_or(0).to_string())
            .collect();

        let ascent = font_data["ascent"].as_i64().expect("ascent missing");
        let descent = font_data["descent"].as_i64().expect("descent missing");
        let italic_angle = font_data["italic_angle"].as_f64().expect("italic_angle missing");
        let cap_height = font_data["cap_height"].as_i64().expect("cap_height missing");
        let stem_v = font_data["stem_v"].as_i64().expect("stem_v missing");

        let encoding_str = font_data["encoding"].as_str().expect("encoding missing");
        let encoding = match encoding_str {
            "StandardEncoding" => "NamedEncoding::Standard",
            "SymbolEncoding" => "NamedEncoding::Symbol",
            "ZapfDingbatsEncoding" => "NamedEncoding::ZapfDingbats",
            _ => "NamedEncoding::Standard",
        };

        metrics_structs.push_str(&format!(r#"
static {}_WIDTHS: &[u16; 256] = &[{}];
static {}_METRICS: Std14Metrics = Std14Metrics {{
    widths: &{}_WIDTHS,
    ascent: {},
    descent: {},
    italic_angle: {}f32,
    font_bbox: [{}],
    cap_height: {},
    stem_v: {},
    encoding: {},
}};
"#,
            font_ident.to_uppercase(),
            weights_array.join(", "),
            font_ident.to_uppercase(),
            font_ident.to_uppercase(),
            ascent,
            descent,
            italic_angle,
            font_bbox.join(", "),
            cap_height,
            stem_v,
            encoding
        ));
    }

    // Build the phf map using phf_codegen
    let mut map_builder = phf_codegen::Map::new();

    for font_name in fonts.keys() {
        let ident = font_name.replace("-", "_");
        map_builder.entry(font_name.as_str(), &format!("&{}_METRICS", ident.to_uppercase()));
    }

    let rust_code = format!(r#"
// Auto-generated Standard 14 font metrics.
// Do not edit manually.

{}

pub fn get_std14_metrics(name: &str) -> Option<&'static Std14Metrics> {{
    static METRICS: phf::Map<&'static str, &'static Std14Metrics> = {};
    METRICS.get(name).copied()
}}
"#,
        metrics_structs,
        map_builder.build()
    );

    fs::write(Path::new(&out_dir).join("std14_registry.rs"), rust_code)
        .expect("Failed to write std14_registry.rs");
}
