//! Scaling recommendation types for adaptive subscription management.
//!
//! These types represent scaling decisions made by the AdaptiveGovernor to signal
//! whether the system should expand, hold, or contract its subscription count.
//!
//! - [`ScalingRecommendation`] - Enum representing a scaling decision

/// A scaling recommendation from the AdaptiveGovernor.
///
/// Represents the governor's decision about how to adjust the number of
/// active subscriptions based on current resource utilization and market scores.
///
/// # Variants
///
/// - `Expand` - Increase subscriptions to the suggested count
/// - `Hold` - Maintain current subscription count
/// - `Contract` - Decrease subscriptions to the suggested count
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalingRecommendation {
    /// Expand subscriptions to the suggested count.
    Expand {
        /// The suggested number of subscriptions to expand to.
        suggested_count: usize,
    },
    /// Maintain current subscription count.
    Hold,
    /// Contract subscriptions to the suggested count.
    Contract {
        /// The suggested number of subscriptions to contract to.
        suggested_count: usize,
    },
}

impl ScalingRecommendation {
    /// Create an expand recommendation with the given count.
    ///
    /// # Arguments
    ///
    /// * `count` - The suggested number of subscriptions to expand to
    #[must_use]
    pub const fn expand(count: usize) -> Self {
        Self::Expand {
            suggested_count: count,
        }
    }

    /// Create a contract recommendation with the given count.
    ///
    /// # Arguments
    ///
    /// * `count` - The suggested number of subscriptions to contract to
    #[must_use]
    pub const fn contract(count: usize) -> Self {
        Self::Contract {
            suggested_count: count,
        }
    }

    /// Returns `true` if this is an expand recommendation.
    #[must_use]
    pub const fn is_expand(&self) -> bool {
        matches!(self, Self::Expand { .. })
    }

    /// Returns `true` if this is a hold recommendation.
    #[must_use]
    pub const fn is_hold(&self) -> bool {
        matches!(self, Self::Hold)
    }

    /// Returns `true` if this is a contract recommendation.
    #[must_use]
    pub const fn is_contract(&self) -> bool {
        matches!(self, Self::Contract { .. })
    }

    /// Returns the suggested subscription count, if any.
    ///
    /// Returns `Some(count)` for `Expand` and `Contract` variants,
    /// and `None` for `Hold`.
    #[must_use]
    pub const fn suggested_count(&self) -> Option<usize> {
        match self {
            Self::Expand { suggested_count } | Self::Contract { suggested_count } => {
                Some(*suggested_count)
            }
            Self::Hold => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- ScalingRecommendation::expand tests ---

    #[test]
    fn scaling_recommendation_expand_creates_expand_variant() {
        let rec = ScalingRecommendation::expand(100);

        assert!(matches!(
            rec,
            ScalingRecommendation::Expand {
                suggested_count: 100
            }
        ));
    }

    #[test]
    fn scaling_recommendation_expand_with_zero() {
        let rec = ScalingRecommendation::expand(0);

        assert!(matches!(
            rec,
            ScalingRecommendation::Expand { suggested_count: 0 }
        ));
    }

    // --- ScalingRecommendation::contract tests ---

    #[test]
    fn scaling_recommendation_contract_creates_contract_variant() {
        let rec = ScalingRecommendation::contract(50);

        assert!(matches!(
            rec,
            ScalingRecommendation::Contract {
                suggested_count: 50
            }
        ));
    }

    #[test]
    fn scaling_recommendation_contract_with_zero() {
        let rec = ScalingRecommendation::contract(0);

        assert!(matches!(
            rec,
            ScalingRecommendation::Contract { suggested_count: 0 }
        ));
    }

    // --- ScalingRecommendation::is_expand tests ---

    #[test]
    fn scaling_recommendation_is_expand_returns_true_for_expand() {
        let rec = ScalingRecommendation::expand(100);

        assert!(rec.is_expand());
    }

    #[test]
    fn scaling_recommendation_is_expand_returns_false_for_hold() {
        let rec = ScalingRecommendation::Hold;

        assert!(!rec.is_expand());
    }

    #[test]
    fn scaling_recommendation_is_expand_returns_false_for_contract() {
        let rec = ScalingRecommendation::contract(50);

        assert!(!rec.is_expand());
    }

    // --- ScalingRecommendation::is_hold tests ---

    #[test]
    fn scaling_recommendation_is_hold_returns_true_for_hold() {
        let rec = ScalingRecommendation::Hold;

        assert!(rec.is_hold());
    }

    #[test]
    fn scaling_recommendation_is_hold_returns_false_for_expand() {
        let rec = ScalingRecommendation::expand(100);

        assert!(!rec.is_hold());
    }

    #[test]
    fn scaling_recommendation_is_hold_returns_false_for_contract() {
        let rec = ScalingRecommendation::contract(50);

        assert!(!rec.is_hold());
    }

    // --- ScalingRecommendation::is_contract tests ---

    #[test]
    fn scaling_recommendation_is_contract_returns_true_for_contract() {
        let rec = ScalingRecommendation::contract(50);

        assert!(rec.is_contract());
    }

    #[test]
    fn scaling_recommendation_is_contract_returns_false_for_expand() {
        let rec = ScalingRecommendation::expand(100);

        assert!(!rec.is_contract());
    }

    #[test]
    fn scaling_recommendation_is_contract_returns_false_for_hold() {
        let rec = ScalingRecommendation::Hold;

        assert!(!rec.is_contract());
    }

    // --- ScalingRecommendation::suggested_count tests ---

    #[test]
    fn scaling_recommendation_suggested_count_returns_some_for_expand() {
        let rec = ScalingRecommendation::expand(100);

        assert_eq!(rec.suggested_count(), Some(100));
    }

    #[test]
    fn scaling_recommendation_suggested_count_returns_some_for_contract() {
        let rec = ScalingRecommendation::contract(50);

        assert_eq!(rec.suggested_count(), Some(50));
    }

    #[test]
    fn scaling_recommendation_suggested_count_returns_none_for_hold() {
        let rec = ScalingRecommendation::Hold;

        assert_eq!(rec.suggested_count(), None);
    }

    // --- Equality tests ---

    #[test]
    fn scaling_recommendation_expand_equality() {
        let a = ScalingRecommendation::expand(100);
        let b = ScalingRecommendation::expand(100);
        let c = ScalingRecommendation::expand(200);

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn scaling_recommendation_contract_equality() {
        let a = ScalingRecommendation::contract(50);
        let b = ScalingRecommendation::contract(50);
        let c = ScalingRecommendation::contract(25);

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn scaling_recommendation_hold_equality() {
        let a = ScalingRecommendation::Hold;
        let b = ScalingRecommendation::Hold;

        assert_eq!(a, b);
    }

    #[test]
    fn scaling_recommendation_different_variants_not_equal() {
        let expand = ScalingRecommendation::expand(100);
        let hold = ScalingRecommendation::Hold;
        let contract = ScalingRecommendation::contract(100);

        assert_ne!(expand, hold);
        assert_ne!(expand, contract);
        assert_ne!(hold, contract);
    }

    // --- Clone and Copy tests ---

    #[test]
    fn scaling_recommendation_is_copy() {
        let original = ScalingRecommendation::expand(100);
        let copied = original; // Copy, not move
        let _also_original = original; // Can still use original

        assert_eq!(copied, original);
    }

    #[test]
    fn scaling_recommendation_clone() {
        let original = ScalingRecommendation::contract(50);
        let cloned = original;

        assert_eq!(cloned, original);
    }
}
