// This file is part of pallet-price-oracle.

// Copyright (C) 2020-2021  Intergalactic, Limited (GIB).
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

use frame_support::ensure;
use frame_support::pallet_prelude::*;
use frame_support::sp_runtime::traits::{CheckedDiv, One, Zero};
use frame_support::sp_runtime::{DispatchResult, FixedPointNumber};
use hydradx_traits::{OnCreatePoolHandler, OnTradeHandler};
use sp_std::convert::TryInto;
use sp_std::marker::PhantomData;
use sp_std::prelude::*;

use sp_std::collections::btree_map::BTreeMap;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod types;
pub use types::*;

#[allow(clippy::all)]
pub mod weights;
use weights::WeightInfo;

mod benchmarking; // TODO: rebenchmark

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

/// Unique identifier for an asset pair.
/// AMM pools derive their own unique identifiers for asset pairs,
/// but this one is meant to not be bounded to one particular AMM pool.
pub type AssetPairId = Vec<u8>;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Weight information for the extrinsics.
        type WeightInfo: WeightInfo;

        /// Number of seconds between blocks, used to convert periods.
        type SecsPerBlock: Get<Period>;
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Calculation error occurred while calculating average price
        PriceComputationError,

        /// An unexpected overflow occurred
        UpdateDataOverflow,

        /// Asset has been already added
        AssetAlreadyAdded,

        /// Overflow
        TrackedAssetsOverflow,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {}

    #[pallet::storage]
    #[pallet::getter(fn accumulator)]
    pub type Accumulator<T: Config> = StorageValue<_, BTreeMap<AssetPairId, PriceEntry<T::BlockNumber>>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn oracle)]
    pub type Oracles<T: Config> =
        StorageDoubleMap<_, Twox64Concat, AssetPairId, Twox64Concat, Period, PriceEntry<T::BlockNumber>, OptionQuery>;

    #[pallet::genesis_config]
    #[derive(Default)]
    pub struct GenesisConfig {
        pub price_data: Vec<((AssetId, AssetId), Price, Balance, Balance)>,
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig {
        fn build(&self) {
            for &(asset_pair, price, volume, liquidity) in self.price_data.iter() {
                let pair_id = determine_name(asset_pair.0, asset_pair.1);

                let price_entry: PriceEntry<T::BlockNumber> = PriceEntry {
                    price,
                    volume,
                    liquidity,
                    timestamp: T::BlockNumber::zero(),
                };
                for period in OraclePeriod::all_periods() {
                    Pallet::<T>::update_oracle(&pair_id, period.into_num::<T>(), &price_entry);
                }
            }
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
        fn on_initialize(_n: T::BlockNumber) -> Weight {
            T::WeightInfo::on_finalize_multiple_tokens_all_bucket_levels(5) // TODO update weights
        }

        fn on_finalize(_n: T::BlockNumber) {
            // update average values in the storage
            Self::update_data();
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {}
}

impl<T: Config> Pallet<T> {
    pub fn on_trade(pair_id: AssetPairId, price_entry: PriceEntry<T::BlockNumber>) {
        Accumulator::<T>::mutate(|accumulator| {
            accumulator
                .entry(pair_id)
                .and_modify(|entry| {
                    *entry = entry.accumulate_volume(&price_entry);
                })
                .or_insert(price_entry);
        });
    }

    fn update_data() {
        // EMA oracles
        for (pair_id, price_entry) in Accumulator::<T>::take().into_iter() {
            for period in OraclePeriod::all_periods() {
                Self::update_oracle(&pair_id, period.into_num::<T>(), &price_entry);
            }
        }
    }

    fn update_oracle(pair_id: &AssetPairId, period: Period, price_entry: &PriceEntry<T::BlockNumber>) {
        Oracles::<T>::mutate(pair_id, period, |oracle| {
            let new_entry = oracle
                .map(|entry| {
                    let v = price_entry.calculate_new_ema_entry(period, &entry).unwrap_or(entry);
                    log::debug!("v {v:?}");
                    v
                })
                .unwrap_or(price_entry.clone());
            *oracle = Some(new_entry);
        });
    }

    fn get_updated_entry(pair_id: &AssetPairId, period: OraclePeriod) -> Option<PriceEntry<T::BlockNumber>> {
        use sp_arithmetic::traits::Saturating;

        let current_block = <frame_system::Pallet<T>>::block_number();
        let parent = current_block.saturating_sub(One::one());

        let mut immediate = Oracles::<T>::get(pair_id, Immediate.into_num::<T>())?;
        if immediate.timestamp < parent {
            immediate.timestamp = parent;
            Oracles::<T>::insert(pair_id, Immediate.into_num::<T>(), &immediate);
        }

        log::debug!("immediate: {immediate:?}");
        let mut r = None;
        OraclePeriod::non_immediate_periods()
            .iter()
            .map(|p| {
                let entry = Self::oracle(pair_id, p.into_num::<T>())?;
                let return_entry = if entry.timestamp < parent {
                    immediate
                        .calculate_new_ema_entry(p.into_num::<T>(), &entry)
                        .map(|new_entry| {
                            Oracles::<T>::insert(pair_id, period.into_num::<T>(), &new_entry);
                            new_entry
                        })
                        .unwrap_or(entry)
                } else {
                    entry
                };
                if p == &period {
                    r = Some(return_entry);
                }
                Some(())
            })
            .for_each(|_| {});
        log::debug!("r: {r:?}");
        if period == Immediate {
            Some(immediate)
        } else {
            r
        }
    }
}

pub struct PriceOracleHandler<T>(PhantomData<T>);

impl<T: Config> OnTradeHandler<AssetId, Balance> for PriceOracleHandler<T> {
    fn on_trade(asset_a: AssetId, asset_b: AssetId, amount_in: Balance, amount_out: Balance, liquidity: Balance) {
        let (price, amount) =
            if let Some(price_tuple) = determine_normalized_price(asset_a, asset_b, amount_in, amount_out) {
                price_tuple
            } else {
                // We don't want to throw an error here because this method is used in different extrinsics.
                // Invalid prices are ignored and not added to the queue.
                return;
            };

        // We assume that zero values are not valid.
        // Zero values are ignored and not added to the queue.
        if price.is_zero() || amount.is_zero() || liquidity.is_zero() {
            return;
        }

        let timestamp = <frame_system::Pallet<T>>::block_number();
        let price_entry = PriceEntry {
            price,
            volume: amount,
            liquidity,
            timestamp,
        };

        Pallet::<T>::on_trade(determine_name(asset_a, asset_b), price_entry);
    }

    fn on_trade_weight() -> Weight {
        T::WeightInfo::on_finalize_one_token() - T::WeightInfo::on_finalize_no_entry()
    }
}

// TODO: extract
/// Calculate price from ordered assets
pub fn determine_normalized_price(
    asset_a: AssetId,
    asset_b: AssetId,
    amount_in: Balance,
    amount_out: Balance,
) -> Option<(Price, Balance)> {
    let ordered_asset_pair = ordered_pair(asset_a, asset_b);
    let (balance_a, balance_b) = if ordered_asset_pair.0 == asset_a {
        (amount_in, amount_out)
    } else {
        (amount_out, amount_in)
    };

    let price_a = Price::checked_from_integer(balance_a)?;
    let price_b = Price::checked_from_integer(balance_b)?;
    let price = price_a.checked_div(&price_b);
    price.map(|p| (p, balance_a))
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
pub fn determine_name(asset_a: AssetId, asset_b: AssetId) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();

    let (asset_left, asset_right) = ordered_pair(asset_a, asset_b);

    buf.extend_from_slice(&asset_left.to_le_bytes());
    buf.extend_from_slice(b"HDT");
    buf.extend_from_slice(&asset_right.to_le_bytes());

    buf
}

use codec::{Decode, Encode};
use frame_support::RuntimeDebug;

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, TypeInfo)]
pub enum OraclePeriod {
    Immediate,
    TenMinutes,
    Day,
    Week,
}
use OraclePeriod::*;

impl OraclePeriod {
    pub fn all_periods() -> &'static [OraclePeriod] {
        &[Immediate, TenMinutes, Day, Week]
    }

    pub fn non_immediate_periods() -> &'static [OraclePeriod] {
        &[TenMinutes, Day, Week]
    }

    pub fn into_num<T: Config>(self) -> Period {
        let secs_per_block = T::SecsPerBlock::get();
        let minutes = 60 / secs_per_block;
        let hours = 60 * minutes;
        let days = 24 * hours;
        match self {
            OraclePeriod::Immediate => 1,
            OraclePeriod::TenMinutes => 10 * minutes,
            OraclePeriod::Day => 1 * days,
            OraclePeriod::Week => 7 * days,
        }
    }
}

pub trait EmaOracle {
    fn get_price(asset_a: AssetId, asset_b: AssetId, period: OraclePeriod) -> (Option<Price>, Weight);
}

impl<T: Config> EmaOracle for Pallet<T> {
    fn get_price(asset_a: AssetId, asset_b: AssetId, period: OraclePeriod) -> (Option<Price>, Weight) {
        let pair_id = determine_name(asset_a, asset_b);
        let entry = Self::get_updated_entry(&pair_id, period);
        (entry.map(|entry| entry.price), 100) // TODO: weight
    }
}
