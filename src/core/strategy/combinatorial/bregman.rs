//! Bregman divergence calculations for LMSR cost function.
//!
//! For the Logarithmic Market Scoring Rule (LMSR):
//! - Cost function: C(q) = b * log(Σ exp(qᵢ/b))
//! - Conjugate: R(μ) = Σ μᵢ * ln(μᵢ) (negative entropy)
//! - Bregman divergence: D(μ||θ) = KL divergence
//!
//! The divergence D(μ*||θ) equals the maximum arbitrage profit.

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

/// Compute Bregman divergence D(μ||θ) for LMSR.
///
/// This is the KL divergence when R is negative entropy.
/// D(μ||θ) = Σ μᵢ * ln(μᵢ/θᵢ)
///
/// # Arguments
/// * `mu` - Target probability vector (must be valid distribution)
/// * `theta` - Current market prices
#[must_use]
pub fn bregman_divergence(mu: &[Decimal], theta: &[Decimal]) -> Decimal {
    if mu.len() != theta.len() || mu.is_empty() {
        return Decimal::ZERO;
    }

    let mut divergence = Decimal::ZERO;
    let epsilon = Decimal::new(1, 10); // 1e-10 for numerical stability

    for (m, t) in mu.iter().zip(theta.iter()) {
        if *m > epsilon && *t > epsilon {
            // μ * ln(μ/θ) = μ * (ln(μ) - ln(θ))
            let m_f64 = m.to_f64().unwrap_or(0.0);
            let t_f64 = t.to_f64().unwrap_or(1.0);

            if m_f64 > 0.0 && t_f64 > 0.0 {
                let term = m_f64 * (m_f64.ln() - t_f64.ln());
                divergence += Decimal::try_from(term).unwrap_or(Decimal::ZERO);
            }
        }
    }

    divergence
}

/// Compute gradient of Bregman divergence ∇D(μ||θ) w.r.t. μ.
///
/// For KL divergence: ∂D/∂μᵢ = ln(μᵢ/θᵢ) + 1
///
/// # Arguments
/// * `mu` - Current iterate
/// * `theta` - Target (fixed)
#[must_use]
pub fn bregman_gradient(mu: &[Decimal], theta: &[Decimal]) -> Vec<Decimal> {
    let epsilon = Decimal::new(1, 10);

    mu.iter()
        .zip(theta.iter())
        .map(|(m, t)| {
            let m_safe = (*m).max(epsilon);
            let t_safe = (*t).max(epsilon);

            let m_f64 = m_safe.to_f64().unwrap_or(1.0);
            let t_f64 = t_safe.to_f64().unwrap_or(1.0);

            // ln(μ/θ) + 1 = ln(μ) - ln(θ) + 1
            let grad = m_f64.ln() - t_f64.ln() + 1.0;
            Decimal::try_from(grad).unwrap_or(Decimal::ZERO)
        })
        .collect()
}

/// Compute the LMSR cost function C(q).
///
/// C(q) = b * ln(Σ exp(qᵢ/b))
///
/// # Arguments
/// * `q` - Quantity vector
/// * `b` - Liquidity parameter
#[must_use]
pub fn lmsr_cost(q: &[Decimal], b: Decimal) -> Decimal {
    if q.is_empty() || b == Decimal::ZERO {
        return Decimal::ZERO;
    }

    let b_f64 = b.to_f64().unwrap_or(1.0);

    let sum_exp: f64 = q
        .iter()
        .map(|qi| {
            let qi_f64 = qi.to_f64().unwrap_or(0.0);
            (qi_f64 / b_f64).exp()
        })
        .sum();

    let cost = b_f64 * sum_exp.ln();
    Decimal::try_from(cost).unwrap_or(Decimal::ZERO)
}

/// Compute LMSR prices from quantities.
///
/// Pᵢ = exp(qᵢ/b) / Σₖ exp(qₖ/b)
#[must_use]
pub fn lmsr_prices(q: &[Decimal], b: Decimal) -> Vec<Decimal> {
    if q.is_empty() || b == Decimal::ZERO {
        return vec![];
    }

    let b_f64 = b.to_f64().unwrap_or(1.0);

    let exps: Vec<f64> = q
        .iter()
        .map(|qi| {
            let qi_f64 = qi.to_f64().unwrap_or(0.0);
            (qi_f64 / b_f64).exp()
        })
        .collect();

    let sum_exp: f64 = exps.iter().sum();

    exps.iter()
        .map(|e| Decimal::try_from(e / sum_exp).unwrap_or(Decimal::ZERO))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_bregman_divergence_same_distribution() {
        let mu = vec![dec!(0.5), dec!(0.5)];
        let theta = vec![dec!(0.5), dec!(0.5)];

        let d = bregman_divergence(&mu, &theta);
        assert!(
            d.abs() < dec!(0.001),
            "Same distribution should have ~0 divergence"
        );
    }

    #[test]
    fn test_bregman_divergence_different() {
        let mu = vec![dec!(0.7), dec!(0.3)];
        let theta = vec![dec!(0.5), dec!(0.5)];

        let d = bregman_divergence(&mu, &theta);
        assert!(
            d > Decimal::ZERO,
            "Different distributions should have positive divergence"
        );
    }

    #[test]
    fn test_bregman_gradient_at_same_point() {
        let mu = vec![dec!(0.5), dec!(0.5)];
        let theta = vec![dec!(0.5), dec!(0.5)];

        let grad = bregman_gradient(&mu, &theta);

        // At same point, gradient should be [1, 1] (ln(1) + 1 = 1)
        for g in &grad {
            assert!(
                (*g - Decimal::ONE).abs() < dec!(0.01),
                "Gradient at same point should be ~1"
            );
        }
    }

    #[test]
    fn test_lmsr_prices_sum_to_one() {
        let q = vec![dec!(1), dec!(2), dec!(3)];
        let b = dec!(1);

        let prices = lmsr_prices(&q, b);
        let sum: Decimal = prices.iter().sum();

        assert!(
            (sum - Decimal::ONE).abs() < dec!(0.001),
            "LMSR prices should sum to 1, got {}",
            sum
        );
    }

    #[test]
    fn test_lmsr_prices_equal_quantities() {
        let q = vec![dec!(1), dec!(1), dec!(1)];
        let b = dec!(1);

        let prices = lmsr_prices(&q, b);

        // Equal quantities should give equal prices
        let expected = Decimal::ONE / Decimal::from(3);
        for p in &prices {
            assert!(
                (*p - expected).abs() < dec!(0.01),
                "Equal quantities should give ~1/3 prices"
            );
        }
    }

    #[test]
    fn test_lmsr_cost_basic() {
        let q = vec![dec!(0), dec!(0)];
        let b = dec!(1);

        let cost = lmsr_cost(&q, b);

        // C([0,0]) = b * ln(2) ≈ 0.693
        assert!(
            cost > dec!(0.6) && cost < dec!(0.8),
            "LMSR cost for [0,0] should be ~ln(2)"
        );
    }

    #[test]
    fn test_lmsr_empty_inputs() {
        assert_eq!(lmsr_cost(&[], dec!(1)), Decimal::ZERO);
        assert_eq!(lmsr_prices(&[], dec!(1)), Vec::<Decimal>::new());
        assert_eq!(bregman_divergence(&[], &[]), Decimal::ZERO);
    }
}
