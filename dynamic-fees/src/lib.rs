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
use sp_runtime::{FixedPointOperand, FixedU128, PerThing};

mod math;
#[cfg(test)]
mod tests;
pub mod traits;

pub use pallet::*;

use crate::math::{recalculate_asset_fee, recalculate_protocol_fee, AssetVolume, FeeParams};
use crate::traits::{Volume, VolumeProvider};

type Balance = u128;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use crate::traits::VolumeProvider;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::BlockNumberFor;
    use sp_runtime::traits::{BlockNumberProvider, Zero};

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn asset_fee)]
    /// Stores last calculated fee of an asset and block number in which it was changed..
    /// Stored as (Asset fee, Protocol fee, Block number)
    pub type AssetFee<T: Config> =
        StorageMap<_, Twox64Concat, T::AssetId, (T::Fee, T::Fee, T::BlockNumber), OptionQuery>;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Provider for the current block number.
        type BlockNumberProvider: BlockNumberProvider<BlockNumber = Self::BlockNumber>;

        /// Fee PerThing type
        type Fee: Parameter + MaybeSerializeDeserialize + MaxEncodedLen + PerThing;

        /// Asset id type
        type AssetId: Parameter + Member + Copy + MaybeSerializeDeserialize + MaxEncodedLen;

        /// Oracle period type
        type OraclePeriod: Parameter + Member + MaybeSerializeDeserialize;

        /// Volume provider implementation
        type Oracle: VolumeProvider<Self::AssetId, Balance, Self::OraclePeriod>;

        /// Chosen Oracle period
        #[pallet::constant]
        type SelectedPeriod: Get<Self::OraclePeriod>;

        /// Asset fee decay parameter
        #[pallet::constant]
        type AssetFeeDecay: Get<FixedU128>;

        /// Asset fee amplification parameter
        #[pallet::constant]
        type AssetFeeAmplification: Get<FixedU128>;

        /// Minimum asset fee
        #[pallet::constant]
        type AssetMinimumFee: Get<Self::Fee>;

        /// Maximum asset fee
        #[pallet::constant]
        type AssetMaximumFee: Get<Self::Fee>;

        /// Protocol fee decay parameter
        #[pallet::constant]
        type ProtocolFeeDecay: Get<FixedU128>;

        /// Protocol fee amplification
        #[pallet::constant]
        type ProtocolFeeAmplification: Get<FixedU128>;

        /// Minimum protocol fee
        #[pallet::constant]
        type ProtocolMinimumFee: Get<Self::Fee>;

        /// Maximum protocol fee
        #[pallet::constant]
        type ProtocolMaximumFee: Get<Self::Fee>;
    }

    #[pallet::event]
    pub enum Event<T: Config> {}

    #[pallet::error]
    pub enum Error<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {}

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn integrity_test() {
            assert!(
                T::AssetMinimumFee::get() <= T::AssetMaximumFee::get(),
                "Asset fee min > asset fee max."
            );
            assert!(
                !T::AssetFeeAmplification::get().is_zero(),
                "Asset fee amplification is 0."
            );
            assert!(
                T::ProtocolMinimumFee::get() <= T::ProtocolMaximumFee::get(),
                "Protocol fee min > protocol fee max."
            );
            assert!(
                !T::ProtocolFeeAmplification::get().is_zero(),
                "Protocol fee amplification is 0."
            );
        }
    }
}

impl<T: Config> Pallet<T>
where
    <T::Fee as PerThing>::Inner: FixedPointOperand,
{
    fn update_fee(asset_id: T::AssetId) -> (T::Fee, T::Fee) {
        let block_number = T::BlockNumberProvider::current_block_number();

        let (current_fee, current_protocol_fee, last_block) = Self::asset_fee(asset_id).unwrap_or((
            T::AssetMinimumFee::get(),
            T::ProtocolMinimumFee::get(),
            T::BlockNumber::default(),
        ));

        let Some(delta_blocks) = TryInto::<u128>::try_into(block_number.saturating_sub(last_block)).ok() else{
            return (current_fee, current_protocol_fee);
        };

        // Update only if it has not yet been updated this block
        if block_number != last_block {
            let Some(volume) = T::Oracle::asset_volume(asset_id, T::SelectedPeriod::get()) else{
                return (current_fee, current_protocol_fee);
            };
            let Some(liquidity) = T::Oracle::asset_liquidity(asset_id, T::SelectedPeriod::get()) else{
                return (current_fee, current_protocol_fee);
            };

            let asset_fee = recalculate_asset_fee(
                AssetVolume {
                    amount_in: volume.amount_in(),
                    amount_out: volume.amount_out(),
                    liquidity,
                },
                current_fee,
                delta_blocks,
                Self::asset_fee_params(),
            );
            let protocol_fee = recalculate_protocol_fee(
                AssetVolume {
                    amount_in: volume.amount_in(),
                    amount_out: volume.amount_out(),
                    liquidity,
                },
                current_protocol_fee,
                delta_blocks,
                Self::protocol_fee_params(),
            );

            AssetFee::<T>::insert(asset_id, (asset_fee, protocol_fee, block_number));
            (asset_fee, protocol_fee)
        } else {
            (current_fee, current_protocol_fee)
        }
    }

    fn asset_fee_params() -> FeeParams<T::Fee> {
        FeeParams {
            max_fee: T::AssetMaximumFee::get(),
            min_fee: T::AssetMinimumFee::get(),
            decay: T::AssetFeeDecay::get(),
            amplification: T::AssetFeeAmplification::get(),
        }
    }

    fn protocol_fee_params() -> FeeParams<T::Fee> {
        FeeParams {
            max_fee: T::ProtocolMaximumFee::get(),
            min_fee: T::ProtocolMinimumFee::get(),
            decay: T::ProtocolFeeDecay::get(),
            amplification: T::ProtocolFeeAmplification::get(),
        }
    }
}

pub struct UpdateAndRetrieveFees<T: Config>(sp_std::marker::PhantomData<T>);

impl<T: Config> GetByKey<T::AssetId, (T::Fee, T::Fee)> for UpdateAndRetrieveFees<T>
where
    <T::Fee as PerThing>::Inner: FixedPointOperand,
{
    fn get(k: &T::AssetId) -> (T::Fee, T::Fee) {
        Pallet::<T>::update_fee(*k)
    }
}
