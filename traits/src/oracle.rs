use super::*;

use scale_info::TypeInfo;

/// Implementers of this trait provide the price of a given asset compared to the native currency.
///
/// So if `100` native tokens correspond to `200` ABC tokens, the price returned would be `2.0`.
///
/// Should return `None` if no price is available.
pub trait NativePriceOracle<AssetId, Price> {
    fn price(currency: AssetId) -> Option<Price>;
}

impl<AssetId, Price> NativePriceOracle<AssetId, Price> for () {
    fn price(_currency: AssetId) -> Option<Price> {
        None
    }
}

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, TypeInfo)]
pub enum OraclePeriod {
    LastBlock,
    TenMinutes,
    Day,
    Week,
}

impl OraclePeriod {
    pub fn all_periods() -> &'static [OraclePeriod] {
        use OraclePeriod::*;
        &[LastBlock, TenMinutes, Day, Week]
    }

    pub fn non_immediate_periods() -> &'static [OraclePeriod] {
        use OraclePeriod::*;
        &[TenMinutes, Day, Week]
    }
}

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, TypeInfo)]
pub struct AggregatedEntry<Balance, Price> {
    pub price: Price,
    pub volume: Balance,
    pub liquidity: Balance,
}

impl<Balance, Price> From<(Price, Balance, Balance)> for AggregatedEntry<Balance, Price> {
    fn from((price, volume, liquidity): (Price, Balance, Balance)) -> Self {
        Self {
            price,
            volume,
            liquidity,
        }
    }
}

pub trait AggregatedOracle<AssetId, Balance, Price> {
    type Error;
    fn get_entry(
        asset_a: AssetId,
        asset_b: AssetId,
        period: OraclePeriod,
    ) -> (Result<AggregatedEntry<Balance, Price>, Self::Error>, Weight);

    fn get_entry_weight() -> Weight;
}

impl<AssetId, Balance, Price> AggregatedOracle<AssetId, Balance, Price> for () {
    type Error = ();

    fn get_entry(
        _asset_a: AssetId,
        _asset_b: AssetId,
        _period: OraclePeriod,
    ) -> (Result<AggregatedEntry<Balance, Price>, Self::Error>, Weight) {
        (Err(()), Weight::zero())
    }

    fn get_entry_weight() -> Weight {
        Weight::zero()
    }
}

pub trait AggregatedPriceOracle<AssetId, Price> {
    type Error;
    fn get_price(asset_a: AssetId, asset_b: AssetId, period: OraclePeriod) -> (Result<Price, Self::Error>, Weight);

    fn get_price_weight() -> Weight;
}

impl<AssetId, Price> AggregatedPriceOracle<AssetId, Price> for () {
    type Error = ();

    fn get_price(_asset_a: AssetId, _asset_b: AssetId, _period: OraclePeriod) -> (Result<Price, Self::Error>, Weight) {
        (Err(()), Weight::zero())
    }

    fn get_price_weight() -> Weight {
        Weight::zero()
    }
}
