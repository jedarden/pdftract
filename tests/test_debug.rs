use std::vec;

struct TestBitmap {
    pixels: Vec<Vec<u8>>,
    width: i32,
    height: i32,
}

impl TestBitmap {
    fn new(width: i32, height: i32) -> Self {
        let pixels = vec![vec![255u8; width as usize]; height as usize];
        Self {
            pixels,
            width,
            height,
        }
    }

    fn white(width: i32, height: i32) -> Self {
        Self::new(width, height)
    }

    fn get(&self, x: i32, y: i32) -> Option<u8> {
        if x < 0 || x >= self.width || y < 0 || y >= self.height {
            return None;
        }
        Some(self.pixels[y as usize][x as usize])
    }
}

trait Bitmap {
    fn set(&mut self, x: i32, y: i32, value: u8) -> bool;
    fn width(&self) -> i32;
    fn height(&self) -> i32;
}

impl Bitmap for TestBitmap {
    fn set(&mut self, x: i32, y: i32, value: u8) -> bool {
        if x < 0 || x >= self.width || y < 0 || y >= self.height {
            return false;
        }
        self.pixels[y as usize][x as usize] = value;
        true
    }

    fn width(&self) -> i32 {
        self.width
    }

    fn height(&self) -> i32 {
        self.height
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Edge {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
}

impl Edge {
    const fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> Self {
        Self { x0, y0, x1, y1 }
    }

    const fn is_horizontal(&self) -> bool {
        self.y0 == self.y1
    }

    const fn y_bounds(&self) -> (i32, i32) {
        if self.y0 < self.y1 {
            (self.y0, self.y1)
        } else {
            (self.y1, self.y0)
        }
    }
}

fn fill_polygon<B: Bitmap>(bitmap: &mut B, edges: &[Edge], fill_value: u8) {
    if edges.is_empty() {
        return;
    }

    let width = bitmap.width();
    let height = bitmap.height();

    // Find y-bounds
    let mut min_y = height;
    let mut max_y = 0i32;

    for edge in edges {
        let (y_min, y_max) = edge.y_bounds();
        min_y = min_y.min(y_min);
        max_y = max_y.max(y_max);
    }

    // Clamp to bitmap bounds
    min_y = min_y.max(0);
    max_y = max_y.min(height - 1);

    // For each scanline
    for y in min_y..=max_y {
        let mut intersections = Vec::new();

        // Find all intersections with this scanline
        for edge in edges {
            // Skip horizontal edges
            if edge.is_horizontal() {
                continue;
            }

            let (y_min, y_max) = edge.y_bounds();
            if y_min <= y && y < y_max {
                let dy = edge.y1 - edge.y0;
                let t = (y - edge.y0) as f64 / dy as f64;
                let x = edge.x0 as f64 + t * (edge.x1 - edge.x0) as f64;
                intersections.push(x);
            }
        }

        // Sort intersections
        intersections.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Fill between pairs
        for i in (0..intersections.len()).step_by(2) {
            if i + 1 < intersections.len() {
                let x_start = intersections[i].ceil() as i32;
                let x_end = intersections[i + 1].floor() as i32;

                for x in x_start..=x_end {
                    if x >= 0 && x < width {
                        bitmap.set(x, y, fill_value);
                    }
                }
            }
        }
    }
}

#[test]
fn test_fill_polygon_rectangle() {
    let mut bitmap = TestBitmap::white(32, 32);

    let edges = vec![
        Edge::new(10, 10, 20, 10),
        Edge::new(20, 10, 20, 20),
        Edge::new(20, 20, 10, 20),
        Edge::new(10, 20, 10, 10),
    ];

    fill_polygon(&mut bitmap, &edges, 0);

    // Test that corners are filled
    assert_eq!(bitmap.get(10, 10), Some(0), "Top-left corner should be filled");
    assert_eq!(bitmap.get(20, 20), Some(0), "Bottom-right corner should be filled");
    assert_eq!(bitmap.get(15, 15), Some(0), "Center should be filled");

    // Test that points outside the rectangle are not filled
    assert_eq!(bitmap.get(5, 5), Some(255), "Point outside should not be filled");
    assert_eq!(bitmap.get(25, 25), Some(255), "Point outside should not be filled");
}

#[test]
fn test_fill_polygon_triangle() {
    let mut bitmap = TestBitmap::white(32, 32);

    let edges = vec![
        Edge::new(10, 5, 30, 25),
        Edge::new(30, 25, 10, 25),
        Edge::new(10, 25, 10, 5),
    ];

    fill_polygon(&mut bitmap, &edges, 0);

    // Test that points on the base are filled
    assert_eq!(bitmap.get(15, 25), Some(0), "Point on base should be filled");
    assert_eq!(bitmap.get(20, 25), Some(0), "Point on base should be filled");

    // Test that points outside the triangle are not filled
    assert_eq!(bitmap.get(5, 5), Some(255), "Point outside should not be filled");
    assert_eq!(bitmap.get(25, 5), Some(255), "Point outside should not be filled");
}
