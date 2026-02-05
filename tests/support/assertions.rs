use rust_decimal::Decimal;

pub fn assert_decimal_near(actual: Decimal, expected: Decimal, tolerance: Decimal) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= tolerance,
        "expected {} ± {}, got {}",
        expected,
        tolerance,
        actual
    );
}
