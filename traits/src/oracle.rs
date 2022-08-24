use super::*;

use frame_support::sp_runtime::traits::{AtLeast32BitUnsigned, One};
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

pub struct AlwaysPriceOfOne;
impl<AssetId, Price> NativePriceOracle<AssetId, Price> for AlwaysPriceOfOne
where
    Price: One,
{
    fn price(_currency: AssetId) -> Option<Price> {
        Some(Price::one())
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

#[derive(Encode, Decode, Eq, PartialEq, Clone, Default, RuntimeDebug, TypeInfo)]
pub struct AggregatedEntry<Balance, BlockNumber, Price> {
    pub price: Price,
    pub volume: Volume<Balance>,
    pub liquidity: Balance,
    pub oracle_age: BlockNumber,
}

impl<Balance, BlockNumber, Price> From<(Price, Volume<Balance>, Balance, BlockNumber)>
    for AggregatedEntry<Balance, BlockNumber, Price>
{
    fn from((price, volume, liquidity, oracle_age): (Price, Volume<Balance>, Balance, BlockNumber)) -> Self {
        Self {
            price,
            volume,
            liquidity,
            oracle_age,
        }
    }
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(RuntimeDebug, Encode, Decode, Clone, PartialEq, Eq, Default, TypeInfo)]
pub struct Volume<Balance> {
    pub a_in: Balance,
    pub b_out: Balance,
    pub a_out: Balance,
    pub b_in: Balance,
}

impl<Balance> Volume<Balance>
where
    Balance: Copy + AtLeast32BitUnsigned,
{
    pub fn from_a_in_b_out(a_in: Balance, b_out: Balance) -> Self {
        Self {
            a_in,
            b_out,
            a_out: Zero::zero(),
            b_in: Zero::zero(),
        }
    }

    pub fn from_a_out_b_in(a_out: Balance, b_in: Balance) -> Self {
        Self {
            a_in: Zero::zero(),
            b_out: Zero::zero(),
            a_out,
            b_in,
        }
    }

    pub fn saturating_add(&self, rhs: &Self) -> Self {
        let Self {
            a_in: r_a_in,
            b_out: r_b_out,
            a_out: r_a_out,
            b_in: r_b_in,
        } = rhs;
        let Self {
            a_in,
            b_out,
            a_out,
            b_in,
        } = self;
        Self {
            a_in: a_in.saturating_add(*r_a_in),
            b_out: b_out.saturating_add(*r_b_out),
            a_out: a_out.saturating_add(*r_a_out),
            b_in: b_in.saturating_add(*r_b_in),
        }
    }

    /// Returns the cumulative volume as `(cumulative_a, cumulative_b)`.
    pub fn cumulative_volume(&self) -> (Balance, Balance) {
        (
            self.a_in.saturating_add(self.a_out),
            (self.b_out).saturating_add(self.b_in),
        )
    }
}

pub trait AggregatedOracle<AssetId, Balance, BlockNumber, Price> {
    type Error;
    fn get_entry(
        asset_a: AssetId,
        asset_b: AssetId,
        period: OraclePeriod,
    ) -> (
        Result<AggregatedEntry<Balance, BlockNumber, Price>, Self::Error>,
        Weight,
    );

    fn get_entry_weight() -> Weight;
}

impl<AssetId, Balance, BlockNumber, Price> AggregatedOracle<AssetId, Balance, BlockNumber, Price> for () {
    type Error = ();

    fn get_entry(
        _asset_a: AssetId,
        _asset_b: AssetId,
        _period: OraclePeriod,
    ) -> (
        Result<AggregatedEntry<Balance, BlockNumber, Price>, Self::Error>,
        Weight,
    ) {
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

impl<AssetId, Price> AggregatedPriceOracle<AssetId, Price> for AlwaysPriceOfOne
where
    Price: One,
{
    type Error = ();

    fn get_price(_asset_a: AssetId, _asset_b: AssetId, _period: OraclePeriod) -> (Result<Price, Self::Error>, Weight) {
        (Ok(Price::one()), Weight::zero())
    }

    fn get_price_weight() -> Weight {
        Weight::zero()
    }
}
