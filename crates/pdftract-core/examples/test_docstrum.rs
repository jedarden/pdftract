/// Standalone test for Docstrum algorithm verification.
/// This verifies the acceptance criteria for bead pdftract-4bylb.

use pdftract_core::layout::reading_order::{docstrum, BlockWithBBox};

fn main() {
    println!("Testing Docstrum algorithm...\n");

    // Test 1: Magazine main + sidebar
    println!("Test 1: Magazine main + sidebar");
    let blocks = vec![
        BlockWithBBox::new(0, [50.0, 700.0, 250.0, 750.0]),  // main, top
        BlockWithBBox::new(1, [50.0, 600.0, 250.0, 650.0]),  // main, mid
        BlockWithBBox::new(2, [50.0, 500.0, 250.0, 550.0]),  // main, bot
        BlockWithBBox::new(3, [350.0, 680.0, 450.0, 720.0]), // sidebar, top
        BlockWithBBox::new(4, [350.0, 620.0, 450.0, 660.0]), // sidebar, mid
    ];

    let order = docstrum(&blocks);
    println!("  Order: {:?}", order);

    // Find where sidebar blocks appear
    let sidebar_pos = order.iter().position(|&i| i >= 3).unwrap_or(order.len());
    let main_blocks: Vec<_> = order.iter().filter(|&&i| i < 3).collect();

    assert_eq!(main_blocks.len(), 3, "main column should have 3 blocks");
    assert!(sidebar_pos >= 3, "sidebar should start after main column");
    println!("  PASS: Main column (0,1,2) before sidebar (3,4)\n");

    // Test 2: Pathological scattered
    println!("Test 2: Pathological scattered");
    let blocks = vec![
        BlockWithBBox::new(0, [50.0, 700.0, 100.0, 750.0]),
        BlockWithBBox::new(1, [150.0, 600.0, 200.0, 650.0]),
        BlockWithBBox::new(2, [250.0, 500.0, 300.0, 550.0]),
        BlockWithBBox::new(3, [350.0, 400.0, 400.0, 450.0]),
    ];

    let order = docstrum(&blocks);
    println!("  Order: {:?}", order);

    assert_eq!(order.len(), 4, "all 4 blocks should be in the order");

    // No duplicate blocks
    let mut sorted = order.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), 4, "no duplicate blocks");
    println!("  PASS: All blocks in order, no duplicates\n");

    // Test 3: All one line horizontal
    println!("Test 3: All one line horizontal");
    let blocks = vec![
        BlockWithBBox::new(0, [50.0, 700.0, 100.0, 750.0]),
        BlockWithBBox::new(1, [120.0, 700.0, 170.0, 750.0]),
        BlockWithBBox::new(2, [190.0, 700.0, 240.0, 750.0]),
    ];

    let order = docstrum(&blocks);
    println!("  Order: {:?}", order);

    assert_eq!(order.len(), 3, "all blocks should be in one component");
    assert_eq!(order, vec![0, 1, 2], "order should be left-to-right (0, 1, 2)");
    println!("  PASS: Single component, left-to-right order\n");

    // Test 4: All one column vertical
    println!("Test 4: All one column vertical");
    let blocks = vec![
        BlockWithBBox::new(0, [50.0, 700.0, 100.0, 750.0]), // top
        BlockWithBBox::new(1, [50.0, 600.0, 100.0, 650.0]), // middle
        BlockWithBBox::new(2, [50.0, 500.0, 100.0, 550.0]), // bottom
    ];

    let order = docstrum(&blocks);
    println!("  Order: {:?}", order);

    assert_eq!(order.len(), 3, "all blocks should be in one component");
    assert_eq!(order, vec![0, 1, 2], "order should be top-to-bottom (0, 1, 2)");
    println!("  PASS: Single component, top-to-bottom order\n");

    println!("All Docstrum acceptance criteria tests PASSED!");
}
