use rust_decimal::Decimal;

pub fn decimal_to_f32(d: Decimal) -> f32 {
    use rust_decimal::prelude::ToPrimitive;
    d.to_f32().unwrap_or(0.0)
}

pub fn f32_to_decimal(f: f32) -> Decimal {
    use rust_decimal::prelude::FromPrimitive;
    Decimal::from_f32(f).unwrap_or(Decimal::ZERO)
}
