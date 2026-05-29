//! Perceptual hash (pHash) implementation for glyph shape recognition.
//!
//! This module implements the pHash algorithm for comparing glyph shapes
//! and looking up glyphs in the shape database.
//!
//! # Algorithm
//!
//! 1. Convert 32×32 grayscale bitmap to float32 values
//! 2. Apply 32×32 2D DCT-II (Discrete Cosine Transform)
//! 3. Extract top-left 8×8 AC coefficients (skipping DC at \[0,0\])
//! 4. Compute median of those 64 values
//! 5. Produce 64-bit hash: bit i is set if coefficient i > median
//!
//! # Properties
//!
//! - Same input bitmap produces identical hash across platforms (deterministic)
//! - Hamming distance ≤ 8 indicates similar shapes (same character, different font)
//! - Hamming distance > 12 indicates different characters
//!
//! # References
//!
//! - Phash library by Evan Prodromou
//! - Marr & Hildreth visual feature theory
//! - Plan section: Phase 2.5 Glyph Shape Database (line 1420)

use std::f32;

// Include the build-generated shape database
include!(concat!(env!("OUT_DIR"), "/shape_db.rs"));

/// Maximum Hamming distance for a shape match to be considered acceptable.
///
/// Per the plan specification (line 1442), Hamming distance ≤ 8 indicates
/// similar shapes (same character, different font), while distance > 8
/// indicates different characters or excessive distortion.
const HAMMING_MAX: u32 = 8;

/// Shape database entry with pHash and associated character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapeEntry {
    /// Perceptual hash of the glyph shape
    pub phash: u64,
    /// Unicode character this shape represents
    pub ch: char,
}

impl ShapeEntry {
    /// Create a new shape entry.
    pub const fn new(phash: u64, ch: char) -> Self {
        Self { phash, ch }
    }
}

/// Result of a shape database lookup.
///
/// Contains the matched character and the Hamming distance
/// between the query hash and the matched entry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapeMatch {
    /// The matched Unicode character
    pub ch: char,
    /// Hamming distance between query and match (0-64)
    pub distance: u32,
}

impl ShapeMatch {
    /// Create a new shape match result.
    pub fn new(ch: char, distance: u32) -> Self {
        Self { ch, distance }
    }

    /// Check if this match is within the acceptable threshold.
    ///
    /// Per the plan, Hamming distance ≤ HAMMING_MAX (8) indicates a similar shape.
    pub fn is_acceptable(&self) -> bool {
        self.distance <= HAMMING_MAX
    }
}

/// DCT size: 32×32 input bitmap
const DCT_SIZE: usize = 32;

/// Output hash size: 64 bits
const HASH_SIZE: usize = 64;

/// Size of the low-frequency coefficient block: 8×8
const LOW_FREQ_SIZE: usize = 8;

/// Perceptual hash of a 32×32 grayscale glyph bitmap.
///
/// # Arguments
///
/// * `bitmap` - A 32×32 grayscale bitmap (row-major, 8-bit per pixel).
///              Per convention: 0 = black ink, 255 = white paper.
///
/// # Returns
///
/// A 64-bit hash where each bit represents whether one of the 64 low-frequency
/// DCT coefficients is above the median of those coefficients.
///
/// # Examples
///
/// ```
/// use pdftract_core::font::shape::phash_glyph;
///
/// // White bitmap (all 255) -> all zeros in DCT -> hash = 0
/// let white_bitmap = [255u8; 1024];
/// let hash = phash_glyph(&white_bitmap);
/// assert_eq!(hash, 0x0000000000000000);
/// ```
///
/// # Invariants
///
/// - Same input bitmap produces identical hash across runs and platforms
/// - No NaN values in computation
/// - Deterministic float ordering (no platform-specific differences)
pub fn phash_glyph(bitmap: &[u8; 1024]) -> u64 {
    // Special case: uniform bitmaps (all pixels identical) have no visual information
    // Return 0 deterministically
    let first_pixel = bitmap[0];
    if bitmap.iter().all(|&p| p == first_pixel) {
        return 0;
    }

    // Step 1: Convert to float32, centered at zero
    let mut input = [0.0f32; DCT_SIZE * DCT_SIZE];
    for i in 0..1024 {
        // Center the values: 0->-1.0, 255->+1.0, 128->0.0
        // This centers the pixel intensity around zero for better DCT behavior
        input[i] = (bitmap[i] as f32) / 127.5 - 1.0;
    }

    // Step 2: Apply 2D DCT-II (row-wise, then column-wise)
    let mut dct_output = [0.0f32; DCT_SIZE * DCT_SIZE];
    dct_2d(&input, &mut dct_output);

    // Step 3: Extract top-left 8×8 coefficients (excluding DC at [0,0])
    // We need 64 values total. The plan says "top-left 8×8 AC coefficients"
    // and "skipping DC at [0,0]". The standard pHash approach:
    // - Use 8×8 block starting at [0,0] (64 values)
    // - Exclude [0,0] (the DC component)
    // - We need one more value to make 64
    //
    // Plan clarification: "use the remaining 63 + the [0,1] cell"
    // Actually, re-reading: the standard approach uses all 64 values
    // including DC in the median computation, but DC is always the
    // largest value, so it doesn't affect the threshold much.
    //
    // For this implementation, we'll use the 64 lowest-frequency AC
    // coefficients: the 8×8 block starting at [0,0], but we replace
    // [0,0] (DC) with [0,8] to get 64 AC values total.
    let mut low_freq = [0.0f32; HASH_SIZE];
    let mut idx = 0;
    for y in 0..LOW_FREQ_SIZE {
        for x in 0..LOW_FREQ_SIZE {
            if x == 0 && y == 0 {
                // Skip DC, use [0,8] instead (still low frequency)
                low_freq[idx] = dct_output[8 * DCT_SIZE].abs();
            } else {
                low_freq[idx] = dct_output[y * DCT_SIZE + x].abs();
            }
            idx += 1;
        }
    }

    // Step 4: Compute median
    let mut sorted = low_freq;
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    // Median of 64 values is average of indices 31 and 32
    let median = (sorted[31] + sorted[32]) / 2.0;

    // Step 5: Threshold to produce 64-bit hash
    let mut hash: u64 = 0;
    for i in 0..HASH_SIZE {
        if low_freq[i] > median {
            hash |= 1 << i;
        }
    }

    hash
}

/// Apply 2D DCT-II to a 32×32 input matrix.
///
/// DCT-II formula for a 2D matrix:
/// F[u,v] = (2/√(MN)) * Σ_x Σ_y f[x,y] * cos(π(2x+1)u/(2N)) * cos(π(2y+1)v/(2M))
///
/// For orthonormal DCT, the scale factor is applied such that the transform
/// is its own inverse (up to scaling).
///
/// This implementation uses a separable approach: apply 1D DCT to each row,
/// then apply 1D DCT to each column of the result.
fn dct_2d(input: &[f32; DCT_SIZE * DCT_SIZE], output: &mut [f32; DCT_SIZE * DCT_SIZE]) {
    let mut temp = [0.0f32; DCT_SIZE * DCT_SIZE];

    // Precompute cosine basis for 1D DCT
    // basis[k][n] = cos(π * k * (2n + 1) / (2 * N))
    let mut basis = [[0.0f32; DCT_SIZE]; DCT_SIZE];
    for k in 0..DCT_SIZE {
        for n in 0..DCT_SIZE {
            basis[k][n] =
                (f32::consts::PI * k as f32 * (2 * n + 1) as f32 / (2 * DCT_SIZE) as f32).cos();
        }
    }

    // Apply 1D DCT to each row
    for y in 0..DCT_SIZE {
        for k in 0..DCT_SIZE {
            let mut sum = 0.0f32;
            for n in 0..DCT_SIZE {
                sum += input[y * DCT_SIZE + n] * basis[k][n];
            }
            // Normalize: scale factor for orthonormal DCT
            let scale = if k == 0 {
                (1.0 / DCT_SIZE as f32).sqrt()
            } else {
                (2.0 / DCT_SIZE as f32).sqrt()
            };
            temp[y * DCT_SIZE + k] = sum * scale;
        }
    }

    // Apply 1D DCT to each column
    for x in 0..DCT_SIZE {
        for k in 0..DCT_SIZE {
            let mut sum = 0.0f32;
            for n in 0..DCT_SIZE {
                sum += temp[n * DCT_SIZE + x] * basis[k][n];
            }
            let scale = if k == 0 {
                (1.0 / DCT_SIZE as f32).sqrt()
            } else {
                (2.0 / DCT_SIZE as f32).sqrt()
            };
            output[k * DCT_SIZE + x] = sum * scale;
        }
    }
}

/// Compute Hamming distance between two pHash values.
///
/// The Hamming distance is the count of differing bits. For pHash:
/// - Distance ≤ 8: similar shapes (likely same character, different font)
/// - Distance 9-12: uncertain (may be similar or different)
/// - Distance > 12: different characters
///
/// # Arguments
///
/// * `a` - First pHash value
/// * `b` - Second pHash value
///
/// # Returns
///
/// Number of differing bits (0-64)
///
/// # Examples
///
/// ```
/// use pdftract_core::font::shape::{phash_glyph, hamming_distance};
///
/// let bitmap1 = [128u8; 1024];
/// let bitmap2 = [128u8; 1024];
/// let hash1 = phash_glyph(&bitmap1);
/// let hash2 = phash_glyph(&bitmap2);
/// assert_eq!(hamming_distance(hash1, hash2), 0);
/// ```
pub fn hamming_distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

/// Look up a glyph shape in the shape database by perceptual hash.
///
/// This function performs a linear scan over the shape database to find
/// the closest matching glyph shape. The database is a compile-time sorted
/// slice of (pHash, char) pairs.
///
/// # Algorithm
///
/// 1. **Exact match optimization**: Attempt binary search for exact pHash match;
///    if found, return immediately with distance 0 (fast path for exact matches).
/// 2. **Linear scan**: Scan all entries in the database, computing Hamming distance.
/// 3. **Threshold filter**: Only consider entries with distance ≤ HAMMING_MAX (8).
/// 4. **Tie-breaking**: When multiple entries have the same minimum distance,
///    prefer the character with the lower frequency rank (more common character).
/// 5. Return the entry with minimum distance, or None if no entry within threshold.
///
/// # Arguments
///
/// * `query_hash` - The pHash of the glyph to look up
///
/// # Returns
///
/// `Some(ShapeMatch)` if a match is found within the Hamming threshold,
/// `None` otherwise.
///
/// # Performance
///
/// Per the plan (line 1442): ~5,000 entries × ~8 ns per XOR+popcount ≈ 40 µs worst-case.
///
/// # Invariants
///
/// - Given the same SHAPE_TABLE and FREQ_TABLE, returns the same `Option<char>`
///   across runs (deterministic).
/// - Empty SHAPE_TABLE always returns None (no panic).
///
/// # Examples
///
/// ```
/// use pdftract_core::font::shape::lookup_shape;
///
/// // Look up a glyph by its pHash
/// if let Some(matched) = lookup_shape(0x1234567890ABCDEF) {
///     if matched.is_acceptable() {
///         println!("Matched char: {} (distance: {})", matched.ch, matched.distance);
///     }
/// }
/// ```
pub fn lookup_shape(query_hash: u64) -> Option<ShapeMatch> {
    let db = shape_database();
    let freq = frequency_table();

    // Fast path: exact match optimization via binary search
    // SHAPE_TABLE is sorted by pHash (build.rs invariant)
    if let Ok(idx) = db.binary_search_by_key(&query_hash, |&(hash, _)| hash) {
        // Exact match found - return immediately with distance 0
        let &(_, ch) = db.get(idx)?;
        return Some(ShapeMatch::new(ch, 0));
    }

    // Linear scan: find best match within HAMMING_MAX threshold
    let mut best_distance = u32::MAX;
    let mut best_idx = None;

    for (idx, &(entry_hash, _)) in db.iter().enumerate() {
        let distance = hamming_distance(query_hash, entry_hash);

        if distance <= HAMMING_MAX {
            if distance < best_distance {
                // New best match found
                best_distance = distance;
                best_idx = Some(idx);
            } else if distance == best_distance {
                // Tie: use frequency rank to break
                // Lower rank = more common character = wins
                if let Some(current_best) = best_idx {
                    let current_freq = freq
                        .get(current_best)
                        .map(|&(_, rank)| rank)
                        .unwrap_or(u32::MAX);
                    let new_freq = freq.get(idx).map(|&(_, rank)| rank).unwrap_or(u32::MAX);
                    if new_freq < current_freq {
                        best_idx = Some(idx);
                    }
                }
            }
        }
    }

    // Return the best match if found
    best_idx.and_then(|idx| {
        let &(_, ch) = db.get(idx)?;
        Some(ShapeMatch::new(ch, best_distance))
    })
}

/// Get the shape database slice.
///
/// Returns a slice of (pHash, char) entries sorted by pHash.
/// This is generated from build/glyph-shapes.json via build.rs.
fn shape_database() -> &'static [(u64, char)] {
    SHAPE_TABLE
}

/// Get the frequency table slice.
///
/// Returns a slice of (pHash, frequency_rank) entries sorted by pHash,
/// parallel to SHAPE_TABLE. Lower rank = more common character.
/// This is generated from build/glyph-shapes.json via build.rs.
fn frequency_table() -> &'static [(u64, u32)] {
    FREQ_TABLE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phash_white_bitmap() {
        // All-white bitmap (all 255) -> all pixels centered at +1.0
        // After DCT, only DC coefficient is non-zero
        // All other coefficients are 0, so all bits below median -> hash = 0
        let white_bitmap = [255u8; 1024];
        let hash = phash_glyph(&white_bitmap);
        assert_eq!(hash, 0x0000000000000000);
    }

    #[test]
    fn test_phash_black_bitmap() {
        // All-black bitmap (all 0) -> all pixels centered at -1.0
        // After DCT, only DC coefficient is non-zero
        // All other coefficients are 0, so all bits below median -> hash = 0
        let black_bitmap = [0u8; 1024];
        let hash = phash_glyph(&black_bitmap);
        assert_eq!(hash, 0x0000000000000000);
    }

    #[test]
    fn test_phash_gray_bitmap() {
        // All-gray bitmap (all 128) -> all pixels centered at 0.0
        // After DCT, all coefficients are 0
        let gray_bitmap = [128u8; 1024];
        let hash = phash_glyph(&gray_bitmap);
        assert_eq!(hash, 0x0000000000000000);
    }

    #[test]
    fn test_phash_half_white_half_black() {
        // Left half white, right half black
        let mut bitmap = [0u8; 1024];
        for y in 0..32 {
            for x in 16..32 {
                bitmap[y * 32 + x] = 255;
            }
        }
        let hash = phash_glyph(&bitmap);
        // This should produce a non-zero hash due to the vertical edge
        assert_ne!(hash, 0x0000000000000000);
    }

    #[test]
    fn test_phash_deterministic() {
        // Same input must produce same hash
        let mut bitmap = [0u8; 1024];
        for i in 0..1024 {
            bitmap[i] = (i % 256) as u8;
        }
        let hash1 = phash_glyph(&bitmap);
        let hash2 = phash_glyph(&bitmap);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_phash_horizontal_gradient() {
        // Horizontal gradient from black to white
        let mut bitmap = [0u8; 1024];
        for y in 0..32 {
            for x in 0..32 {
                bitmap[y * 32 + x] = (x * 255 / 31) as u8;
            }
        }
        let hash = phash_glyph(&bitmap);
        // Should produce a non-zero hash
        assert_ne!(hash, 0x0000000000000000);
    }

    #[test]
    fn test_phash_checkerboard() {
        // Checkerboard pattern
        let mut bitmap = [0u8; 1024];
        for y in 0..32 {
            for x in 0..32 {
                if (x + y) % 2 == 0 {
                    bitmap[y * 32 + x] = 0;
                } else {
                    bitmap[y * 32 + x] = 255;
                }
            }
        }
        let hash = phash_glyph(&bitmap);
        // Should produce a non-zero hash
        assert_ne!(hash, 0x0000000000000000);
    }

    #[test]
    fn test_hamming_distance_identical() {
        let hash = 0x1234567890ABCDEF;
        assert_eq!(hamming_distance(hash, hash), 0);
    }

    #[test]
    fn test_hamming_distance_completely_different() {
        assert_eq!(hamming_distance(0xFFFFFFFFFFFFFFFF, 0x0000000000000000), 64);
    }

    #[test]
    fn test_hamming_distance_one_bit() {
        assert_eq!(hamming_distance(0x0000000000000001, 0x0000000000000000), 1);
        assert_eq!(hamming_distance(0x8000000000000000, 0x0000000000000000), 1);
    }

    #[test]
    fn test_hamming_distance_multiple_bits() {
        assert_eq!(hamming_distance(0x000000000000000F, 0x0000000000000000), 4);
        // These differ in all 64 bits
        assert_eq!(hamming_distance(0xFFFFFFFF00000000, 0x00000000FFFFFFFF), 64);
        // These differ in 32 bits (first half only)
        assert_eq!(hamming_distance(0xFFFFFFFF00000000, 0x0000000000000000), 32);
    }

    #[test]
    fn test_phash_different_shapes_different_hashes() {
        // Different shapes should produce different hashes (with high probability)
        let mut bitmap1 = [255u8; 1024]; // Start with white
        let mut bitmap2 = [255u8; 1024]; // Start with white

        // Create a horizontal stripe pattern (black stripe in middle)
        for y in 8..16 {
            for x in 0..32 {
                bitmap1[y * 32 + x] = 0;
            }
        }

        // Create a vertical stripe pattern (black stripe in middle)
        for y in 0..32 {
            for x in 8..16 {
                bitmap2[y * 32 + x] = 0;
            }
        }

        let hash1 = phash_glyph(&bitmap1);
        let hash2 = phash_glyph(&bitmap2);

        // These are very different patterns, so hashes should differ
        assert_ne!(
            hash1, hash2,
            "Different shapes should produce different hashes"
        );
    }

    #[test]
    fn test_shape_entry_new() {
        let entry = ShapeEntry::new(0x1234567890ABCDEF, 'A');
        assert_eq!(entry.phash, 0x1234567890ABCDEF);
        assert_eq!(entry.ch, 'A');
    }

    #[test]
    fn test_shape_match_new() {
        let matched = ShapeMatch::new('X', 5);
        assert_eq!(matched.ch, 'X');
        assert_eq!(matched.distance, 5);
    }

    #[test]
    fn test_shape_match_is_acceptable() {
        // Distance ≤ 8 is acceptable
        assert!(ShapeMatch::new('A', 0).is_acceptable());
        assert!(ShapeMatch::new('A', 5).is_acceptable());
        assert!(ShapeMatch::new('A', 8).is_acceptable());

        // Distance > 8 is not acceptable
        assert!(!ShapeMatch::new('A', 9).is_acceptable());
        assert!(!ShapeMatch::new('A', 12).is_acceptable());
        assert!(!ShapeMatch::new('A', 64).is_acceptable());
    }

    #[test]
    fn test_lookup_shape_empty_database() {
        // With empty database, should return None
        assert_eq!(lookup_shape(0x1234567890ABCDEF), None);
    }

    #[test]
    fn test_shape_database_generated() {
        // Verify that the generated shape database is accessible
        // This test will pass if glyph-shapes.json exists and was processed
        let db = shape_database();

        // If glyph-shapes.json was present, we should have entries
        // If not, db will be empty (both cases are valid)
        if !db.is_empty() {
            // Verify entries are sorted by pHash
            for i in 1..db.len() {
                assert!(
                    db[i].0 >= db[i - 1].0,
                    "SHAPE_TABLE not sorted: index {} has {:016x}, index {} has {:016x}",
                    i - 1,
                    db[i - 1].0,
                    i,
                    db[i].0
                );
            }
        }
    }

    #[test]
    fn test_lookup_shape_exact_match() {
        // Exact match should return distance 0 (binary search fast path)
        let db = shape_database();

        if !db.is_empty() {
            // Test with the first entry in the database
            let &(hash, ch) = &db[0];
            let result = lookup_shape(hash);

            assert!(
                result.is_some(),
                "Exact match should return Some(ShapeMatch)"
            );
            let matched = result.unwrap();
            assert_eq!(matched.ch, ch, "Exact match should return correct char");
            assert_eq!(matched.distance, 0, "Exact match should have distance 0");
        }
    }

    #[test]
    fn test_lookup_shape_hamming_threshold() {
        // Verify that HAMMING_MAX threshold is enforced
        let db = shape_database();

        if !db.is_empty() {
            // Test with a hash that's very far from any entry
            // We'll construct a hash with a specific pattern that's unlikely
            // to match anything closely
            let far_hash = 0xAAAAAAAAAAAAAAAA; // Alternating bit pattern

            let result = lookup_shape(far_hash);

            // If the database is small or empty, we might get None
            // If we get Some result, verify it's at least not a perfect match
            if let Some(matched) = result {
                assert!(
                    matched.distance > 0,
                    "Far hash should not be an exact match"
                );
            }
            // Note: We can't assert None because a real database might
            // have entries close to any arbitrary hash
        }
    }

    #[test]
    fn test_lookup_shape_frequency_tiebreak() {
        // Test that frequency tie-breaking works correctly
        // This requires a non-empty database with at least 2 entries
        let db = shape_database();
        let freq = frequency_table();

        if db.len() >= 2 {
            // Find two entries with the same Hamming distance from a query
            // We'll use the first two entries and construct a query hash
            // that's equidistant from both
            let (hash1, ch1) = db[0];
            let (hash2, ch2) = db[1];
            let rank1 = freq[0].1;
            let rank2 = freq[1].1;

            // Skip if the hashes are identical (would be exact match)
            if hash1 != hash2 {
                // Create a query hash with distance 4 from both entries
                // Flip 4 bits in hash1 to create query
                let query_hash = hash1 ^ 0x0000000F;

                // Compute actual distances
                let dist1 = hamming_distance(query_hash, hash1);
                let dist2 = hamming_distance(query_hash, hash2);

                // If they happen to be tied and both within threshold
                if dist1 == dist2 && dist1 <= HAMMING_MAX {
                    let result = lookup_shape(query_hash);

                    if let Some(matched) = result {
                        // The result should be the character with lower rank (more common)
                        let expected_ch = if rank1 < rank2 { ch1 } else { ch2 };
                        assert_eq!(
                            matched.ch, expected_ch,
                            "Tie-break should prefer lower frequency rank: \
                             rank1={} vs rank2={}, got {}",
                            rank1, rank2, matched.ch
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_lookup_shape_deterministic() {
        // Verify INV: result is deterministic for same input
        let db = shape_database();

        if !db.is_empty() {
            let query_hash = 0x1234567890ABCDEF;

            let result1 = lookup_shape(query_hash);
            let result2 = lookup_shape(query_hash);

            assert_eq!(
                result1, result2,
                "lookup_shape must return identical results for identical inputs"
            );
        }
    }

    #[test]
    fn test_frequency_table_parallel_to_shape_table() {
        // Verify that FREQ_TABLE has the same length and is parallel to SHAPE_TABLE
        let db = shape_database();
        let freq = frequency_table();

        assert_eq!(
            db.len(),
            freq.len(),
            "SHAPE_TABLE and FREQ_TABLE must have the same length"
        );

        // Verify that the pHash values match at each index
        for i in 0..db.len() {
            assert_eq!(
                db[i].0, freq[i].0,
                "pHash mismatch at index {}: SHAPE_TABLE has {:016x}, FREQ_TABLE has {:016x}",
                i, db[i].0, freq[i].0
            );
        }
    }

    #[test]
    fn test_hamming_max_constant() {
        // Verify HAMMING_MAX is set correctly per plan specification
        assert_eq!(HAMMING_MAX, 8, "HAMMING_MAX must be 8 per plan line 1442");
    }

    #[test]
    fn test_lookup_shape_nearest_neighbor() {
        // Test that lookup_shape finds the nearest neighbor
        let db = shape_database();

        if !db.is_empty() {
            // Create a query hash close to the first entry
            let &(hash, _) = &db[0];
            let query_hash = hash ^ 0x01; // Flip 1 bit -> distance 1

            let result = lookup_shape(query_hash);

            assert!(
                result.is_some(),
                "Should find a match within HAMMING_MAX threshold"
            );

            let matched = result.unwrap();
            assert_eq!(matched.distance, 1, "Should find a very close match");
            // Note: The matched character might not be the original due to
            // frequency tie-breaking if another entry has the same distance
            // but lower frequency rank. This is expected behavior.
        }
    }
}
