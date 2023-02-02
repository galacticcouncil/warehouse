// This file is part of pallet-dynamic-fees.

// Copyright (C) 2020-2023  Intergalactic, Limited (GIB).
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

extern crate core;

use frame_support::traits::Get;
use orml_traits::GetByKey;
use sp_runtime::traits::{BlockNumberProvider, Saturating, Zero};
use sp_runtime::{FixedPointNumber, FixedU128, Permill};
use std::fmt::Debug;

#[cfg(test)]
mod tests;

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

type Fee = Permill;
type Balance = u128;

pub trait Volume<Balance>: Debug {
    fn amount_a_in(&self) -> Balance;
    fn amount_b_in(&self) -> Balance;
    fn amount_a_out(&self) -> Balance;
    fn amount_b_out(&self) -> Balance;
}

pub trait VolumeProvider<AssetId, Balance, Period> {
    type Volume: Volume<Balance>;

    fn asset_pair_volume(pair: (AssetId, AssetId), period: Period) -> Option<Self::Volume>;

    fn asset_pair_liquidity(pair: (AssetId, AssetId), period: Period) -> Option<Balance>;
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use sp_runtime::traits::BlockNumberProvider;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn asset_fee)]
    /// STores last calculated fee of an asset and block number in which it was changed..
    pub type AssetFee<T: Config> = StorageMap<_, Twox64Concat, T::AssetId, (Fee, T::BlockNumber), OptionQuery>;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Provider for the current block number.
        type BlockNumberProvider: BlockNumberProvider<BlockNumber = Self::BlockNumber>;

        /// Asset id type
        type AssetId: Parameter + Member + Copy + MaybeSerializeDeserialize + MaxEncodedLen;

        /// Oracle period type
        type OraclePeriod: Parameter + Member + Copy + MaybeSerializeDeserialize;

        ///
        type Oracle: VolumeProvider<Self::AssetId, Balance, Self::OraclePeriod>;

        #[pallet::constant]
        type SelectedPeriod: Get<Self::OraclePeriod>;

        #[pallet::constant]
        type Decay: Get<FixedU128>;

        #[pallet::constant]
        type Amplification: Get<FixedU128>;

        #[pallet::constant]
        type MinimumFee: Get<Fee>;

        #[pallet::constant]
        type MaximumFee: Get<Fee>;
    }

    #[pallet::event]
    pub enum Event<T: Config> {}

    #[pallet::error]
    pub enum Error<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {}
}

impl<T: Config> Pallet<T> {
    fn recalculate_fee(
        asset_id: T::AssetId,
        volume: <T::Oracle as VolumeProvider<T::AssetId, Balance, T::OraclePeriod>>::Volume,
        liquidity: Balance,
        block_number: T::BlockNumber,
    ) -> Fee {
        let previous_fee = if let Some(pf) = Self::asset_fee(asset_id) {
            let delta_blocks = block_number.saturating_sub(pf.1);
            let db = TryInto::<u128>::try_into(delta_blocks).ok().unwrap();
            let decaying = T::Decay::get().saturating_mul(FixedU128::from(db.saturating_sub(1)));
            let fee = FixedU128::from(pf.0);
            let s = Fee::from_rational(fee.saturating_sub(decaying).into_inner(), FixedU128::DIV);
            s.max(T::MinimumFee::get())
        } else {
            Fee::zero()
        };

        let v_o = volume.amount_a_out();
        let v_i = volume.amount_a_in();
        // x = (V0 - Vi) / L
        let (x, x_neg) = if liquidity != Balance::zero() {
            (FixedU128::from_rational(v_o.abs_diff(v_i), liquidity), v_o < v_i)
        } else {
            (FixedU128::zero(), false)
        };

        let a_x = T::Amplification::get().saturating_mul(x);

        let (delta_f, neg) = if x_neg {
            (a_x.saturating_add(T::Decay::get()), true)
        } else {
            if a_x > T::Decay::get() {
                (a_x.saturating_sub(T::Decay::get()), false)
            } else {
                (T::Decay::get().saturating_sub(a_x), true)
            }
        };

        let left = if neg {
            FixedU128::from(previous_fee)
                .saturating_sub(delta_f)
                .max(FixedU128::from(T::MinimumFee::get()))
        } else {
            FixedU128::from(previous_fee)
                .saturating_add(delta_f)
                .max(FixedU128::from(T::MinimumFee::get()))
        };

        let f_plus = left.min(FixedU128::from(T::MaximumFee::get()));

        let fee = Fee::from_rational(f_plus.into_inner(), FixedU128::DIV);

        AssetFee::<T>::insert(asset_id, (fee, block_number));

        fee
    }

    fn update_fee(pair: (T::AssetId, T::AssetId)) -> Fee {
        let block_number = T::BlockNumberProvider::current_block_number();
        let (current_fee, last_block) = Self::asset_fee(pair.0).unwrap_or((Fee::zero(), T::BlockNumber::default()));

        // Update only if it was not yet updated in this block
        if block_number != last_block {
            let Some(volume) = T::Oracle::asset_pair_volume(pair, T::SelectedPeriod::get()) else{
                return T::MaximumFee::get();
            };
            let Some(liquidity) = T::Oracle::asset_pair_liquidity(pair, T::SelectedPeriod::get()) else{
                return T::MaximumFee::get();
            };

            Self::recalculate_fee(pair.0, volume, liquidity, block_number)
        } else {
            current_fee
        }
    }
}

pub struct UpdateAndRetrieveAssetFee<T: Config>(sp_std::marker::PhantomData<T>);

impl<T: Config> GetByKey<(T::AssetId, T::AssetId), Fee> for UpdateAndRetrieveAssetFee<T> {
    fn get(k: &(T::AssetId, T::AssetId)) -> Fee {
        Pallet::<T>::update_fee(*k)
    }
}

pub struct RetrieveAssetFee<T: Config>(sp_std::marker::PhantomData<T>);

impl<T: Config> GetByKey<(T::AssetId, T::AssetId), Fee> for RetrieveAssetFee<T> {
    fn get(k: &(T::AssetId, T::AssetId)) -> Fee {
        Pallet::<T>::asset_fee(k.0)
            .unwrap_or((T::MaximumFee::get(), T::BlockNumber::default()))
            .0
    }
}
