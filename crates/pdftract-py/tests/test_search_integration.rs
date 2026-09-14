//! Integration tests for the Python `pdftract.search()` function.
//!
//! These tests drive the real Python package (`python/pdftract`) through a
//! `python3` subprocess, so they exercise exactly the surface a Python user
//! sees: the native `_native.search()` binding and the typed `pdftract.search()`
//! wrapper built on top of it.
//!
//! They are the TDD "red" half of bead bf-6cevpy: `search()` currently comes
//! back with an empty match list (or an outright extraction error) for a
//! pattern that verifiably exists in the fixture. Every assertion below is
//! written to fail until that is fixed; once search() is repaired the same
//! assertions become its regression guard.
//!
//! Fixture: `tests/fixtures/sample.pdf` is a 534-byte hand-written PDF whose
//! single page draws the text `Test` — its content stream contains the literal
//! operator `(Test) Tj` — so a case-sensitive search for `"Test"` must return
//! at least one match on page 0.

use std::io::Read;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

/// This crate's directory (…/crates/pdftract-py), resolved at compile time so
/// the tests work regardless of the working directory cargo test runs in.
const CRATE_DIR: &str = env!("CARGO_MANIFEST_DIR");

/// Fixture with known text: its content stream contains `(Test) Tj`.
const FIXTURE: &str = "tests/fixtures/sample.pdf";

/// A pattern that verifiably occurs in [`FIXTURE`].
const PATTERN: &str = "Test";

/// A driver run is a batch search over a 534-byte PDF; anything past this is
/// a hang and gets killed rather than wedging the test runner.
const DRIVER_TIMEOUT: Duration = Duration::from_secs(60);

/// Python driver: runs both search surfaces against one fixture and prints a
/// single JSON line. Extraction/mapping failures are reported in the `error`
/// field instead of crashing, so the Rust side controls how the failure is
/// presented.
const DRIVER_SRC: &str = r#"
import json, sys

fixture, pattern, pkg_dir = sys.argv[1], sys.argv[2], sys.argv[3]
sys.path.insert(0, pkg_dir)

out = {"error": None, "native_pattern": None, "native_matches": [], "typed_matches": []}
try:
    import pdftract
    from pdftract import _native

    native = _native.search(fixture, pattern)
    out["native_pattern"] = native.get("pattern")
    out["native_matches"] = native.get("matches") or []

    typed = list(pdftract.search(fixture, pattern))
    out["typed_matches"] = [
        {"text": m.text, "page": m.page, "bbox": list(m.bbox)} for m in typed
    ]
except BaseException as exc:  # reported verbatim to the Rust side
    out["error"] = f"{type(exc).__name__}: {exc}"

print(json.dumps(out))
"#;

/// RAII guard owning the driver child: it is killed and reaped on scope exit,
/// including on panic or early return (repo test-hygiene rule for tests that
/// spawn processes).
struct ChildGuard(Child);

impl ChildGuard {
    fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        self.0.try_wait()
    }

    fn kill_and_reap(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }

    /// Mutable access to the child once it is known to have exited, so its
    /// pipes can be read while the guard still owns it.
    fn child(&mut self) -> &mut Child {
        &mut self.0
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        self.kill_and_reap();
    }
}

/// Run the Python driver against [`FIXTURE`] and return its parsed JSON report.
///
/// Panics (with the driver output attached) if python cannot be spawned, the
/// run exceeds [`DRIVER_TIMEOUT`], or the driver produces no parseable JSON.
fn run_driver() -> Value {
    let fixture = PathBuf::from(CRATE_DIR).join(FIXTURE);
    let pkg_dir = PathBuf::from(CRATE_DIR).join("python");

    let child = Command::new("python3")
        .arg("-B")
        .arg("-c")
        .arg(DRIVER_SRC)
        .arg(&fixture)
        .arg(PATTERN)
        .arg(&pkg_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn python3 driver: {e}"));

    let mut guard = ChildGuard(child);

    // Bounded wait: poll instead of an unbounded child.wait() so a wedged
    // driver becomes a timeout panic, not a hung test. The guard owns the
    // child, so every path out of this loop leaves it killed and reaped.
    let deadline = Instant::now() + DRIVER_TIMEOUT;
    let mut status = None;
    loop {
        match guard.try_wait().expect("failed to poll driver status") {
            Some(exit) => {
                status = Some(exit);
                break;
            }
            None if Instant::now() >= deadline => {
                guard.kill_and_reap();
                break;
            }
            None => std::thread::sleep(Duration::from_millis(50)),
        }
    }

    let status = match status {
        Some(status) => status,
        None => panic!(
            "python driver timed out after {}s searching {:?} for {:?}",
            DRIVER_TIMEOUT.as_secs(),
            fixture,
            PATTERN
        ),
    };

    // The child has exited (or been killed), so these reads cannot hang. The
    // driver prints one small JSON line, far below the pipe buffer capacity.
    let mut stdout = String::new();
    let _ = guard
        .child()
        .stdout
        .take()
        .expect("driver stdout was piped")
        .read_to_string(&mut stdout);
    let mut stderr = String::new();
    let _ = guard
        .child()
        .stderr
        .take()
        .expect("driver stderr was piped")
        .read_to_string(&mut stderr);
    assert!(
        status.success(),
        "python driver exited with {status}\nstdout: {stdout}\nstderr: {stderr}"
    );

    // Take the last parseable line so a stray print from the package cannot
    // break the report.
    stdout
        .lines()
        .rev()
        .find_map(|line| serde_json::from_str::<Value>(line).ok())
        .unwrap_or_else(|| {
            panic!("python driver produced no JSON report\nstdout: {stdout}\nstderr: {stderr}")
        })
}

/// Failure preamble shared by both tests: what was searched, where, and what
/// the driver actually reported.
fn describe_failure(report: &Value) -> String {
    let error = report["error"].as_str().unwrap_or("-");
    let native = report["native_matches"].as_array().map_or(0, Vec::len);
    let typed = report["typed_matches"].as_array().map_or(0, Vec::len);
    format!(
        "fixture: {CRATE_DIR}/{FIXTURE} (content stream contains `(Test) Tj`), \
pattern: {PATTERN:?}, driver error: {error}, native matches: {native}, typed matches: {typed}"
    )
}

/// The headline bug: `search()` must find a pattern that exists in the PDF.
///
/// Currently FAILS: the matches list comes back empty even though the fixture
/// contains `(Test) Tj`. This is the documented search() bug this bead pins
/// down; the assertion becomes a regression guard once search() is fixed.
#[test]
fn test_search_empty_result_when_pattern_present() {
    let report = run_driver();

    let error = report["error"].as_str().unwrap_or_default();
    assert!(
        error.is_empty(),
        "search() raised instead of returning matches — pattern {PATTERN:?} is present \
in the fixture, so at least one match is expected. {}",
        describe_failure(&report)
    );

    let native = report["native_matches"]
        .as_array()
        .expect("native_matches list");
    assert!(
        !native.is_empty(),
        "native _native.search() returned an EMPTY match list even though the pattern \
{PATTERN:?} exists in the fixture — the documented empty-matches bug. {}",
        describe_failure(&report)
    );

    let typed = report["typed_matches"]
        .as_array()
        .expect("typed_matches list");
    assert!(
        !typed.is_empty(),
        "pdftract.search() yielded an EMPTY iterator even though the pattern {PATTERN:?} \
exists in the fixture and the native layer reported {} raw matches — the typed wrapper \
must surface them. {}",
        native.len(),
        describe_failure(&report)
    );

    assert_eq!(
        report["native_pattern"].as_str(),
        Some(PATTERN),
        "native result should echo the requested pattern. {}",
        describe_failure(&report)
    );
}

/// Each match must carry the full structure: `page_index`, `span_index`,
/// `text` and `bbox` on native matches, and the equivalent typed fields on
/// the `Match` objects `pdftract.search()` yields.
///
/// Currently FAILS at the non-empty guard — the list is empty, so there is
/// nothing whose structure could be verified (the other documented symptom).
#[test]
fn test_search_returns_match_structure() {
    let report = run_driver();

    let native = report["native_matches"]
        .as_array()
        .expect("native_matches list");
    assert!(
        !native.is_empty(),
        "cannot verify match structure: the native match list is empty because search() \
found nothing for a pattern that exists in the fixture. {}",
        describe_failure(&report)
    );

    for (i, m) in native.iter().enumerate() {
        let page_index = m.get("page_index").and_then(Value::as_u64);
        assert!(
            page_index.is_some(),
            "native match #{i} is missing an integer `page_index` field: {m}"
        );
        let span_index = m.get("span_index").and_then(Value::as_u64);
        assert!(
            span_index.is_some(),
            "native match #{i} is missing an integer `span_index` field: {m}"
        );
        let text = m.get("text").and_then(Value::as_str);
        assert!(
            text.is_some_and(|t| !t.is_empty()),
            "native match #{i} is missing a non-empty `text` field: {m}"
        );
        let bbox = m.get("bbox").and_then(Value::as_array);
        assert!(
            bbox.is_some_and(|b| b.len() == 4 && b.iter().all(|x| x.is_number())),
            "native match #{i} needs a `bbox` of 4 numbers [x0, y0, x1, y1]: {m}"
        );
    }

    // The fixture has exactly one page and one text run, so the first match
    // should land on page 0 and the matched text should be the pattern itself.
    let first = &native[0];
    assert_eq!(
        first["page_index"].as_u64(),
        Some(0),
        "expected the first match on page 0: {first}"
    );
    assert_eq!(
        first["text"].as_str(),
        Some(PATTERN),
        "expected the matched text to be the pattern itself: {first}"
    );

    let typed = report["typed_matches"]
        .as_array()
        .expect("typed_matches list");
    assert!(
        !typed.is_empty(),
        "cannot verify typed match structure: pdftract.search() yielded nothing. {}",
        describe_failure(&report)
    );
    for (i, m) in typed.iter().enumerate() {
        let text = m.get("text").and_then(Value::as_str);
        assert!(
            text.is_some_and(|t| !t.is_empty()),
            "typed match #{i} is missing a non-empty `text` field: {m}"
        );
        let page = m.get("page").and_then(Value::as_u64);
        assert!(
            page.is_some(),
            "typed match #{i} is missing a non-negative integer `page` field: {m}"
        );
        let bbox = m.get("bbox").and_then(Value::as_array);
        assert!(
            bbox.is_some_and(|b| b.len() == 4 && b.iter().all(|x| x.is_number())),
            "typed match #{i} needs a `bbox` of 4 numbers [x0, y0, x1, y1]: {m}"
        );
    }
}
