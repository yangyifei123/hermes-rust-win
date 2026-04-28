//! Smart Model Router
//!
//! Selects the best AI model based on query requirements and a routing
//! strategy (cheapest, most-capable, or balanced cost/quality tradeoff).

use crate::model_metadata::{all_models, ModelMetadata};

// =============================================================================
// Routing Strategy
// =============================================================================

/// Routing strategy for model selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingStrategy {
    /// Pick the cheapest model that supports required capabilities
    Cheapest,
    /// Pick the most capable model (highest context length)
    MostCapable,
    /// Pick based on balanced cost/quality tradeoff
    Balanced,
}

// =============================================================================
// Query Requirements
// =============================================================================

/// Required capabilities for a query.
#[derive(Debug, Clone, Default)]
pub struct QueryRequirements {
    /// Query needs vision support
    pub vision: bool,
    /// Query needs tool/function calling
    pub tools: bool,
    /// Query needs streaming
    pub streaming: bool,
    /// Maximum cost budget in USD per million tokens (combined input+output price). None = unlimited.
    pub max_cost_per_million: Option<f64>,
    /// Minimum context length required
    pub min_context_length: Option<u32>,
}

// =============================================================================
// Selection
// =============================================================================

/// Select the best model given requirements and strategy.
///
/// Returns the model name string (e.g. `"gpt-4o"`, `"claude-sonnet-4-20250514"`),
/// or `None` when no model satisfies every requirement.
pub fn select_model(
    requirements: &QueryRequirements,
    strategy: RoutingStrategy,
) -> Option<String> {
    let candidates: Vec<&ModelMetadata> = all_models()
        .iter()
        .filter(|m| matches_requirements(m, requirements))
        .collect();

    if candidates.is_empty() {
        return None;
    }

    let best = match strategy {
        RoutingStrategy::Cheapest => candidates
            .into_iter()
            .min_by(|a, b| total_price(a).total_cmp(&total_price(b)))?,
        RoutingStrategy::MostCapable => candidates
            .into_iter()
            .max_by(|a, b| a.context_length.cmp(&b.context_length))?,
        RoutingStrategy::Balanced => candidates
            .into_iter()
            .max_by(|a, b| {
                balance_score(a).total_cmp(&balance_score(b))
            })?,
    };

    Some(best.name.to_string())
}

// =============================================================================
// Helpers
// =============================================================================

/// Check whether a model satisfies every requirement.
fn matches_requirements(m: &ModelMetadata, req: &QueryRequirements) -> bool {
    if req.vision && !m.supports_vision {
        return false;
    }
    if req.tools && !m.supports_tools {
        return false;
    }
    if req.streaming && !m.supports_streaming {
        return false;
    }
    if let Some(min_ctx) = req.min_context_length {
        if m.context_length < min_ctx {
            return false;
        }
    }
    if let Some(budget) = req.max_cost_per_million {
        if total_price(m) > budget {
            return false;
        }
    }
    true
}

/// Combined price per million tokens (input + output).
fn total_price(m: &ModelMetadata) -> f64 {
    m.input_price_per_million + m.output_price_per_million
}

/// Balance score: context_length / total_price.
///
/// Higher is better. Models that are free (price == 0) get a large bonus
/// so they always win the balanced ranking.
fn balance_score(m: &ModelMetadata) -> f64 {
    let price = total_price(m);
    if price == 0.0 {
        // Free models: give them a very large but finite score so they
        // are still ordered by context_length among themselves.
        return f64::from(m.context_length) + 1_000_000.0;
    }
    f64::from(m.context_length) / price
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_cheapest_model_with_tools() {
        let req = QueryRequirements {
            tools: true,
            streaming: true,
            ..Default::default()
        };
        let model = select_model(&req, RoutingStrategy::Cheapest).expect("should find a model");
        // The cheapest tool-capable, streaming-capable model should be one of
        // the free Ollama models or a very cheap provider model.
        let meta = crate::model_metadata::get_model_metadata(&model).unwrap();
        assert!(meta.supports_tools);
        assert!(meta.supports_streaming);
        // Verify it is actually the cheapest by checking no cheaper candidate exists.
        let cheapest_price = meta.input_price_per_million + meta.output_price_per_million;
        for m in all_models().iter() {
            if m.supports_tools && m.supports_streaming {
                assert!(
                    cheapest_price <= m.input_price_per_million + m.output_price_per_million,
                    "{} ({}) should not be cheaper than {} ({})",
                    m.name,
                    m.input_price_per_million + m.output_price_per_million,
                    meta.name,
                    cheapest_price,
                );
            }
        }
    }

    #[test]
    fn test_select_most_capable() {
        let req = QueryRequirements {
            streaming: true,
            ..Default::default()
        };
        let model = select_model(&req, RoutingStrategy::MostCapable).expect("should find a model");
        let meta = crate::model_metadata::get_model_metadata(&model).unwrap();
        // gemini-1.5-pro has the largest context at 2_097_152
        assert_eq!(meta.context_length, 2_097_152);
    }

    #[test]
    fn test_no_model_with_vision_if_not_needed() {
        // Request vision=false, tools=true — should never return a model
        // that lacks tools, but vision flag is not a filter here.
        let req = QueryRequirements {
            vision: false,
            tools: true,
            ..Default::default()
        };
        let model = select_model(&req, RoutingStrategy::Cheapest).expect("should find a model");
        let meta = crate::model_metadata::get_model_metadata(&model).unwrap();
        // Must support tools; vision flag was false so we don't require it,
        // but the model may or may not have vision.
        assert!(meta.supports_tools);
    }

    #[test]
    fn test_vision_filter() {
        // Require vision — every returned model must support it.
        let req = QueryRequirements {
            vision: true,
            ..Default::default()
        };
        let model = select_model(&req, RoutingStrategy::Cheapest).expect("should find a model");
        let meta = crate::model_metadata::get_model_metadata(&model).unwrap();
        assert!(meta.supports_vision);
    }

    #[test]
    fn test_cost_budget_filter() {
        // Budget so low only free models qualify.
        let req = QueryRequirements {
            max_cost_per_million: Some(0.01),
            ..Default::default()
        };
        let model = select_model(&req, RoutingStrategy::Cheapest);
        // There should be free models that qualify.
        assert!(model.is_some(), "should find a free model");
        let meta = crate::model_metadata::get_model_metadata(&model.unwrap()).unwrap();
        assert_eq!(meta.input_price_per_million + meta.output_price_per_million, 0.0);

        // Budget so tight even free models are excluded by other constraints.
        let req = QueryRequirements {
            max_cost_per_million: Some(0.01),
            vision: true,
            tools: true,
            streaming: true,
            ..Default::default()
        };
        let model = select_model(&req, RoutingStrategy::Cheapest);
        // No free model has vision + tools + streaming — expect None.
        assert!(model.is_none(), "no free model should have vision + tools + streaming");
    }

    #[test]
    fn test_min_context_length() {
        let req = QueryRequirements {
            min_context_length: Some(500_000),
            ..Default::default()
        };
        let model = select_model(&req, RoutingStrategy::Cheapest).expect("should find a model");
        let meta = crate::model_metadata::get_model_metadata(&model).unwrap();
        assert!(meta.context_length >= 500_000);
    }

    #[test]
    fn test_impossible_requirements() {
        // Require vision, tools, and an absurdly large context that no model has.
        let req = QueryRequirements {
            vision: true,
            tools: true,
            min_context_length: Some(100_000_000),
            ..Default::default()
        };
        assert!(select_model(&req, RoutingStrategy::Cheapest).is_none());
    }

    #[test]
    fn test_balanced_strategy() {
        let req = QueryRequirements {
            tools: true,
            streaming: true,
            ..Default::default()
        };
        let model =
            select_model(&req, RoutingStrategy::Balanced).expect("should find a model");
        let meta = crate::model_metadata::get_model_metadata(&model).unwrap();
        assert!(meta.supports_tools);
        assert!(meta.supports_streaming);
        // The balanced pick should be a high-context, low-price model.
        // We just verify it's a valid pick; the exact choice depends on
        // registry data.
        assert!(meta.context_length > 0);
    }
}
