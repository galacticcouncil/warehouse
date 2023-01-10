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

//! # EMA Oracle Pallet
//!
//! ## Overview
//!
//! This pallet provides oracles of different periods for a combination of source and asset pair
//! based on data coming in from `OnActivityHandler`.
//!
//! It is meant to be used by other pallets via the `AggregatedOracle` and `AggregatedPriceOracle`
//! traits.
//!
//! When integrating with this pallet take care to use the `on_trade_weight`,
//! `on_liquidity_changed_weight` and `get_entry_weight` into account when calculating the weight
//! for your extrinsics.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet_prelude::*;
use frame_support::sp_runtime::traits::{BlockNumberProvider, One, Zero};
use hydradx_traits::{
    AggregatedEntry, AggregatedOracle, AggregatedPriceOracle, OnCreatePoolHandler, OnLiquidityChangedHandler,
    OnTradeHandler,
    OraclePeriod::{self, *},
    Volume,
};
use sp_arithmetic::traits::Saturating;
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

const LOG_TARGET: &str = "runtime::ema-oracle";

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

#[allow(clippy::type_complexity)]
#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::BoundedBTreeMap;
    use frame_system::pallet_prelude::BlockNumberFor;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Weight information for the extrinsics.
        type WeightInfo: WeightInfo;

        /// Provider for the current block number.
        type BlockNumberProvider: BlockNumberProvider<BlockNumber = Self::BlockNumber>;

        /// Number of seconds between blocks, used to convert periods.
        type SecsPerBlock: Get<Self::BlockNumber>;
    }

    #[pallet::error]
    pub enum Error<T> {}

    #[pallet::event]
    pub enum Event<T: Config> {}

    /// Accumulator for oracle data in current block that will be recorded at the end of the block.
    #[pallet::storage]
    #[pallet::getter(fn accumulator)]
    pub type Accumulator<T: Config> = StorageValue<
        _,
        BoundedBTreeMap<(Source, (AssetId, AssetId)), OracleEntry<T::BlockNumber>, ConstU32<MAX_TRADES>>,
        ValueQuery,
    >;

    /// Orace storage keyed by data source, involved asset ids and the period length of the oracle.
    ///
    /// Stores the data entry as well as the block number when the oracle was first initialized.
    #[pallet::storage]
    #[pallet::getter(fn oracle)]
    pub type Oracles<T: Config> = StorageNMap<
        _,
        (
            NMapKey<Twox64Concat, Source>,
            NMapKey<Twox64Concat, (AssetId, AssetId)>,
            NMapKey<Twox64Concat, T::BlockNumber>,
        ),
        (OracleEntry<T::BlockNumber>, T::BlockNumber),
        OptionQuery,
    >;

    #[pallet::genesis_config]
    #[derive(Default)]
    pub struct GenesisConfig {
        pub price_data: Vec<(Source, (AssetId, AssetId), Price, Balance)>,
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig {
        fn build(&self) {
            for &(source, (asset_a, asset_b), price, liquidity) in self.price_data.iter() {
                let entry: OracleEntry<T::BlockNumber> = OracleEntry {
                    price,
                    volume: Volume::default(),
                    liquidity,
                    timestamp: T::BlockNumber::zero(),
                };
                for period in OraclePeriod::all_periods() {
                    Pallet::<T>::update_oracle(
                        source,
                        ordered_pair(asset_a, asset_b),
                        into_blocks::<T>(period),
                        entry.clone(),
                    );
                }
            }
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
            T::WeightInfo::on_finalize_no_entry()
        }

        fn on_finalize(_n: BlockNumberFor<T>) {
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
    pub(crate) fn on_entry(src: Source, assets: (AssetId, AssetId), oracle_entry: OracleEntry<T::BlockNumber>) {
        Accumulator::<T>::mutate(|accumulator| {
            let is_present = if let Some(entry) = accumulator.get_mut(&(src, assets)) {
                entry.accumulate_volume_and_update_from(&oracle_entry);
                true
            } else {
                false
            };
            if !is_present {
                accumulator.try_insert((src, assets), oracle_entry).unwrap_or_else(|((src, assets), entry)| {
                        log::error!(
                            target: LOG_TARGET,
                            "Could not insert oracle entry at ({src:?}, {assets:?}). Dropping new entry ({entry:?}). This should not happen and implies that the configuration needs to be changed!"
                        );
                        debug_assert!(false, "Should not try to insert more than MAX_TRADES entries in accumulator.");
                        None
                    });
            }
        });
    }

    pub(crate) fn on_trade(src: Source, assets: (AssetId, AssetId), oracle_entry: OracleEntry<T::BlockNumber>) {
        Self::on_entry(src, assets, oracle_entry)
    }

    pub(crate) fn on_liquidity_changed(
        src: Source,
        assets: (AssetId, AssetId),
        oracle_entry: OracleEntry<T::BlockNumber>,
    ) {
        Self::on_entry(src, assets, oracle_entry)
    }

    /// Update oracles based on data accumulated during the block.
    fn update_oracles_from_accumulator() {
        for ((src, assets), oracle_entry) in Accumulator::<T>::take().into_iter() {
            for period in OraclePeriod::non_immediate_periods() {
                Self::update_oracle(src, assets, into_blocks::<T>(period), oracle_entry.clone());
            }
            // We use (the old value of) the `LastBlock` entry to update the other oracles so it
            // gets updated last.
            Self::update_oracle(src, assets, into_blocks::<T>(&LastBlock), oracle_entry.clone());
        }
    }

    /// Update the oracle of the given source, assets and period with `oracle_entry`.
    fn update_oracle(
        src: Source,
        assets: (AssetId, AssetId),
        period: T::BlockNumber,
        oracle_entry: OracleEntry<T::BlockNumber>,
    ) {
        Oracles::<T>::mutate((src, assets, period), |oracle| {
            // initialize the oracle entry if it doesn't exist
            if oracle.is_none() {
                *oracle = Some((oracle_entry.clone(), T::BlockNumberProvider::current_block_number()));
                return;
            }
            if let Some((prev_entry, _)) = oracle.as_mut() {
                let parent = T::BlockNumberProvider::current_block_number().saturating_sub(One::one());
                // update the entry to the parent block if it hasn't been updated for a while
                // skip if we're updating the `LastBlock` event
                if parent > prev_entry.timestamp && period != into_blocks::<T>(&LastBlock) {
                    Self::oracle((src, assets, into_blocks::<T>(&LastBlock)))
                        .and_then(|(mut last_block, _)| -> Option<()> {
                            // update the `LastBlock` oracle to the last block if it hasn't been updated for a while
                            // price and liquidity stay constant, volume becomes zero
                            if last_block.timestamp != parent {
                                last_block.fast_forward_to(parent);
                            }
                            prev_entry.update_via_ema_with(period, &last_block)?;
                            Some(())
                        }).unwrap_or_else(|| {
                            log::warn!(
                                target: LOG_TARGET,
                                "Updating EMA oracle ({src:?}, {assets:?}, {period:?}) to parent block failed. Defaulting to previous value."
                            );
                            debug_assert!(false, "Updating to parent block should not fail.");
                        })
                }
                // calculate the actual update with the new value
                prev_entry.update_via_ema_with(period, &oracle_entry)
                    .unwrap_or_else(|| {
                        log::warn!(
                            target: LOG_TARGET,
                            "Updating EMA oracle ({src:?}, {assets:?}, {period:?}) to new value failed. Defaulting to previous value."
                        );
                        debug_assert!(false, "Updating to new value should not fail.");
                });
            };
        });
    }

    /// Return the updated oracle entry for the given source, assets and period.
    ///
    /// The value will be up to date until the parent block, thus excluding trading data from the
    /// current block. Note: It does not update the values in storage.
    fn get_updated_entry(
        src: Source,
        assets: (AssetId, AssetId),
        period: OraclePeriod,
    ) -> Option<(OracleEntry<T::BlockNumber>, T::BlockNumber)> {
        let parent = T::BlockNumberProvider::current_block_number().saturating_sub(One::one());
        // First get the `LastBlock` oracle as we will use it to calculate the updated values for
        // the others.
        let (mut last_block, init) = Self::oracle((src, assets, into_blocks::<T>(&LastBlock)))?;
        // update the `LastBlock` oracle to the last block if it hasn't been updated for a while
        // price and liquidity stay constant, volume becomes zero
        if last_block.timestamp != parent {
            last_block.fast_forward_to(parent);
        }

        if period == LastBlock {
            return Some((last_block, init));
        }

        let (entry, init) = Self::oracle((src, assets, into_blocks::<T>(&period)))?;
        if entry.timestamp < parent {
            entry.combine_via_ema_with(into_blocks::<T>(&period), &last_block)
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
    fn on_trade(
        source: Source,
        asset_in: AssetId,
        asset_out: AssetId,
        amount_in: Balance,
        amount_out: Balance,
        liquidity: Balance,
    ) {
        // We assume that zero values are not valid and can be ignored.
        if liquidity.is_zero() || amount_in.is_zero() || amount_out.is_zero() {
            log::warn!(target: LOG_TARGET, "Neither liquidity nor amounts should be zero. Ignoring. Source: {source:?}, liquidity: {liquidity}, amount_in: {amount_in}, amount_out: {amount_out}");
            return;
        }
        let price = determine_normalized_price(asset_in, asset_out, amount_in, amount_out);
        let volume = determine_normalized_volume(asset_in, asset_out, amount_in, amount_out);

        let timestamp = T::BlockNumberProvider::current_block_number();
        let entry = OracleEntry {
            price,
            volume,
            liquidity,
            timestamp,
        };
        Pallet::<T>::on_trade(source, ordered_pair(asset_in, asset_out), entry);
    }

    fn on_trade_weight() -> Weight {
        // on_trade + on_finalize / max_trades
        T::WeightInfo::on_trade_multiple_tokens(MAX_TRADES).saturating_add(
            // TODO: Can we also divide the proof size weight the same way?
            Weight::from_ref_time(
                T::WeightInfo::on_finalize_multiple_tokens(MAX_TRADES)
                    .saturating_sub(T::WeightInfo::on_finalize_no_entry())
                    .ref_time()
                    / (MAX_TRADES as u64),
            ),
        )
    }
}

impl<T: Config> OnLiquidityChangedHandler<AssetId, Balance> for OnActivityHandler<T> {
    fn on_liquidity_changed(
        source: Source,
        asset_a: AssetId,
        asset_b: AssetId,
        amount_a: Balance,
        amount_b: Balance,
        liquidity: Balance,
    ) {
        // We assume that zero values are not valid and can be ignored.
        if liquidity.is_zero() || amount_a.is_zero() || amount_b.is_zero() {
            log::warn!(target: LOG_TARGET, "Neither liquidity nor amounts should be zero. Ignoring. Source: {source:?}, liquidity: {liquidity}, amount_a: {amount_a}, amount_b: {amount_b}");
            return;
        }
        // We don't want to throw an error here because this method is used in different extrinsics.
        let price = determine_normalized_price(asset_a, asset_b, amount_a, amount_b);
        let timestamp = T::BlockNumberProvider::current_block_number();
        let entry = OracleEntry {
            price,
            // liquidity provision does not count as trade volume
            volume: Volume::default(),
            liquidity,
            timestamp,
        };
        Pallet::<T>::on_liquidity_changed(source, ordered_pair(asset_a, asset_b), entry);
    }

    fn on_liquidity_changed_weight() -> Weight {
        // on_liquidity + on_finalize / max_trades
        T::WeightInfo::on_liquidity_changed_multiple_tokens(MAX_TRADES).saturating_add(
            // TODO: Can we also divide the proof size weight the same way?
            Weight::from_ref_time(
                T::WeightInfo::on_finalize_multiple_tokens(MAX_TRADES)
                    .saturating_sub(T::WeightInfo::on_finalize_no_entry())
                    .ref_time()
                    / (MAX_TRADES as u64),
            ),
        )
    }
}

/// Calculate price from ordered assets
pub fn determine_normalized_price(
    asset_in: AssetId,
    asset_out: AssetId,
    amount_in: Balance,
    amount_out: Balance,
) -> Price {
    if ordered_pair(asset_in, asset_out) == (asset_in, asset_out) {
        Price::new(amount_in, amount_out)
    } else {
        Price::new(amount_out, amount_in)
    }
}

/// Construct `Volume` based on unordered assets.
pub fn determine_normalized_volume(
    asset_in: AssetId,
    asset_out: AssetId,
    amount_in: Balance,
    amount_out: Balance,
) -> Volume<Balance> {
    if ordered_pair(asset_in, asset_out) == (asset_in, asset_out) {
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
}

impl<T: Config> AggregatedOracle<AssetId, Balance, T::BlockNumber, Price> for Pallet<T> {
    type Error = OracleError;

    /// Returns the entry corresponding to the given assets and period.
    /// The entry is updated to the state of the parent block (but not trading data in the current
    /// block). It is also adjusted to make sense for the asset order given as parameters. So
    /// calling `get_entry(HDX, DOT, LastBlock, Omnipool)` will return the price `HDX/DOT`, while
    /// `get_entry(DOT, HDX, LastBlock, Omnipool)` will return `DOT/HDX`.
    fn get_entry(
        asset_a: AssetId,
        asset_b: AssetId,
        period: OraclePeriod,
        source: Source,
    ) -> Result<AggregatedEntry<Balance, T::BlockNumber, Price>, OracleError> {
        if asset_a == asset_b {
            return Err(OracleError::SameAsset);
        };
        Self::get_updated_entry(source, ordered_pair(asset_a, asset_b), period)
            .ok_or(OracleError::NotPresent)
            .map(|(entry, initialized)| {
                let entry = if (asset_a, asset_b) != ordered_pair(asset_a, asset_b) {
                    entry.inverted()
                } else {
                    entry
                };
                entry.into_aggregated(initialized)
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
        source: Source,
    ) -> Result<(Price, T::BlockNumber), Self::Error> {
        Self::get_entry(asset_a, asset_b, period, source)
            .map(|AggregatedEntry { price, oracle_age, .. }| (price, oracle_age))
    }

    fn get_price_weight() -> Weight {
        Self::get_entry_weight()
    }
}
