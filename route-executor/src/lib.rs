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
use orml_traits::arithmetic::{CheckedSub, CheckedAdd};
use codec::{Decode, Encode};
use hydradx_traits::router::PoolType;
use scale_info::TypeInfo;

#[cfg(test)]
mod tests;

pub mod inspect;
pub mod weights;

use weights::WeightInfo;

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

///A single trade for buy/sell, describing the asset pair and the pool type in which the trade is executed
#[derive(Encode, Decode, Debug, Eq, PartialEq, Copy, Clone, TypeInfo)]
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

        /// Asset id type
        type AssetId: Parameter + Member + Copy + MaybeSerializeDeserialize;

        /// Balance type
        type Balance: Parameter + Member + Copy + PartialOrd + MaybeSerializeDeserialize + Default + CheckedSub + CheckedAdd;

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
        ///The trading limit has been reached
        TradingLimitReached,
        ///The the max number of trades limit is reached
        MaxTradesExceeded,
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
        /// - `min_amount_out`: The minimum amount of `asset_out` to receive.
        /// - `route`: Series of [`types::Trade<AssetId>`] containing AMM and asset pair information.
        ///
        /// Emits `RouteExecuted` when successful.
        #[pallet::weight(<T as Config>::WeightInfo::sell(route.len() as u32))]
        #[transactional]
        pub fn sell(
            origin: OriginFor<T>,
            asset_in: T::AssetId,
            asset_out: T::AssetId,
            amount_in: T::Balance,
            min_amount_out: T::Balance,
            route: Vec<Trade<T::AssetId>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin.clone())?;
            Self::ensure_route_size(route.len())?;

            let user_balance_of_asset_out_before_trade = T::Currency::reducible_balance(asset_out, &who, false);
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
            ensure!(last_amount >= min_amount_out, Error::<T>::TradingLimitReached);

            for (amount, trade) in amounts_to_sell.iter().zip(route) {
                let execution_result =
                    T::AMM::execute_sell(origin.clone(), trade.pool, trade.asset_in, trade.asset_out, *amount);

                handle_execution_error!(execution_result);
            }

            Self::ensure_that_user_received_asset_out(who, asset_out, user_balance_of_asset_out_before_trade, last_amount)?;

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
        /// - `max_amount_in`: The max amount of `asset_in` to spend on the buy.
        /// - `route`: Series of [`types::Trade<AssetId>`] containing AMM and asset pair information.
        ///
        /// Emits `RouteExecuted` when successful.
        #[pallet::weight(<T as Config>::WeightInfo::buy(route.len() as u32))]
        #[transactional]
        pub fn buy(
            origin: OriginFor<T>,
            asset_in: T::AssetId,
            asset_out: T::AssetId,
            amount_out: T::Balance,
            max_amount_in: T::Balance,
            route: Vec<Trade<T::AssetId>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin.clone())?;
            Self::ensure_route_size(route.len())?;

            let user_balance_of_asset_in_before_trade = T::Currency::reducible_balance(asset_in, &who, false);

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
            ensure!(last_amount <= max_amount_in, Error::<T>::TradingLimitReached);

            for (amount, trade) in amounts_to_buy.iter().rev().zip(route) {
                let execution_result =
                    T::AMM::execute_buy(origin.clone(), trade.pool, trade.asset_in, trade.asset_out, *amount);

                handle_execution_error!(execution_result);
            }

            Self::ensure_that_user_spent_asset_in(who, asset_in, user_balance_of_asset_in_before_trade, last_amount)?;

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
            Error::<T>::MaxTradesExceeded
        );

        Ok(())
    }

    fn ensure_that_user_received_asset_out(who: T::AccountId, asset_out: T::AssetId, user_balance_of_asset_out_before_trade : T::Balance, last_amount: T::Balance) -> Result<(), DispatchError> {
        let user_balance_of_asset_out_after_trade = T::Currency::reducible_balance(asset_out, &who, false);
        let user_expected_balance_of_asset_out_after_trade = user_balance_of_asset_out_before_trade
            .checked_add(&last_amount)
            .ok_or(Error::<T>::UnexpectedError)?;

        ensure!(
                user_balance_of_asset_out_after_trade == user_expected_balance_of_asset_out_after_trade,
                Error::<T>::UnexpectedError
            );

        Ok(())
    }

    fn ensure_that_user_spent_asset_in(who: T::AccountId, asset_in: T::AssetId, user_balance_of_asset_in_before_trade : T::Balance, last_amount: T::Balance) -> Result<(), DispatchError> {
        let user_balance_of_asset_in_after_trade = T::Currency::reducible_balance(asset_in, &who, false);
        let user_expected_balance_of_asset_in_after_trade = user_balance_of_asset_in_before_trade
            .checked_sub(&last_amount)
            .ok_or(Error::<T>::UnexpectedError)?;

        ensure!(
                user_expected_balance_of_asset_in_after_trade == user_balance_of_asset_in_after_trade,
                Error::<T>::UnexpectedError
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
