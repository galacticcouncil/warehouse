// This file is part of pallet-router.

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
#![allow(clippy::unused_unit)]

use frame_support::traits::fungibles::Inspect;
use frame_support::{
    ensure,
    weights::{DispatchClass, Pays},
};
use frame_system::ensure_signed;
use hydradx_traits::router::Executor;
use sp_std::vec::Vec;

pub mod types;

#[cfg(test)]
mod tests;

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

//TODO: Dani
//- add integration tests
//- refactoring
//- add API doc
//- benchmarking

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::OriginFor;
    use hydradx_traits::router::ExecutorError;
    use types::Trade;

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        type AssetId: Parameter + Member + Copy + MaybeSerializeDeserialize;

        type Balance: Parameter + Member + Copy + PartialOrd + MaybeSerializeDeserialize;

        type Currency: Inspect<Self::AccountId, AssetId = Self::AssetId, Balance = Self::Balance>;

        type AMM: Executor<Self::AccountId, Self::AssetId, Self::Balance, Output = Self::Balance>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        TradeIsExecuted {
            asset_in: T::AssetId,
            asset_out: T::AssetId,
            amount_in: T::Balance,
            amount_out: T::Balance,
        }
    }

    #[pallet::error]
    pub enum Error<T> {
        Limit,
        PoolNotSupported,
        PriceCalculationFailed,
        Execution,
        RouteHasNoTrades,
        InsufficientAssetBalance
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight((0, DispatchClass::Normal, Pays::No))]
        pub fn execute_sell(
            origin: OriginFor<T>,
            asset_in: T::AssetId,
            asset_out: T::AssetId,
            amount_in: T::Balance,
            limit: T::Balance,
            route: Vec<Trade<T::AssetId>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(route.len() > 0, Error::<T>::RouteHasNoTrades);
            ensure!(
                T::Currency::reducible_balance(asset_in, &who, false) >= amount_in,
                Error::<T>::InsufficientAssetBalance
            );

            let mut amounts = Vec::<T::Balance>::with_capacity(route.len() + 1);

            let mut amount = amount_in;

            amounts.push(amount);

            for trade in route.iter() {
                let result = T::AMM::calculate_sell(trade.pool, trade.asset_in, trade.asset_out, amount);

                match result {
                    Err(ExecutorError::NotSupported) => return Err(Error::<T>::PoolNotSupported.into()),
                    Err(ExecutorError::Error(_)) => return Err(Error::<T>::PriceCalculationFailed.into()),
                    Ok(r) => {
                        amount = r;
                        amounts.push(r);
                    }
                }
            }

            let last_amount = amounts.pop().ok_or(Error::<T>::Limit)?;
            ensure!(last_amount >= limit, Error::<T>::Limit);

            for (amount, trade) in amounts.iter().zip(route) {
                T::AMM::execute_sell(trade.pool, &who, trade.asset_in, trade.asset_out, *amount)
                    .map_err(|_| Error::<T>::Execution)?;
            }

            Self::deposit_event(Event::TradeIsExecuted {
                asset_in,
                asset_out,
                amount_in,
                amount_out: last_amount
            });
            // check asset out balance to verify that who receives at least last_amount

            Ok(())
        }
        #[pallet::weight((0, DispatchClass::Normal, Pays::No))]
        pub fn execute_buy(
            origin: OriginFor<T>,
            asset_in: T::AssetId,
            asset_out: T::AssetId,
            amount_out: T::Balance,
            limit: T::Balance,
            route: Vec<Trade<T::AssetId>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(route.len() > 0, Error::<T>::RouteHasNoTrades);

            ensure!(
                T::Currency::reducible_balance(asset_in, &who, false) >= amount_out,
                Error::<T>::InsufficientAssetBalance
            );

            let mut amounts = Vec::<T::Balance>::with_capacity(route.len() + 1);

            let mut amount = amount_out;

            amounts.push(amount);

            for trade in route.iter().rev() {
                let result = T::AMM::calculate_buy(trade.pool, trade.asset_in, trade.asset_out, amount);

                match result {
                    Err(ExecutorError::NotSupported) => return Err(Error::<T>::PoolNotSupported.into()),
                    Err(ExecutorError::Error(_)) => return Err(Error::<T>::PriceCalculationFailed.into()),
                    Ok(r) => {
                        amount = r;
                        amounts.push(r);
                    }
                }
            }

            let last_amount = amounts.last().ok_or(Error::<T>::Limit)?;
            ensure!(*last_amount <= limit, Error::<T>::Limit);

            for (amount, trade) in amounts.iter().rev().zip(route) {
                T::AMM::execute_sell(trade.pool, &who, trade.asset_in, trade.asset_out, *amount)
                    .map_err(|_| Error::<T>::Execution)?;
            }

            Self::deposit_event(Event::TradeIsExecuted {
                asset_in,
                asset_out,
                amount_in: *last_amount,
                amount_out
            });

            // check asset out balance to verify that who receives at least last_amount

            Ok(())
        }
    }
}
