#![allow(warnings)]
// This file is part of pallet-dca.

// Copyright (C) 2020-2022  Intergalactic, Limited (GIB).
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

use codec::{Decode, Encode};
use frame_support::ensure;
use frame_support::traits::fungibles::Inspect;
use frame_support::traits::Get;
use frame_support::transactional;
use frame_system::ensure_signed;
use hydradx_traits::router::TradeExecution;
use hydradx_traits::router::{ExecutorError, PoolType};
use orml_traits::arithmetic::{CheckedAdd, CheckedSub};
use scale_info::TypeInfo;
use sp_runtime::DispatchError;
use sp_std::vec::Vec;

#[cfg(test)]
mod tests;

pub mod weights;
pub mod types;

use weights::WeightInfo;

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;
use crate::types::Balance;


#[derive(Encode, Decode, Debug, Eq, PartialEq, Clone, TypeInfo)]
pub enum Recurrence{
    Fixed,
    Perpetual
}

#[derive(Encode, Decode, Debug, Eq, PartialEq, Clone, TypeInfo)]
pub struct Order {
    pub asset_in: Balance,
    pub asset_out: Balance,
    pub amount_in: Balance,
    pub amount_out: Balance,
    pub limit: Balance,
    pub route: Vec<Trade<u128>>
}

#[derive(Encode, Decode, Debug, Eq, PartialEq, Clone, TypeInfo)]
pub struct Schedule {
    pub period: u128,
    pub recurrence: Recurrence,
    pub order: Order
}

///A single trade for buy/sell, describing the asset pair and the pool type in which the trade is executed
#[derive(Encode, Decode, Debug, Eq, PartialEq, Clone, TypeInfo)]
pub struct Trade<AssetId> {
    pub pool: PoolType<AssetId>,
    pub asset_in: AssetId,
    pub asset_out: AssetId,
}


#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::OriginFor;
    use hydradx_traits::router::ExecutorError;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Weight information for the extrinsics.
        type WeightInfo: WeightInfo;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        ///First event
        DummyEvent {
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        ///First error
        DummyError,

    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        ///Schedule
        #[pallet::weight(<T as Config>::WeightInfo::sell(5))]
        #[transactional]
        pub fn schedule(
            origin: OriginFor<T>,
            schedule: Schedule
        ) -> DispatchResult {
            let who = ensure_signed(origin.clone())?;
            Ok(())
        }
    }
}

impl<T: Config> Pallet<T> {

}
