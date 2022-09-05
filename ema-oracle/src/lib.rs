// This file is part of pallet-ema-oracle.

// Copyright (C) 2022  Intergalactic, Limited (GIB).
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet_prelude::*;
use frame_support::sp_runtime::traits::{BlockNumberProvider, One, Zero};
use frame_support::sp_runtime::FixedPointNumber;
use hydradx_traits::{
    AggregatedEntry, AggregatedOracle, AggregatedPriceOracle, OnCreatePoolHandler, OnLiquidityChangedHandler,
    OnTradeHandler,
    OraclePeriod::{self, *},
    Volume,
};
use sp_arithmetic::traits::Saturating;
use sp_std::collections::btree_map::BTreeMap;
use sp_std::marker::PhantomData;
use sp_std::prelude::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod types;
pub use types::*;

#[allow(clippy::all)]
pub mod weights;
use weights::WeightInfo;

mod benchmarking;

/// Maximum number of trades expected in one block. Empirically determined by running
/// `trades_estimation.py` and rounding up from 212 to 300.
pub const MAX_TRADES: u32 = 300;

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

/// Unique identifier for an asset pair.
/// AMM pools derive their own unique identifiers for asset pairs,
/// but this one is meant to not be bounded to one particular AMM pool.
pub type AssetPairId = Vec<u8>;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Weight information for the extrinsics.
        type WeightInfo: WeightInfo;

        type BlockNumberProvider: BlockNumberProvider<BlockNumber = Self::BlockNumber>;

        /// Number of seconds between blocks, used to convert periods.
        type SecsPerBlock: Get<Self::BlockNumber>;
    }

    #[pallet::error]
    pub enum Error<T> {}

    #[pallet::event]
    pub enum Event<T: Config> {}

    #[pallet::storage]
    #[pallet::getter(fn accumulator)]
    pub type Accumulator<T: Config> = StorageValue<_, BTreeMap<AssetPairId, OracleEntry<T::BlockNumber>>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn oracle)]
    pub type Oracles<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        AssetPairId,
        Twox64Concat,
        T::BlockNumber,
        (OracleEntry<T::BlockNumber>, T::BlockNumber),
        OptionQuery,
    >;

    #[pallet::genesis_config]
    #[derive(Default)]
    pub struct GenesisConfig {
        pub price_data: Vec<((AssetId, AssetId), Price, Balance)>,
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig {
        fn build(&self) {
            for &(asset_pair, price, liquidity) in self.price_data.iter() {
                let pair_id = derive_name(asset_pair.0, asset_pair.1);

                let entry: OracleEntry<T::BlockNumber> = OracleEntry {
                    price,
                    volume: Volume::default(),
                    liquidity,
                    timestamp: T::BlockNumber::zero(),
                };
                for period in OraclePeriod::all_periods() {
                    Pallet::<T>::update_oracle(&pair_id, into_blocks::<T>(period), entry.clone());
                }
            }
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
        fn on_initialize(_n: T::BlockNumber) -> Weight {
            T::WeightInfo::on_finalize_no_entry()
        }

        fn on_finalize(_n: T::BlockNumber) {
            // update oracles based on data accumulated during the block
            Self::update_oracles_from_accumulator();
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {}
}

impl<T: Config> Pallet<T> {
    /// Insert or update data in the accumulator from received price entry. Aggregates volume and
    /// takes the most recent data for the rest.
    pub(crate) fn on_entry(pair_id: AssetPairId, oracle_entry: OracleEntry<T::BlockNumber>) {
        Accumulator::<T>::mutate(|accumulator| {
            accumulator
                .entry(pair_id)
                .and_modify(|entry| {
                    *entry = oracle_entry.accumulate_volume(entry);
                })
                .or_insert(oracle_entry);
        });
    }

    pub(crate) fn on_trade(pair_id: AssetPairId, oracle_entry: OracleEntry<T::BlockNumber>) {
        Self::on_entry(pair_id, oracle_entry)
    }

    pub(crate) fn on_liquidity_changed(pair_id: AssetPairId, oracle_entry: OracleEntry<T::BlockNumber>) {
        Self::on_entry(pair_id, oracle_entry)
    }

    fn update_oracles_from_accumulator() {
        // update oracles based on data accumulated during the block
        for (pair_id, oracle_entry) in Accumulator::<T>::take().into_iter() {
            for period in OraclePeriod::all_periods() {
                Self::update_oracle(&pair_id, into_blocks::<T>(period), oracle_entry.clone());
            }
        }
    }

    fn update_oracle(pair_id: &AssetPairId, period: T::BlockNumber, oracle_entry: OracleEntry<T::BlockNumber>) {
        Oracles::<T>::mutate(pair_id, period, |oracle| {
            let new_oracle = oracle
                .as_ref()
                .map(|(prev_entry, init)| {
                    (
                        oracle_entry
                            .calculate_new_ema_entry(period, prev_entry)
                            .unwrap_or_else(|| prev_entry.clone()),
                        *init,
                    )
                })
                .unwrap_or((oracle_entry.clone(), T::BlockNumberProvider::current_block_number()));
            *oracle = Some(new_oracle);
        });
    }

    fn get_updated_entry(
        pair_id: &AssetPairId,
        period: OraclePeriod,
    ) -> Option<(OracleEntry<T::BlockNumber>, T::BlockNumber)> {
        let parent = T::BlockNumberProvider::current_block_number().saturating_sub(One::one());
        // First get the `LastBlock` oracle as we will use it to calculate the updated values for
        // the others.
        let (mut last_block, init) = Self::oracle(pair_id, into_blocks::<T>(&LastBlock))?;
        last_block.timestamp = parent;

        if period == LastBlock {
            return Some((last_block, init));
        }
        let last_block = last_block;

        let (entry, init) = Self::oracle(pair_id, into_blocks::<T>(&period))?;
        if entry.timestamp < parent {
            last_block.calculate_new_ema_entry(into_blocks::<T>(&period), &entry)
        } else {
            Some(entry)
        }
        .map(|return_entry| (return_entry, init))
    }
}

/// A callback handler for trading and liquidity activity that schedules oracle updates.
pub struct OnActivityHandler<T>(PhantomData<T>);

impl<T: Config> OnCreatePoolHandler<AssetId> for OnActivityHandler<T> {
    // Nothing to do on pool creation. Oracles are created lazily.
    fn on_create_pool(_asset_a: AssetId, _asset_b: AssetId) -> DispatchResult {
        Ok(())
    }
}

impl<T: Config> OnTradeHandler<AssetId, Balance> for OnActivityHandler<T> {
    fn on_trade(asset_in: AssetId, asset_out: AssetId, amount_in: Balance, amount_out: Balance, liquidity: Balance) {
        // We assume that zero values are not valid and can be ignored.
        if liquidity.is_zero() {
            return;
        }
        // We don't want to throw an error here because this method is used in different extrinsics.
        let price = determine_normalized_price(asset_in, asset_out, amount_in, amount_out).unwrap_or_else(Zero::zero);
        // We assume that zero values are not valid and are ignored.
        if price.is_zero() || amount_in.is_zero() || amount_out.is_zero() {
            return;
        }

        let volume = determine_normalized_volume(asset_in, asset_out, amount_in, amount_out);

        let timestamp = T::BlockNumberProvider::current_block_number();
        let entry = OracleEntry {
            price,
            volume,
            liquidity,
            timestamp,
        };
        Pallet::<T>::on_trade(derive_name(asset_in, asset_out), entry);
    }

    fn on_trade_weight() -> Weight {
        T::WeightInfo::on_trade_multiple_tokens(MAX_TRADES).saturating_add(
            T::WeightInfo::on_finalize_multiple_tokens(MAX_TRADES)
                .saturating_sub(T::WeightInfo::on_finalize_no_entry())
                / Weight::from(MAX_TRADES),
        )
    }
}

impl<T: Config> OnLiquidityChangedHandler<AssetId, Balance> for OnActivityHandler<T> {
    fn on_liquidity_changed(
        asset_a: AssetId,
        asset_b: AssetId,
        amount_a: Balance,
        amount_b: Balance,
        liquidity: Balance,
    ) {
        // We assume that zero values are not valid and can be ignored.
        if liquidity.is_zero() {
            return;
        }
        // We don't want to throw an error here because this method is used in different extrinsics.
        let price = determine_normalized_price(asset_a, asset_b, amount_a, amount_b).unwrap_or_else(Zero::zero);
        // We assume that zero values are not valid and are ignored.
        if price.is_zero() {
            return;
        }

        let timestamp = T::BlockNumberProvider::current_block_number();
        let entry = OracleEntry {
            price,
            // liquidity provision does not count as trade volume
            volume: Volume::default(),
            liquidity,
            timestamp,
        };
        Pallet::<T>::on_liquidity_changed(derive_name(asset_a, asset_b), entry);
    }

    fn on_liquidity_changed_weight() -> Weight {
        T::WeightInfo::on_liquidity_changed_multiple_tokens(MAX_TRADES).saturating_add(
            T::WeightInfo::on_finalize_multiple_tokens(MAX_TRADES)
                .saturating_sub(T::WeightInfo::on_finalize_no_entry())
                / Weight::from(MAX_TRADES),
        )
    }
}

// TODO: extract
/// Calculate price from ordered assets
pub fn determine_normalized_price(
    asset_in: AssetId,
    asset_out: AssetId,
    amount_in: Balance,
    amount_out: Balance,
) -> Option<Price> {
    let ordered_asset_pair = ordered_pair(asset_in, asset_out);
    let (balance_a, balance_b) = if ordered_asset_pair == (asset_in, asset_out) {
        (amount_in, amount_out)
    } else {
        (amount_out, amount_in)
    };

    Price::checked_from_rational(balance_a, balance_b)
}

pub fn determine_normalized_volume(
    asset_in: AssetId,
    asset_out: AssetId,
    amount_in: Balance,
    amount_out: Balance,
) -> Volume<Balance> {
    let ordered_asset_pair = ordered_pair(asset_in, asset_out);
    if ordered_asset_pair == (asset_in, asset_out) {
        Volume::from_a_in_b_out(amount_in, amount_out)
    } else {
        Volume::from_a_out_b_in(amount_out, amount_in)
    }
}

/// Return ordered asset tuple (A,B) where A < B
/// Used in storage
/// The implementation is the same as for AssetPair
pub fn ordered_pair(asset_a: AssetId, asset_b: AssetId) -> (AssetId, AssetId) {
    match asset_a <= asset_b {
        true => (asset_a, asset_b),
        false => (asset_b, asset_a),
    }
}

/// Return share token name
/// The implementation is the same as for AssetPair
pub fn derive_name(asset_a: AssetId, asset_b: AssetId) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();

    let (asset_left, asset_right) = ordered_pair(asset_a, asset_b);

    buf.extend_from_slice(&asset_left.to_le_bytes());
    buf.extend_from_slice(b"HDT");
    buf.extend_from_slice(&asset_right.to_le_bytes());

    buf
}

/// Convert the given `period` into a number of blocks based on `T::SecsPerBlock`.
pub fn into_blocks<T: Config>(period: &OraclePeriod) -> T::BlockNumber {
    let secs_per_block = T::SecsPerBlock::get();
    let minutes = T::BlockNumber::from(60u8) / secs_per_block;
    let days = T::BlockNumber::from(24u8) * T::BlockNumber::from(60u8) * minutes;
    match period {
        OraclePeriod::LastBlock => One::one(),
        OraclePeriod::TenMinutes => T::BlockNumber::from(10u8) * minutes,
        OraclePeriod::Day => days,
        OraclePeriod::Week => T::BlockNumber::from(7u8) * days,
    }
}

/// Possible errors when requesting an oracle value.
#[derive(RuntimeDebug, Encode, Decode, Copy, Clone, PartialEq, Eq, TypeInfo)]
pub enum OracleError {
    /// The oracle could not be found
    NotPresent,
    /// The oracle is not defined if the asset ids are the same.
    SameAsset,
    CannotInvert,
}

impl<T: Config> AggregatedOracle<AssetId, Balance, T::BlockNumber, Price> for Pallet<T> {
    type Error = OracleError;

    /// Returns the entry corresponding to the given assets and period.
    /// The entry is updated to the state of the parent block (but not trading data in the current
    /// block). It is also adjusted to make sense for the asset order given as parameters. So
    /// calling `get_entry(HDX, DOT, LastBlock)` will return the price `HDX/DOT`, while
    /// `get_entry(DOT HDX, LastBlock)` will return `DOT/HDX`.
    fn get_entry(
        asset_a: AssetId,
        asset_b: AssetId,
        period: OraclePeriod,
    ) -> Result<AggregatedEntry<Balance, T::BlockNumber, Price>, OracleError> {
        if asset_a == asset_b {
            return Err(OracleError::SameAsset);
        };
        let pair_id = derive_name(asset_a, asset_b);
        Self::get_updated_entry(&pair_id, period)
            .ok_or(OracleError::NotPresent)
            .and_then(|(entry, initialized)| {
                let entry = if (asset_a, asset_b) != ordered_pair(asset_a, asset_b) {
                    entry.inverted().ok_or(OracleError::CannotInvert)?
                } else {
                    entry
                };
                Ok(entry.into_aggregated(initialized))
            })
    }

    fn get_entry_weight() -> Weight {
        T::WeightInfo::get_entry()
    }
}

impl<T: Config> AggregatedPriceOracle<AssetId, T::BlockNumber, Price> for Pallet<T> {
    type Error = OracleError;

    fn get_price(
        asset_a: AssetId,
        asset_b: AssetId,
        period: OraclePeriod,
    ) -> Result<(Price, T::BlockNumber), Self::Error> {
        Self::get_entry(asset_a, asset_b, period).map(|AggregatedEntry { price, oracle_age, .. }| (price, oracle_age))
    }

    fn get_price_weight() -> Weight {
        Self::get_entry_weight()
    }
}
