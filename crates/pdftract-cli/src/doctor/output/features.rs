//! Feature listing output for --features flag

use crate::doctor::DoctorFeatures;

/// Print compiled features, one per line
pub fn output_features(features: &DoctorFeatures) {
    let mut feature_names = Vec::new();

    if features.ocr {
        feature_names.push("ocr");
    }
    if features.full_render {
        feature_names.push("full-render");
    }
    if features.remote {
        feature_names.push("remote");
    }
    if features.profiles {
        feature_names.push("profiles");
    }
    if features.serve {
        feature_names.push("serve");
    }
    if features.mcp {
        feature_names.push("mcp");
    }
    if features.inspect {
        feature_names.push("inspect");
    }
    if features.grep {
        feature_names.push("grep");
    }
    if features.cache {
        feature_names.push("cache");
    }
    if features.receipts {
        feature_names.push("receipts");
    }
    if features.markdown {
        feature_names.push("markdown");
    }

    // Sort for consistent output
    feature_names.sort();

    for feature in feature_names {
        println!("{}", feature);
    }
}
