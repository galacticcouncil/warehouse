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

//! Implementation of a fee level mechanism that dynamically changes based on the values provided by an oracle.
//!
//! ## Overview
//!
//! This module provides functionality to compute an asset fee and a protocol fee within a block.
//!
//! To use it in the runtime, implement the pallet's[`pallet_dynamic_fees::Config`]()
//!
//! and integrate provided [`UpdateAndRetrieveFees`]().
//!
//! ### Terminology
//!
//! * **Fee:** The type representing a fee. Must implement PerThing.
//! * **Oracle:** Implementation of an oracle providing volume in and out as wel ass liquidity for an asset.
//! * **Oracle period:** The period which is used to retrieve volumes of an asset from the oracle.
//! * **Asset decay:** The decaying parameter for an asset fee.
//! * **Protocol decay:** The decaying parameter for a protocol fee.
//! * **Asset fee amplification:** The amplification parameter for asset fee.
//! * **Protocol fee amplification:** The amplification parameter for protocol fee.
//! * **Minimum and maximum fee:** The minimum and maximum fee value for asset or protocol fee.
//!
//! ### Storage
//!
//! The module stores last calculated fees as tuple of `(Fee, Fee, Block number)` where the first item is asset fee,
//! the second one is protocol fee and the third one is block number indicating when the two fees were updated.
//!
//! ## Interface
//!
//! ### Update and retrieve fee
//!
//! The module provides implemenation of GetByKey trait for `UpdateAndRetrieveFee` struct.
//! This can be used to integrate the dynamic fee mechanism where desired.
//!
//! On first retrieve call in a block, the asset fee as well as the protocol are updated and new fees are returned.
//!
//! ### Prerequisites
//!
//! An oracle which provides volume in and out of an asset and liquidity for selected period.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::Get;
use orml_traits::GetByKey;
use sp_runtime::traits::{BlockNumberProvider, Saturating};
use sp_runtime::{FixedPointOperand, PerThing};

mod math;
#[cfg(test)]
mod tests;
pub mod traits;
pub mod types;

pub use pallet::*;

use crate::math::{recalculate_asset_fee, recalculate_protocol_fee, OracleEntry};
use crate::traits::{Volume, VolumeProvider};
use crate::types::FeeParams;

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

        #[pallet::constant]
        type AssetFeeParameters: Get<FeeParams<Self::Fee>>;

        #[pallet::constant]
        type ProtocolFeeParameters: Get<FeeParams<Self::Fee>>;
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
            let asset_fee_params = T::AssetFeeParameters::get();
            let protocol_fee_params = T::ProtocolFeeParameters::get();
            assert!(
                asset_fee_params.min_fee <= asset_fee_params.max_fee,
                "Asset fee min > asset fee max."
            );
            assert!(
                !asset_fee_params.amplification.is_zero(),
                "Asset fee amplification is 0."
            );
            assert!(
                protocol_fee_params.min_fee <= protocol_fee_params.max_fee,
                "Protocol fee min > protocol fee max."
            );
            assert!(
                !protocol_fee_params.amplification.is_zero(),
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

        let asset_fee_params = T::AssetFeeParameters::get();
        let protocol_fee_params = T::ProtocolFeeParameters::get();

        let (current_fee, current_protocol_fee, last_block) = Self::asset_fee(asset_id).unwrap_or((
            asset_fee_params.min_fee,
            protocol_fee_params.min_fee,
            T::BlockNumber::default(),
        ));

        let Some(delta_blocks) = TryInto::<u128>::try_into(block_number.saturating_sub(last_block)).ok() else {
            return (current_fee, current_protocol_fee);
        };

        // Update only if it has not yet been updated this block
        if block_number != last_block {
            let Some(volume) = T::Oracle::asset_volume(asset_id, T::SelectedPeriod::get()) else {
                return (current_fee, current_protocol_fee);
            };
            let Some(liquidity) = T::Oracle::asset_liquidity(asset_id, T::SelectedPeriod::get()) else {
                return (current_fee, current_protocol_fee);
            };

            let asset_fee = recalculate_asset_fee(
                OracleEntry {
                    amount_in: volume.amount_in(),
                    amount_out: volume.amount_out(),
                    liquidity,
                },
                current_fee,
                delta_blocks,
                asset_fee_params,
            );
            let protocol_fee = recalculate_protocol_fee(
                OracleEntry {
                    amount_in: volume.amount_in(),
                    amount_out: volume.amount_out(),
                    liquidity,
                },
                current_protocol_fee,
                delta_blocks,
                protocol_fee_params,
            );

            AssetFee::<T>::insert(asset_id, (asset_fee, protocol_fee, block_number));
            (asset_fee, protocol_fee)
        } else {
            (current_fee, current_protocol_fee)
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
