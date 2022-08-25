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
//----renaming main traits
//----simplify logic in lib.rs
//- XYK execute_sell map error in a better way, also in other
//- use UNITS in tests
//- benchmarking
//- TODO: Danis

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
        ///The route with trades has been successfully executed
        RouteIsExecuted {
            asset_in: T::AssetId,
            asset_out: T::AssetId,
            amount_in: T::Balance,
            amount_out: T::Balance,
        }
    }

    #[pallet::error]
    pub enum Error<T> {
        ///The minimum limit to receive after a sell is not reached
        MinLimitToReceiveIsNotReached,
        ///The maximum limit to spend on a buy is reached
        MaxLimitToSpendIsReached,
        ///The AMM pool is not supported for executing trades
        PoolIsNotSupported,
        /// The price calculation has failed in the AMM pool
        PriceCalculationIsFailed,
        /// The trade execution has failed in the AMM pool
        ExecutionIsFailed,
        /// Route has not trades to be executed
        RouteHasNoTrades,
        ///The user has not enough balance to execute the trade
        InsufficientAssetBalance,
        ///Unexpected error when retrieving the last trade calculation amount
        UnexpectedErrorWhenRetrievingLastTradeCalculationAmount
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {

        /// Executes a sell with a series of trades specified in the route.
        /// The price for each trade is determined by the corresponding AMM.
        ///
        /// - `origin`: The executor of the trade
        /// - `asset_in`: The identifier of the asset to sell
        /// - `asset_out`: The identifier of the asset to receive
        /// - `amount_in`: The amount of `asset_in` to sell
        /// - `limit`: The minimum amount of `asset_out` to receive.
        /// - `route`: Series of trades containing AMM and asset pair information
        ///
        /// Emits `RouteIsExecuted` when successful.
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
                    Err(ExecutorError::NotSupported) => return Err(Error::<T>::PoolIsNotSupported.into()),
                    Err(ExecutorError::Error(_)) => return Err(Error::<T>::PriceCalculationIsFailed.into()),
                    Ok(r) => {
                        amount = r;
                        amounts.push(r);
                    }
                }
            }

            let last_amount = amounts.pop().ok_or(Error::<T>::UnexpectedErrorWhenRetrievingLastTradeCalculationAmount)?;
            ensure!(last_amount >= limit, Error::<T>::MinLimitToReceiveIsNotReached);

            for (amount, trade) in amounts.iter().zip(route) {
                T::AMM::execute_sell(trade.pool, &who, trade.asset_in, trade.asset_out, *amount)
                    .map_err(|_| Error::<T>::ExecutionIsFailed)?;
            }

            Self::deposit_event(Event::RouteIsExecuted {
                asset_in,
                asset_out,
                amount_in,
                amount_out: last_amount
            });
            // check asset out balance to verify that who receives at least last_amount

            Ok(())
        }


        /// Executes a buy with a series of trades specified in the route.
        /// The price for each trade is determined by the corresponding AMM.
        ///
        /// - `origin`: The executor of the trade
        /// - `asset_in`: The identifier of the asset to be swapped to buy `asset_out`
        /// - `asset_out`: The identifier of the asset to buy
        /// - `amount_out`: The amount of `asset_out` to buy
        /// - `limit`: The max amount of `asset_in` to spend on the buy.
        /// - `route`: Series of trades containing AMM and asset pair info
        ///
        /// Emits `RouteIsExecuted` when successful.
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
                T::Currency::reducible_balance(asset_out, &who, false) >= amount_out,
                Error::<T>::InsufficientAssetBalance
            );

            let mut amounts = Vec::<T::Balance>::with_capacity(route.len() + 1);

            let mut amount = amount_out;

            amounts.push(amount);

            for trade in route.iter().rev() {
                let result = T::AMM::calculate_buy(trade.pool, trade.asset_in, trade.asset_out, amount);

                match result {
                    Err(ExecutorError::NotSupported) => return Err(Error::<T>::PoolIsNotSupported.into()),
                    Err(ExecutorError::Error(_)) => return Err(Error::<T>::PriceCalculationIsFailed.into()),
                    Ok(r) => {
                        amount = r;
                        amounts.push(r);
                    }
                }
            }

            let last_amount = amounts.pop().ok_or(Error::<T>::UnexpectedErrorWhenRetrievingLastTradeCalculationAmount)?;
            ensure!(last_amount <= limit, Error::<T>::MaxLimitToSpendIsReached);

            for (amount, trade) in amounts.iter().rev().zip(route) {
                T::AMM::execute_buy(trade.pool, &who, trade.asset_in, trade.asset_out, *amount)
                    .map_err(|_| Error::<T>::ExecutionIsFailed)?;
            }

            Self::deposit_event(Event::RouteIsExecuted {
                asset_in,
                asset_out,
                amount_in: last_amount,
                amount_out
            });

            // check asset out balance to verify that who receives at least last_amount

            Ok(())
        }
    }
}