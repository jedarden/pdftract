//! Perceptual hash (pHash) implementation for glyph shape recognition.
//!
//! This module implements the pHash algorithm for comparing glyph shapes
//! and looking up glyphs in the shape database.
//!
//! # Algorithm
//!
//! 1. Convert 32×32 grayscale bitmap to float32 values
//! 2. Apply 32×32 2D DCT-II (Discrete Cosine Transform)
//! 3. Extract top-left 8×8 AC coefficients (skipping DC at [0,0])
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
    /// Per the plan, Hamming distance ≤ 8 indicates a similar shape.
    pub fn is_acceptable(&self) -> bool {
        self.distance <= 8
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
/// 1. Scan all entries in the database
/// 2. Compute Hamming distance for each entry
/// 3. Collect entries with distance ≤ 8
/// 4. Return the entry with minimum distance
/// 5. If no entry within threshold, return None
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
/// Per the plan: ~5,000 entries × ~8 ns per XOR+popcount ≈ 40 µs worst-case.
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
    // Get the shape database from the build-generated module
    let db = shape_database();

    // Linear scan: find all entries within Hamming threshold
    let mut best_match: Option<ShapeMatch> = None;
    let mut best_distance = u32::MAX;

    for entry in db.iter() {
        let distance = hamming_distance(query_hash, entry.phash);

        // Only consider matches within the threshold
        if distance <= 8 {
            // Update best match if this is closer
            if distance < best_distance {
                best_distance = distance;
                best_match = Some(ShapeMatch::new(entry.ch, distance));

                // Distance 0 is perfect match, can't do better
                if distance == 0 {
                    break;
                }
            }
        }
    }

    best_match
}

/// Get the shape database slice.
///
/// Returns a slice of (pHash, char) entries sorted by pHash.
/// This is a stub that returns an empty slice; the actual database
/// will be generated from build/glyph-shapes.json in a future bead.
fn shape_database() -> &'static [ShapeEntry] {
    &[]
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
}
