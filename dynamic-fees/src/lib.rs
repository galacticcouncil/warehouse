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

use frame_support::traits::Get;
use orml_traits::GetByKey;
use sp_runtime::traits::{BlockNumberProvider, Saturating};
use sp_runtime::{FixedU128, Permill};

mod math;
#[cfg(test)]
mod tests;

// Re-export pallet items so that they can be accessed from the crate namespace.
use crate::math::{recalculate_asset_fee, recalculate_protocol_fee, AssetVolume, FeeParams};
pub use pallet::*;

type Fee = Permill;
type Balance = u128;

pub trait Volume<Balance> {
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
    /// Stored as (Asset fee, Protocol fee, Block number)
    pub type AssetFee<T: Config> = StorageMap<_, Twox64Concat, T::AssetId, (Fee, Fee, T::BlockNumber), OptionQuery>;

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
    fn update_fee(pair: (T::AssetId, T::AssetId)) -> (Fee, Fee) {
        let block_number = T::BlockNumberProvider::current_block_number();

        //TODO: what if no previous and block 0 ( not happning but should be covered)
        let (current_fee, current_protocol_fee, last_block) =
            Self::asset_fee(pair.0).unwrap_or((Fee::zero(), Fee::zero(), T::BlockNumber::default()));

        //TODO: it is difference btween blocks?!
        let delta_blocks = block_number.saturating_sub(last_block);
        let db = TryInto::<u128>::try_into(delta_blocks).ok().unwrap();

        // Update only if it was not yet updated in this block
        if block_number != last_block {
            let Some(volume) = T::Oracle::asset_pair_volume(pair, T::SelectedPeriod::get()) else{
                //TODO: what if fails to retrieve from oracle
                return (T::MaximumFee::get(), T::MaximumFee::get());
            };
            let Some(liquidity) = T::Oracle::asset_pair_liquidity(pair, T::SelectedPeriod::get()) else{
                //TODO: what if fails to retrieve from oracle
                return (T::MaximumFee::get(), T::MaximumFee::get());
            };

            let f = recalculate_asset_fee(
                AssetVolume {
                    amount_in: volume.amount_a_in(),
                    amount_out: volume.amount_a_out(),
                    liquidity,
                },
                Some(current_fee),
                db,
                Self::asset_fee_params(),
            );
            let protocol_fee = recalculate_protocol_fee(
                AssetVolume {
                    amount_in: volume.amount_a_in(),
                    amount_out: volume.amount_a_out(),
                    liquidity,
                },
                Some(current_protocol_fee),
                db,
                Self::protocol_fee_params(),
            );

            AssetFee::<T>::insert(pair.0, (f, protocol_fee, block_number));
            (f, protocol_fee)
        } else {
            (current_fee, current_protocol_fee)
        }
    }

    fn asset_fee_params() -> FeeParams {
        FeeParams {
            max_fee: T::MaximumFee::get(),
            min_fee: T::MinimumFee::get(),
            decay: T::Decay::get(),
            amplification: T::Amplification::get(),
        }
    }

    fn protocol_fee_params() -> FeeParams {
        FeeParams {
            max_fee: T::MaximumFee::get(),
            min_fee: T::MinimumFee::get(),
            decay: T::Decay::get(),
            amplification: T::Amplification::get(),
        }
    }
}

pub struct UpdateAndRetrieveAssetFee<T: Config>(sp_std::marker::PhantomData<T>);

impl<T: Config> GetByKey<(T::AssetId, T::AssetId), Fee> for UpdateAndRetrieveAssetFee<T> {
    fn get(k: &(T::AssetId, T::AssetId)) -> Fee {
        Pallet::<T>::update_fee(*k).0
    }
}

pub struct UpdateAndRetrieveFees<T: Config>(sp_std::marker::PhantomData<T>);

impl<T: Config> GetByKey<(T::AssetId, T::AssetId), (Fee, Fee)> for UpdateAndRetrieveFees<T> {
    fn get(k: &(T::AssetId, T::AssetId)) -> (Fee, Fee) {
        Pallet::<T>::update_fee(*k)
    }
}

pub struct RetrieveAssetFee<T: Config>(sp_std::marker::PhantomData<T>);

impl<T: Config> GetByKey<(T::AssetId, T::AssetId), Fee> for RetrieveAssetFee<T> {
    fn get(k: &(T::AssetId, T::AssetId)) -> Fee {
        Pallet::<T>::asset_fee(k.0)
            .unwrap_or((T::MaximumFee::get(), T::MaximumFee::get(), T::BlockNumber::default()))
            .0
    }
}
