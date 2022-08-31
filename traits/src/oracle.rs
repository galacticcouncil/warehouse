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

/// Defines the different kinds of aggregation periods for oracles.
#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, TypeInfo)]
pub enum OraclePeriod {
    /// The oracle data is from the last block, thus unaggregated.
    LastBlock,
    /// The oracle data was aggregated over the blocks of the last ten minutes.
    TenMinutes,
    /// The oracle data was aggregated over the blocks of the last day.
    Day,
    /// The oracle data was aggregated over the blocks of the last week.
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

/// Struct to represent oracle data aggregated over a time period. Includes the age of the oracle
/// as metadata. Age is the blocks between first data and the timestamp of the most recent value.
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

/// Struct to represent trade volume for an asset pair.
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
    /// Constructor for volume flowing based on trades from asset a to asset b.
    pub fn from_a_in_b_out(a_in: Balance, b_out: Balance) -> Self {
        Self {
            a_in,
            b_out,
            a_out: Zero::zero(),
            b_in: Zero::zero(),
        }
    }

    /// Constructor for volume flowing based on trades from asset b to asset a.
    pub fn from_a_out_b_in(a_out: Balance, b_in: Balance) -> Self {
        Self {
            a_in: Zero::zero(),
            b_out: Zero::zero(),
            a_out,
            b_in,
        }
    }

    /// Utility function that sums the underlying values of the volumes.
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

/// An oracle returning an entry of oracle data aggregated over `period`.
pub trait AggregatedOracle<AssetId, Balance, BlockNumber, Price> {
    type Error;
    fn get_entry(
        asset_a: AssetId,
        asset_b: AssetId,
        period: OraclePeriod,
    ) -> Result<AggregatedEntry<Balance, BlockNumber, Price>, Self::Error>;

    fn get_entry_weight() -> Weight;
}

/// Default implementation of the oracle trait that always returns `Err`.
impl<AssetId, Balance, BlockNumber, Price> AggregatedOracle<AssetId, Balance, BlockNumber, Price> for () {
    type Error = ();

    fn get_entry(
        _asset_a: AssetId,
        _asset_b: AssetId,
        _period: OraclePeriod,
    ) -> Result<AggregatedEntry<Balance, BlockNumber, Price>, Self::Error> {
        Err(())
    }

    fn get_entry_weight() -> Weight {
        Weight::zero()
    }
}

/// An oracle returning a price aggregated over `period` with the associated oracle age (to allow
/// judging whether the oracle had a chance to settle yet).
pub trait AggregatedPriceOracle<AssetId, BlockNumber, Price> {
    type Error;
    fn get_price(asset_a: AssetId, asset_b: AssetId, period: OraclePeriod)
        -> Result<(Price, BlockNumber), Self::Error>;

    fn get_price_weight() -> Weight;
}

/// Default implementation of the oracle trait that always returns `Err`.
impl<AssetId, BlockNumber, Price> AggregatedPriceOracle<AssetId, BlockNumber, Price> for () {
    type Error = ();

    fn get_price(
        _asset_a: AssetId,
        _asset_b: AssetId,
        _period: OraclePeriod,
    ) -> Result<(Price, BlockNumber), Self::Error> {
        Err(())
    }

    fn get_price_weight() -> Weight {
        Weight::zero()
    }
}

/// Mock implementation of the oracle trait that always returns `Price::one()` and oracle age of
/// `BlockNumber::one()`.
impl<AssetId, BlockNumber, Price> AggregatedPriceOracle<AssetId, BlockNumber, Price> for AlwaysPriceOfOne
where
    Price: One,
    BlockNumber: One,
{
    type Error = ();

    fn get_price(
        _asset_a: AssetId,
        _asset_b: AssetId,
        _period: OraclePeriod,
    ) -> Result<(Price, BlockNumber), Self::Error> {
        Ok((Price::one(), BlockNumber::one()))
    }

    fn get_price_weight() -> Weight {
        Weight::zero()
    }
}
