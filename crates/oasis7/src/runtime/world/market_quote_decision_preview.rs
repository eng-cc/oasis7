use serde::{Deserialize, Serialize};

use super::super::{MaterialLedgerId, MaterialMarketQuote, MaterialStack};
use super::World;
use super::event_processing::action_to_event_economy::build_material_market_quotes;

/// A conditional, read-only explanation of the material market inputs that a
/// recipe submission would use at the current world state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketQuoteDecisionPreview {
    pub market_quotes: Vec<MaterialMarketQuote>,
    pub local_vs_world_delta: Vec<MarketQuoteSupplyDelta>,
    pub total_unsatisfied_shortfall: i64,
    pub market_pressure: String,
    pub recommendation: String,
    pub rationale: String,
    pub next_reduction_action: String,
    pub conditional: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketQuoteSupplyDelta {
    pub kind: String,
    pub local_deficit_amount: i64,
    pub world_cover_amount: i64,
    pub unsatisfied_shortfall_amount: i64,
}

impl World {
    /// Derives the existing market quote formula without reserving inventory or
    /// submitting an action. The preview is conditional on current balances and
    /// policy at eventual submission time.
    pub fn market_quote_decision_preview(
        &self,
        preferred_consume_ledger: &MaterialLedgerId,
        consume: &[MaterialStack],
    ) -> MarketQuoteDecisionPreview {
        let market_quotes = build_material_market_quotes(self, preferred_consume_ledger, consume);
        let mut total_unsatisfied_shortfall = 0_i64;
        let local_vs_world_delta = market_quotes
            .iter()
            .map(|quote| {
                let world_cover_amount =
                    quote.world_available_amount.min(quote.local_deficit_amount);
                let unsatisfied_shortfall_amount = quote
                    .local_deficit_amount
                    .saturating_sub(world_cover_amount);
                total_unsatisfied_shortfall =
                    total_unsatisfied_shortfall.saturating_add(unsatisfied_shortfall_amount);
                MarketQuoteSupplyDelta {
                    kind: quote.kind.clone(),
                    local_deficit_amount: quote.local_deficit_amount,
                    world_cover_amount,
                    unsatisfied_shortfall_amount,
                }
            })
            .collect::<Vec<_>>();
        let has_local_deficit = market_quotes
            .iter()
            .any(|quote| quote.local_deficit_amount > 0);
        let (market_pressure, recommendation, rationale, next_reduction_action) =
            if total_unsatisfied_shortfall > 0 {
                (
                    "unsatisfied_shortfall",
                    "reduce_or_source_materials",
                    "world inventory cannot cover the local material deficit",
                    "reduce_requested_amount",
                )
            } else if has_local_deficit {
                (
                    "world_supply_pressure",
                    "submit_with_world_supply",
                    "world supply covers the local deficit; tax and transit remain conditional",
                    "use_local_materials",
                )
            } else {
                (
                    "local_supply_ready",
                    "submit_with_local_supply",
                    "local inventory covers the requested materials",
                    "submit_recipe",
                )
            };

        MarketQuoteDecisionPreview {
            market_quotes,
            local_vs_world_delta,
            total_unsatisfied_shortfall,
            market_pressure: market_pressure.to_string(),
            recommendation: recommendation.to_string(),
            rationale: rationale.to_string(),
            next_reduction_action: next_reduction_action.to_string(),
            conditional: true,
        }
    }
}
