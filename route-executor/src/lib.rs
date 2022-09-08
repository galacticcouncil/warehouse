// This file is part of pallet-route-executor.

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

use frame_support::ensure;
use frame_support::traits::fungibles::Inspect;
use frame_support::traits::Get;
use frame_support::transactional;
use frame_system::ensure_signed;
use hydradx_traits::router::TradeExecution;
use sp_runtime::DispatchError;
use sp_std::vec::Vec;
pub mod types;

#[cfg(test)]
mod tests;

pub mod weights;
use weights::WeightInfo;

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::OriginFor;
    use hydradx_traits::router::ExecutorError;
    use types::Trade;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Asset id type
        type AssetId: Parameter + Member + Copy + MaybeSerializeDeserialize;

        /// Balance type
        type Balance: Parameter + Member + Copy + PartialOrd + MaybeSerializeDeserialize + Default;

        /// Max limit for the number of trades within a route
        #[pallet::constant]
        type MaxNumberOfTrades: Get<u8>;

        /// Currency for checking balances
        type Currency: Inspect<Self::AccountId, AssetId = Self::AssetId, Balance = Self::Balance>;

        /// Handlers for AMM pools to calculate and execute trades
        type AMM: TradeExecution<
            <Self as frame_system::Config>::Origin,
            Self::AccountId,
            Self::AssetId,
            Self::Balance,
            Error = DispatchError,
        >;

        /// Weight information for the extrinsics.
        type WeightInfo: WeightInfo;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        ///The route with trades has been successfully executed
        RouteExecuted {
            asset_in: T::AssetId,
            asset_out: T::AssetId,
            amount_in: T::Balance,
            amount_out: T::Balance,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        ///The minimum limit to receive after a sell is not reached
        MinLimitToReceiveNotReached,
        ///The maximum limit to spend on a buy is reached
        MaxLimitToSpendReached,
        ///The the max number of trades limit is reached
        MaxNumberOfTradesLimitReached,
        ///The AMM pool is not supported for executing trades
        PoolNotSupported,
        /// Route has not trades to be executed
        RouteHasNoTrades,
        ///The user has not enough balance to execute the trade
        InsufficientBalance,
        ///Unexpected error which should never really happen, but the error case must be handled to prevent panics.
        UnexpectedError,
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
        #[pallet::weight(<T as Config>::WeightInfo::execute_sell(route.len() as u32))]
        #[transactional]
        pub fn execute_sell(
            origin: OriginFor<T>,
            asset_in: T::AssetId,
            asset_out: T::AssetId,
            amount_in: T::Balance,
            limit: T::Balance,
            route: Vec<Trade<T::AssetId>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin.clone())?;
            Self::ensure_route_size(route.len())?;

            ensure!(
                T::Currency::reducible_balance(asset_in, &who, false) >= amount_in,
                Error::<T>::InsufficientBalance
            );

            let mut amounts_to_sell = Vec::<T::Balance>::with_capacity(route.len() + 1);
            let mut amount = amount_in;
            amounts_to_sell.push(amount);

            for trade in route.iter() {
                let result = T::AMM::calculate_sell(trade.pool, trade.asset_in, trade.asset_out, amount);
                match result {
                    Err(ExecutorError::NotSupported) => return Err(Error::<T>::PoolNotSupported.into()),
                    Err(ExecutorError::Error(dispatch_error)) => return Err(dispatch_error),
                    Ok(amount_to_sell) => {
                        amount = amount_to_sell;
                        amounts_to_sell.push(amount_to_sell);
                    }
                }
            }

            //We pop the last calculation amount as we use it only for verification and not for executing further trades
            let last_amount = amounts_to_sell.pop().ok_or(Error::<T>::UnexpectedError)?;
            ensure!(last_amount >= limit, Error::<T>::MinLimitToReceiveNotReached);

            for (amount, trade) in amounts_to_sell.iter().zip(route) {
                let execution_result =
                    T::AMM::execute_sell(trade.pool, &origin, trade.asset_in, trade.asset_out, *amount);

                handle_execution_error!(execution_result);
            }

            Self::deposit_event(Event::RouteExecuted {
                asset_in,
                asset_out,
                amount_in,
                amount_out: last_amount,
            });

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
        #[pallet::weight(<T as Config>::WeightInfo::execute_buy(route.len() as u32))]
        #[transactional]
        pub fn execute_buy(
            origin: OriginFor<T>,
            asset_in: T::AssetId,
            asset_out: T::AssetId,
            amount_out: T::Balance,
            limit: T::Balance,
            route: Vec<Trade<T::AssetId>>,
        ) -> DispatchResult {
            Self::ensure_route_size(route.len())?;

            let mut amounts_to_buy = Vec::<T::Balance>::with_capacity(route.len() + 1);
            let mut amount = amount_out;
            amounts_to_buy.push(amount);

            for trade in route.iter().rev() {
                let result = T::AMM::calculate_buy(trade.pool, trade.asset_in, trade.asset_out, amount);

                match result {
                    Err(ExecutorError::NotSupported) => return Err(Error::<T>::PoolNotSupported.into()),
                    Err(ExecutorError::Error(dispatch_error)) => return Err(dispatch_error),
                    Ok(amount_to_buy) => {
                        amount = amount_to_buy;
                        amounts_to_buy.push(amount_to_buy);
                    }
                }
            }

            //We pop the last calculation amount as we use it only for verification and not for executing further trades
            let last_amount = amounts_to_buy.pop().ok_or(Error::<T>::UnexpectedError)?;
            ensure!(last_amount <= limit, Error::<T>::MaxLimitToSpendReached);

            for (amount, trade) in amounts_to_buy.iter().rev().zip(route) {
                let execution_result =
                    T::AMM::execute_buy(trade.pool, &origin, trade.asset_in, trade.asset_out, *amount);

                handle_execution_error!(execution_result);
            }

            Self::deposit_event(Event::RouteExecuted {
                asset_in,
                asset_out,
                amount_in: last_amount,
                amount_out,
            });

            Ok(())
        }
    }
}

impl<T: Config> Pallet<T> {
    fn ensure_route_size(route_length: usize) -> Result<(), DispatchError> {
        ensure!(route_length > 0, Error::<T>::RouteHasNoTrades);
        ensure!(
            (route_length as u8) <= T::MaxNumberOfTrades::get(),
            Error::<T>::MaxNumberOfTradesLimitReached
        );

        Ok(())
    }
}

#[macro_export]
macro_rules! handle_execution_error {
    ($execution_result:expr) => {{
        if let Err(error) = $execution_result {
            return match error {
                ExecutorError::NotSupported => Err(Error::<T>::PoolNotSupported.into()),
                ExecutorError::Error(dispatch_error) => Err(dispatch_error),
            };
        }
    }};
}
