use base::entities::candle::CandleId;
use base::entities::{CandleBaseProperties, CandleEdgePrices};

#[derive(Debug, Clone)]
pub struct Candle {
    pub id: CandleId,
    pub base_properties: CandleBaseProperties,
    pub edge_prices: CandleEdgePrices,
}
