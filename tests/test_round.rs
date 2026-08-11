fn round_x(x: f64) -> i32 {
    x.round() as i32
}

#[test]
fn test_round_x() {
    assert_eq!(round_x(-2.3), -2);
    assert_eq!(round_x(-0.5), 0);
    assert_eq!(round_x(-0.1), 0);
    assert_eq!(round_x(-0.6), -1);
    assert_eq!(round_x(-2.7), -3);
}
