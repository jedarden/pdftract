//! OCR image XObjects for the opt-in CLI `--ocr` path.
//!
//! Poppler preserves the original image pixels here, so mixed pages do not
//! OCR their vector layer a second time. Tesseract's plain-text output is
//! retained as one OCR span and block per image.

use anyhow::{bail, Context, Result};
use pdftract_core::extract::ExtractionResult;
use pdftract_core::schema::{BlockJson, SpanJson};
use std::path::Path;
use std::process::{Command, Output};

fn checked_output(command: &mut Command, tool: &str) -> Result<Output> {
    let output = command
        .output()
        .with_context(|| format!("OCR requires {tool} on PATH"))?;
    if !output.status.success() {
        bail!(
            "{tool} failed during OCR (exit {}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(output)
}

pub fn augment_with_ocr(
    input: &Path,
    result: &mut ExtractionResult,
    languages: &[String],
) -> Result<()> {
    let listing = checked_output(
        Command::new("pdfimages").arg("-list").arg(input),
        "pdfimages",
    )?;
    let image_pages: Vec<usize> = String::from_utf8(listing.stdout)
        .context("pdfimages -list returned non-UTF-8 output")?
        .lines()
        .filter_map(|line| line.split_whitespace().next()?.parse::<usize>().ok())
        .collect();
    if image_pages.is_empty() {
        return Ok(());
    }

    let work = tempfile::Builder::new().prefix("pdftract-ocr-").tempdir()?;
    let prefix = work.path().join("image");
    checked_output(
        Command::new("pdfimages")
            .arg("-png")
            .arg(input)
            .arg(&prefix),
        "pdfimages",
    )?;

    let mut images: Vec<_> = std::fs::read_dir(work.path())?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<_>>()?;
    images.sort();
    images.retain(|path| path.extension().is_some_and(|ext| ext == "png"));
    if images.len() != image_pages.len() {
        bail!(
            "pdfimages listed {} images but extracted {} PNGs; cannot map OCR text to pages",
            image_pages.len(),
            images.len()
        );
    }

    let language = languages.join("+");
    for (image, page_number) in images.iter().zip(image_pages) {
        let page_index = page_number
            .checked_sub(1)
            .context("pdfimages reported invalid page zero")?;
        let page = result.pages.get_mut(page_index).with_context(|| {
            format!("pdfimages reported page {page_number} outside extracted page range")
        })?;
        let output = checked_output(
            Command::new("tesseract")
                .arg(image)
                .arg("stdout")
                .arg("-l")
                .arg(&language),
            "tesseract",
        )?;
        let text = String::from_utf8(output.stdout).context("Tesseract returned non-UTF-8 text")?;
        let text = text.trim();
        if text.is_empty() {
            continue;
        }

        let has_vector_text = page
            .blocks
            .iter()
            .any(|block| !block.text.trim().is_empty());
        let bbox = [
            0.0,
            0.0,
            f64::from(page.width.unwrap_or(612.0)),
            f64::from(page.height.unwrap_or(792.0)),
        ];
        let span_index = page.spans.len();
        page.spans.push(SpanJson {
            text: text.to_string(),
            bbox,
            font: "OCR".to_string(),
            size: 0.0,
            color: None,
            rendering_mode: None,
            confidence: None,
            confidence_source: Some("ocr".to_string()),
            lang: None,
            flags: Vec::new(),
            receipt: None,
            column: None,
        });
        page.blocks.push(BlockJson {
            kind: "paragraph".to_string(),
            text: text.to_string(),
            bbox,
            level: None,
            table_index: None,
            spans: vec![span_index],
            receipt: None,
        });
        page.page_type = Some(if has_vector_text { "mixed" } else { "scanned" }.to_string());
        result.metadata.span_count += 1;
        result.metadata.block_count += 1;
    }
    Ok(())
}
