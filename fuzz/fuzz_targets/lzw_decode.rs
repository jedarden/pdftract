//! Fuzz target for the LZWDecode filter (PDF 1.7, section 7.4.4).
//!
//! This is the compensating control ADR-003 (RUSTSEC-2020-0144 advisory
//! exception for the unmaintained `lzw` crate) relies on. The generic
//! `stream_decoder` target only reaches LZWDecode with default parameters
//! (predictor 1, /EarlyChange 1); this target additionally explores the
//! `/DecodeParms` space around the filter:
//!
//! - `/EarlyChange` 0 (GIF-style late change) and 1 (Adobe/TIFF early change),
//!   i.e. both `lzw::Decoder` and `lzw::DecoderEarlyChange` code paths
//! - `/Predictor` 1 (none), 2 (TIFF horizontal differencing) and 10-15 (PNG)
//!   via `apply_predictor` on the decompressed bytes
//! - `/Columns`, `/Colors`, `/BitsPerComponent`, including values large
//!   enough to exercise the `MAX_ROW_BYTES` clamping path
//!
//! Every input is decoded twice, mirroring `stream_decoder`:
//! 1. with the default 512 MiB document budget (INV-8: must never panic)
//! 2. with a 100-byte budget (EC-10: decompression bomb limit must hold)
//!
//! Input format: a fixed 6-byte parameter header followed by the LZW stream.
//!
//! | byte | meaning                                                        |
//! |------|----------------------------------------------------------------|
//! | 0    | predictor selector: `sel % 8` -> 0:1, 1:2, 2..7:10..15          |
//! | 1    | /EarlyChange: `b % 2` -> 0 or 1                                 |
//! | 2    | /Columns low byte: `b % 16 + 1` -> 1..=16                        |
//! | 3    | /Colors: `b % 4 + 1` -> 1..=4                                   |
//! | 4    | /BitsPerComponent: `[1, 2, 4, 8, 16][b % 5]`                    |
//! | 5    | /Columns high byte: `b * 256` (0..=65280). Columns spans        |
//! |      | 1..=65551, so the MAX_ROW_BYTES (64 KB) row-size clamp in       |
//! |      | `PredictorParams::bytes_per_row` / `is_row_size_clamped` is    |
//! |      | reachable (16-bit 4-color input needs > 8192 columns)          |
//!
//! The fixed header keeps libFuzzer's mutations concentrated on the stream
//! payload while the whole parameter space stays reachable. Seeds with real
//! LZW streams from `tests/fixtures/` live in `fuzz/seeds/lzw_decode/`
//! (regenerate with `scripts/gen_lzw_fuzz_seeds.py`).
//!
//! Seeded and executed nightly by the `pdftract-nightly-fuzz` CronWorkflow
//! under the memory ceiling (bf-1g1fd / bf-5dnh1): 1.5 GiB cgroup MemoryMax,
//! libFuzzer -rss_limit_mb=1024, -malloc_limit_mb=1024, -max_len=10000.
//! Measured coverage/no-crash evidence: `docs/notes/lzw-fuzz-coverage.md`.

#![no_main]
use libfuzzer_sys::fuzz_target;

use pdftract_core::parser::object::{PdfDict, PdfObject};
use pdftract_core::parser::stream::{LZWDecoder, StreamDecoder, DEFAULT_MAX_DECOMPRESS_BYTES};

/// Hard cap on the LZW stream payload accepted per execution.
///
/// CI passes `-max_len=10000` (see `pdftract-nightly-fuzz.yaml`); this bound
/// additionally protects bare local invocations that pass no `-max_len`.
const MAX_LZW_INPUT: usize = 16 * 1024;

/// Low bomb limit for the EC-10 (decompression bomb) leg.
const BOMB_LIMIT: u64 = 100;

fn predictor_from_selector(sel: u8) -> i64 {
    match sel % 8 {
        0 => 1,
        1 => 2,
        n => 8 + n as i64, // 10..=15
    }
}

fn bits_per_component_from_selector(sel: u8) -> i64 {
    [1, 2, 4, 8, 16][sel as usize % 5]
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 6 {
        return;
    }

    let predictor = predictor_from_selector(data[0]);
    let early_change = (data[1] % 2) as i64;
    let columns = (data[5] as i64) * 256 + (data[2] % 16) as i64 + 1;
    let colors = (data[3] % 4) as i64 + 1;
    let bits_per_component = bits_per_component_from_selector(data[4]);

    let end = data.len().min(6 + MAX_LZW_INPUT);
    let payload = &data[6..end];

    let mut dict = PdfDict::new();
    dict.insert("/Predictor".into(), PdfObject::Integer(predictor));
    dict.insert("/EarlyChange".into(), PdfObject::Integer(early_change));
    dict.insert("/Columns".into(), PdfObject::Integer(columns));
    dict.insert("/Colors".into(), PdfObject::Integer(colors));
    dict.insert(
        "/BitsPerComponent".into(),
        PdfObject::Integer(bits_per_component),
    );
    let params = PdfObject::Dict(Box::new(dict));

    // INV-8: decoding must never panic at the public boundary, for either
    // EarlyChange variant and any predictor combination.
    let mut counter = 0;
    let _ = LZWDecoder.decode(
        payload,
        Some(&params),
        &mut counter,
        DEFAULT_MAX_DECOMPRESS_BYTES,
    );

    // EC-10: the bomb limit must hold under adversarial LZW input.
    let mut counter = 0;
    let _ = LZWDecoder.decode(payload, Some(&params), &mut counter, BOMB_LIMIT);
});
