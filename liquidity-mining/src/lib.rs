// This file is part of HydraDX

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
//
// Abbr:
//  rpvs - reward per valued share
//  rpz - reward per share in global pool

// Notion spec naming map:
// * shares                 -> s
// * total_shares           -> S
// * valued_shares          -> s'
// * total_valued_shares    -> S'
// * stake_in_global_pool   -> z
// * total_shares_z         -> Z
// * multiplier             -> m

//! # Liquidity mining pallet
//!
//! ## Overview
//!
//! This pallet provide functionality for liquidity mining program with time incentive(loyalty
//! factor).
//! Users are rewarded for each period they stay in liq. mining program.
//!
//! Reward per one period is derived from the user's loyalty factor which grows with time(periods)
//! the user is in the liq. mining and amount of LP shares user locked into deposit.
//! User's loyalty factor is reset if the user exits and reenters liquidity mining pool.
//! User can claim rewards without resetting loyalty factor, only withdrawing shares
//! is penalized by loyalty factor reset.
//! User is rewarded from the next period after he enters.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
mod types;

pub use pallet::*;

pub use crate::types::{
    Balance, Deposit, DepositId, GlobalPool, GlobalPoolId, LiquidityPoolYieldFarm, LoyaltyCurve, PoolId, PoolMultiplier,
};
use codec::{Decode, Encode, FullCodec};
use frame_support::{
    ensure,
    sp_runtime::traits::{BlockNumberProvider, MaybeSerializeDeserialize, One, Zero},
    transactional, PalletId,
};
use frame_support::{
    pallet_prelude::*,
    sp_runtime::{traits::AccountIdConversion, RuntimeDebug},
};

use hydra_dx_math::liquidity_mining as math;
use hydradx_traits::liquidity_mining::Handler;
use orml_traits::MultiCurrency;
use scale_info::TypeInfo;
use sp_arithmetic::{
    traits::{CheckedDiv, CheckedSub},
    FixedPointNumber, FixedU128, Permill,
};
use sp_std::convert::{From, Into, TryInto};

//This value is result of: u128::from_le_bytes([255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 0])
//This is necessary because first 4 bytes of DepositId (u128) is reserved to encode liq_pool_id (u32) into DepositId.
//For more details look at `get_next_deposit_id()`.
const MAX_DEPOSIT_SEQUENCER: u128 = 79_228_162_514_264_337_593_543_950_335;
//consts bellow are used to encode/decode liq. pool into/from DepositId.
const POOL_ID_BYTES: usize = 4;
const DEPOSIT_SEQUENCER_BYTES: usize = 12;

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
type AssetIdOf<T> = <T as pallet::Config>::CurrencyId;
type BlockNumberFor<T> = <T as frame_system::Config>::BlockNumber;
type PeriodOf<T> = <T as frame_system::Config>::BlockNumber;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_system::pallet_prelude::BlockNumberFor;

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::config]
    pub trait Config: frame_system::Config + TypeInfo {
        /// Asset type.
        type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord + From<u32>;

        /// Currency for transfers.
        type MultiCurrency: MultiCurrency<Self::AccountId, CurrencyId = Self::CurrencyId, Balance = Balance>;

        /// Pallet id.
        type PalletId: Get<PalletId>;

        /// Minimum total rewards to distribute from global pool during liquidity mining.
        type MinTotalFarmRewards: Get<Balance>;

        /// Minimum number of periods to run liquidity mining program.
        type MinPlannedYieldingPeriods: Get<Self::BlockNumber>;

        /// Mininum user's deposit to enter liquidity mining pool.
        type MinDeposit: Get<Balance>;

        /// The block number provider
        type BlockNumberProvider: BlockNumberProvider<BlockNumber = Self::BlockNumber>;

        /// Id used as a amm pool id key in the storage.
        type AmmPoolId: Parameter + Member + Clone + FullCodec;

        type Handler: hydradx_traits::liquidity_mining::Handler<
            Self::CurrencyId,
            Self::AmmPoolId,
            GlobalPoolId,
            PoolId,
            Balance,
            DepositId,
            Self::AccountId,
        >;
    }

    #[pallet::error]
    #[cfg_attr(test, derive(PartialEq))]
    pub enum Error<T> {
        /// Math computation overflow.
        Overflow,

        /// Farm does not exist.
        FarmNotFound,

        /// Liquidity pool yield farm does not exist.
        LiquidityPoolNotFound,

        /// Deposit does not exist.
        DepositNotFound,

        /// Multiple claims in the same period is not allowed.
        DoubleClaimInThePeriod,

        /// Liq. pool's metadata does not exist.
        LiquidityPoolMetadataNotFound,

        /// Pool's liquidity mining is canceled.
        LiquidityMiningCanceled,

        /// Pool's liquidity mining is not canceled.
        LiquidityMiningIsNotCanceled,

        /// LP tokens amount is not valid.
        InvalidDepositAmount,

        /// Account is not allowed to perform action.
        Forbidden,

        /// Pool multiplier can't be 0
        InvalidMultiplier,

        /// Liquidity pool already exist in the farm.
        LiquidityPoolAlreadyExists,

        /// Loyalty curve's initial reward percentage is not valid. Valid range is: [0, 1)
        InvalidInitialRewardPercentage,

        /// One or more liq. pools exist in farm.
        FarmIsNotEmpty,

        /// Farm's `incentivized_asset` is missing in provided asset pair.
        MissingIncentivizedAsset,

        /// Global pool rewards balance is not 0.
        RewardBalanceIsNotZero,

        /// Reward currency balance is not sufficient.
        InsufficientRewardCurrencyBalance,

        /// Blocks per period can't be 0.
        InvalidBlocksPerPeriod,

        /// Yield per period can't be 0.
        InvalidYieldPerPeriod,

        /// Total rewards is less than `MinTotalFarmRewards`.
        InvalidTotalRewards,

        /// Planned yielding periods is less than `MinPlannedYieldingPeriods`.
        InvalidPlannedYieldingPeriods,

        /// Insufficient reward currency in global pool.
        InsufficientBalanceInGlobalPool,

        /// Provided pool id is not valid. Valid range is [1, u32::MAX)
        InvalidPoolId,

        /// Max number of deposit id was reached.
        DepositIdOverflow,

        /// Deposit id is not valid.
        InvalidDepositId,
    }

    /// Id sequencer for `GlobalPool` and `LiquidityPoolYieldFarm`.
    #[pallet::storage]
    #[pallet::getter(fn pool_id)]
    pub type PoolIdSequencer<T: Config> = StorageValue<_, PoolId, ValueQuery>;

    /// Sequencer for last 12 bytes of deposit id.
    #[pallet::storage]
    pub type DepositSequencer<T: Config> = StorageValue<_, DepositId, ValueQuery>;

    /// Global pool details.
    #[pallet::storage]
    #[pallet::getter(fn global_pool)]
    pub type GlobalPoolData<T: Config> = StorageMap<_, Twox64Concat, GlobalPoolId, GlobalPool<T>, OptionQuery>;

    /// Liquidity pool yield farm details.
    #[pallet::storage]
    #[pallet::getter(fn liquidity_pool)]
    pub type LiquidityPoolData<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        GlobalPoolId,
        Twox64Concat,
        T::AmmPoolId,
        LiquidityPoolYieldFarm<T>,
        OptionQuery,
    >;

    /// Deposit details.
    #[pallet::storage]
    #[pallet::getter(fn deposit)]
    pub type DepositData<T: Config> = StorageMap<_, Twox64Concat, DepositId, Deposit<T>, OptionQuery>;

    /// `LiquidityPoolYieldFarm` metadata holding: `(existing deposits count, global pool id)`
    #[pallet::storage]
    #[pallet::getter(fn liq_pool_meta)]
    pub type LiquidityPoolMetadata<T: Config> = StorageMap<_, Twox64Concat, PoolId, (u64, GlobalPoolId), OptionQuery>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {}
}

impl<T: Config> Pallet<T> {
    /// Create new liquidity mining program with provided parameters.
    ///
    /// `owner` account have to have at least `total_rewards` balance. This funds will be
    /// transferred from `owner` to farm account.
    ///
    /// Returns: `(global pool id, max reward per period)`
    ///
    /// Parameters:
    /// - `total_rewards`: total rewards planned to distribute. This rewards will be
    /// distributed between all liq. pools in liq. mining program.
    /// - `planned_yielding_periods`: planned number of periods to distribute `total_rewards`.
    /// WARN: THIS IS NOT HARD DEADLINE. Not all rewards have to be distributed in
    /// `planned_yielding_periods`. Rewards are distributed based on the situation in the liq.
    /// pools and can be distributed in a longer time frame but never in the shorter time frame.
    /// - `blocks_per_period`:  number of blocks in a single period. Min. number of blocks per
    /// period is 1.
    /// - `incentivized_asset`: asset to be incentivized in AMM pools. All liq. pools added into
    /// liq. mining program have to have `incentivized_asset` in their pair.
    /// - `reward_currency`: payoff currency of rewards.
    /// - `owner`: liq. mining farm owner.
    /// - `yield_per_period`: percentage return on `reward_currency` of all pools
    #[allow(clippy::too_many_arguments)]
    #[transactional]
    pub fn create_farm(
        total_rewards: Balance,
        planned_yielding_periods: PeriodOf<T>,
        blocks_per_period: BlockNumberFor<T>,
        incentivized_asset: AssetIdOf<T>,
        reward_currency: AssetIdOf<T>,
        owner: AccountIdOf<T>,
        yield_per_period: Permill,
    ) -> Result<(GlobalPoolId, Balance), DispatchError> {
        Self::validate_create_farm_data(
            total_rewards,
            planned_yielding_periods,
            blocks_per_period,
            yield_per_period,
        )?;

        ensure!(
            T::MultiCurrency::free_balance(reward_currency, &owner) >= total_rewards,
            Error::<T>::InsufficientRewardCurrencyBalance
        );

        let planned_periods = TryInto::<u128>::try_into(planned_yielding_periods).map_err(|_e| Error::<T>::Overflow)?;
        let max_reward_per_period = total_rewards.checked_div(planned_periods).ok_or(Error::<T>::Overflow)?;
        let now_period = Self::get_now_period(blocks_per_period)?;
        let pool_id = Self::get_next_pool_id()?;

        let global_pool = GlobalPool::new(
            pool_id,
            now_period,
            reward_currency,
            yield_per_period,
            planned_yielding_periods,
            blocks_per_period,
            owner,
            incentivized_asset,
            max_reward_per_period,
        );

        <GlobalPoolData<T>>::insert(&global_pool.id, &global_pool);

        let global_pool_account = Self::pool_account_id(global_pool.id)?;
        T::MultiCurrency::transfer(reward_currency, &global_pool.owner, &global_pool_account, total_rewards)?;

        Ok((pool_id, max_reward_per_period))
    }

    /// Destroy existing liq. mining program.
    ///
    /// Only farm owner can perform this action.
    ///
    /// WARN: To successfully destroy a farm, farm have to be empty(all liq. pools have to be
    /// removed from the farm) and all undistributed rewards have to be withdrawn.
    ///
    /// Parameters:
    /// - `who`: farm's owner.
    /// - `farm_id`: id of farm to be destroyed.
    #[transactional]
    pub fn destroy_farm(who: AccountIdOf<T>, farm_id: GlobalPoolId) -> DispatchResult {
        <GlobalPoolData<T>>::try_mutate_exists(farm_id, |maybe_global_pool| -> DispatchResult {
            let global_pool = maybe_global_pool.as_ref().ok_or(Error::<T>::FarmNotFound)?;

            ensure!(who == global_pool.owner, Error::<T>::Forbidden);

            ensure!(global_pool.liq_pools_count.is_zero(), Error::<T>::FarmIsNotEmpty);

            let global_pool_account = Self::pool_account_id(global_pool.id)?;
            ensure!(
                T::MultiCurrency::free_balance(global_pool.reward_currency, &global_pool_account).is_zero(),
                Error::<T>::RewardBalanceIsNotZero
            );

            *maybe_global_pool = None;

            Ok(())
        })
    }

    /// Transfer all rewards left to distribute from farm account to farm's `owner` account.
    ///  
    /// Only farm owner can perform this action.
    ///
    /// WARN: Farm have to be empty(all liq. pools have to be removed from the farm) to
    /// successfully withdraw rewards left to distribute from the farm.
    ///
    /// Returns: `(reward currency, withdrawn amount)`;
    ///
    /// Parameters:
    /// - `who`: farm's owner.
    /// - `farm_id`: id of farm to be destroyed.
    #[transactional]
    pub fn withdraw_undistributed_rewards(
        who: AccountIdOf<T>,
        farm_id: GlobalPoolId,
    ) -> Result<(T::CurrencyId, Balance), DispatchError> {
        let global_pool = Self::global_pool(farm_id).ok_or(Error::<T>::FarmNotFound)?;

        ensure!(global_pool.owner == who, Error::<T>::Forbidden);

        ensure!(global_pool.liq_pools_count.is_zero(), Error::<T>::FarmIsNotEmpty);

        let global_pool_account = Self::pool_account_id(global_pool.id)?;

        let undistributed_reward = T::MultiCurrency::total_balance(global_pool.reward_currency, &global_pool_account);

        T::MultiCurrency::transfer(
            global_pool.reward_currency,
            &global_pool_account,
            &who,
            undistributed_reward,
        )?;

        Ok((global_pool.reward_currency, undistributed_reward))
    }

    /// Add liquidity pool to farm and allow yield farming for given assets pair amm.
    ///  
    /// Only farm owner can perform this action.
    ///
    /// One of the AMM assets HAVE TO be `incentivized_token`. Same AMM can be
    /// in the same farm only once.
    ///
    /// Returns: `(new liq. pool yield farm id)`
    ///
    /// Parameters:
    /// - `who`: farm's owner
    /// - `farm_id`: farm id to which a liq. pool will be added.
    /// - `multiplier`: liq. pool multiplier in the farm.
    /// - `loyalty_curve`: curve to calculate loyalty multiplier to distribute rewards to users
    /// with time incentive. `None` means no loyalty multiplier.
    /// - `amm_pool_id`: identifier of the AMM. It's used as a key in the storage.
    /// - `asset_a`: one of the assets in the AMM.
    /// - `asset_b`: second asset in the AMM.
    #[transactional]
    pub fn add_liquidity_pool(
        who: AccountIdOf<T>,
        farm_id: GlobalPoolId,
        multiplier: PoolMultiplier,
        loyalty_curve: Option<LoyaltyCurve>,
        amm_pool_id: T::AmmPoolId,
        asset_a: T::CurrencyId,
        asset_b: T::CurrencyId,
    ) -> Result<PoolId, DispatchError> {
        ensure!(!multiplier.is_zero(), Error::<T>::InvalidMultiplier);

        if let Some(ref curve) = loyalty_curve {
            ensure!(
                curve.initial_reward_percentage.lt(&FixedU128::one()),
                Error::<T>::InvalidInitialRewardPercentage
            );
        }

        <GlobalPoolData<T>>::try_mutate(farm_id, |maybe_pool| -> Result<PoolId, DispatchError> {
            let global_pool = maybe_pool.as_mut().ok_or(Error::<T>::FarmNotFound)?;

            ensure!(who == global_pool.owner, Error::<T>::Forbidden);

            ensure!(
                asset_a == global_pool.incentivized_asset || asset_b == global_pool.incentivized_asset,
                Error::<T>::MissingIncentivizedAsset
            );

            ensure!(
                !<LiquidityPoolData<T>>::contains_key(farm_id, &amm_pool_id),
                Error::<T>::LiquidityPoolAlreadyExists
            );

            // update  global pool accumulated RPZ
            let now_period = Self::get_now_period(global_pool.blocks_per_period)?;
            if !global_pool.total_shares_z.is_zero() && global_pool.updated_at != now_period {
                let reward_per_period = math::calculate_global_pool_reward_per_period(
                    global_pool.yield_per_period.into(),
                    global_pool.total_shares_z,
                    global_pool.max_reward_per_period,
                )
                .map_err(|_e| Error::<T>::Overflow)?;
                Self::update_global_pool(global_pool, now_period, reward_per_period)?;
            }

            let liq_pool_id = Self::get_next_pool_id()?;
            <LiquidityPoolMetadata<T>>::insert(liq_pool_id, (0, global_pool.id));

            let pool = LiquidityPoolYieldFarm::new(liq_pool_id, now_period, loyalty_curve.clone(), multiplier);

            <LiquidityPoolData<T>>::insert(global_pool.id, &amm_pool_id, &pool);
            global_pool.liq_pools_count = global_pool.liq_pools_count.checked_add(1).ok_or(Error::<T>::Overflow)?;

            Ok(liq_pool_id)
        })
    }

    /// Update liquidity pool multiplier.
    ///  
    /// Only farm owner can perform this action.
    ///
    /// Returns: `(liq. pool yield farm id of updated farm)`
    ///
    /// Parameters:
    /// - `who`: farm's owner
    /// - `farm_id`: farm id in which liq. pool will be updated.
    /// - `asset_pair`: asset pair identifying liq. pool in farm.
    /// - `multiplier`: new liq. pool multiplier in the farm.
    /// - `amm_pool_id`: identifier of the AMM.
    #[transactional]
    pub fn update_liquidity_pool(
        who: AccountIdOf<T>,
        farm_id: GlobalPoolId,
        multiplier: PoolMultiplier,
        amm_pool_id: T::AmmPoolId,
    ) -> Result<PoolId, DispatchError> {
        ensure!(!multiplier.is_zero(), Error::<T>::InvalidMultiplier);

        <LiquidityPoolData<T>>::try_mutate(farm_id, &amm_pool_id, |liq_pool| {
            let liq_pool = liq_pool.as_mut().ok_or(Error::<T>::LiquidityPoolNotFound)?;

            ensure!(!liq_pool.canceled, Error::<T>::LiquidityMiningCanceled);

            <GlobalPoolData<T>>::try_mutate(farm_id, |maybe_global_pool| {
                let global_pool = maybe_global_pool.as_mut().ok_or(Error::<T>::FarmNotFound)?;

                ensure!(who == global_pool.owner, Error::<T>::Forbidden);

                let old_stake_in_global_pool =
                    math::calculate_global_pool_shares(liq_pool.total_valued_shares, liq_pool.multiplier)
                        .map_err(|_e| Error::<T>::Overflow)?;

                let now_period = Self::get_now_period(global_pool.blocks_per_period)?;
                Self::maybe_update_pools(global_pool, liq_pool, now_period)?;

                let new_stake_in_global_pool =
                    math::calculate_global_pool_shares(liq_pool.total_valued_shares, multiplier)
                        .map_err(|_e| Error::<T>::Overflow)?;

                global_pool.total_shares_z = global_pool
                    .total_shares_z
                    .checked_sub(old_stake_in_global_pool)
                    .ok_or(Error::<T>::Overflow)?
                    .checked_add(new_stake_in_global_pool)
                    .ok_or(Error::<T>::Overflow)?;

                liq_pool.multiplier = multiplier;

                Ok(liq_pool.id)
            })
        })
    }

    /// Cancel liq. miming for specific liq. pool.
    ///
    /// This function claims rewards from `GlobalPool` last time and stops liq. pool
    /// incentivization from a `GlobalPool`. Users will be able to only claim and withdraw
    /// shares after calling this function.
    /// `deposit_shares()` is not allowed on canceled liq. pool.
    ///  
    /// Only farm owner can perform this action.
    ///
    /// Returns: `(liq. pool yield farm id of canceled farm)`
    ///
    /// Parameters:
    /// - `who`: farm's owner.
    /// - `farm_id`: farm id in which liq. pool will be canceled.
    /// - `amm_pool_id`: identifier of the AMM pool.
    #[transactional]
    pub fn cancel_liquidity_pool(
        who: AccountIdOf<T>,
        farm_id: GlobalPoolId,
        amm_pool_id: T::AmmPoolId,
    ) -> Result<PoolId, DispatchError> {
        <LiquidityPoolData<T>>::try_mutate(farm_id, amm_pool_id, |maybe_liq_pool| {
            let liq_pool = maybe_liq_pool.as_mut().ok_or(Error::<T>::LiquidityPoolNotFound)?;

            ensure!(!liq_pool.canceled, Error::<T>::LiquidityMiningCanceled);

            <GlobalPoolData<T>>::try_mutate(farm_id, |maybe_global_pool| {
                let global_pool = maybe_global_pool.as_mut().ok_or(Error::<T>::FarmNotFound)?;

                ensure!(global_pool.owner == who, Error::<T>::Forbidden);

                let now_period = Self::get_now_period(global_pool.blocks_per_period)?;
                Self::maybe_update_pools(global_pool, liq_pool, now_period)?;

                let old_stake_in_global_pool =
                    math::calculate_global_pool_shares(liq_pool.total_valued_shares, liq_pool.multiplier)
                        .map_err(|_e| Error::<T>::Overflow)?;

                global_pool.total_shares_z = global_pool
                    .total_shares_z
                    .checked_sub(old_stake_in_global_pool)
                    .ok_or(Error::<T>::Overflow)?;

                liq_pool.canceled = true;
                liq_pool.multiplier = 0.into();

                Ok(liq_pool.id)
            })
        })
    }

    /// Resume liq. miming for canceled liq. pool.
    ///
    /// This function resume incentivization from `GlobalPool` and restore full functionality
    /// for liq. pool. Users will be able to deposit, claim and withdraw again.
    ///
    /// WARN: Liq. pool is NOT rewarded for time it was canceled.
    ///
    /// Only farm owner can perform this action.
    ///
    /// Returns: `(liq pool yield farm id of resumed farm)`
    ///
    /// Parameters:
    /// - `who`: farm's owner
    /// - `farm_id`: farm id in which liq. pool will be resumed.
    /// - `multiplier`: liq. pool multiplier in the farm.
    /// - `amm_pool_id`: indentifier of the AMM pool.
    #[transactional]
    pub fn resume_liquidity_pool(
        who: AccountIdOf<T>,
        farm_id: GlobalPoolId,
        multiplier: PoolMultiplier,
        amm_pool_id: T::AmmPoolId,
    ) -> Result<PoolId, DispatchError> {
        ensure!(!multiplier.is_zero(), Error::<T>::InvalidMultiplier);

        <LiquidityPoolData<T>>::try_mutate(farm_id, amm_pool_id, |maybe_liq_pool| {
            let liq_pool = maybe_liq_pool.as_mut().ok_or(Error::<T>::LiquidityPoolNotFound)?;

            ensure!(liq_pool.canceled, Error::<T>::LiquidityMiningIsNotCanceled);

            <GlobalPoolData<T>>::try_mutate(farm_id, |maybe_global_pool| {
                // this should never happen, liq. pool can't exist without global_pool
                let global_pool = maybe_global_pool.as_mut().ok_or(Error::<T>::FarmNotFound)?;

                ensure!(global_pool.owner == who, Error::<T>::Forbidden);

                //update `GlobalPool` accumulated_rpz
                let now_period = Self::get_now_period(global_pool.blocks_per_period)?;
                if !global_pool.total_shares_z.is_zero() && global_pool.updated_at != now_period {
                    let reward_per_period = math::calculate_global_pool_reward_per_period(
                        global_pool.yield_per_period.into(),
                        global_pool.total_shares_z,
                        global_pool.max_reward_per_period,
                    )
                    .map_err(|_e| Error::<T>::Overflow)?;
                    Self::update_global_pool(global_pool, now_period, reward_per_period)?;
                }

                let new_stake_in_global_poll =
                    math::calculate_global_pool_shares(liq_pool.total_valued_shares, multiplier)
                        .map_err(|_e| Error::<T>::Overflow)?;

                global_pool.total_shares_z = global_pool
                    .total_shares_z
                    .checked_add(new_stake_in_global_poll)
                    .ok_or(Error::<T>::Overflow)?;

                liq_pool.accumulated_rpz = global_pool.accumulated_rpz;
                liq_pool.updated_at = now_period;
                liq_pool.canceled = false;
                liq_pool.multiplier = multiplier;

                Ok(liq_pool.id)
            })
        })
    }

    /// Remove liq. pool for a farm.
    ///
    /// This function remove liq. pool from the farm and also from storage. Users will be able to
    /// only withdraw shares(without claiming rewards from liq. mining). Unpaid rewards will be
    /// transferred back to farm(`GlobalPool`) account and will be used to distribute to other
    /// liq. pools in the farm.
    ///
    /// Liq. pool must be canceled before calling this function.
    ///
    /// Only farm owner can perform this action.
    ///
    /// Returns: `(liq. pool yield farm id of removed farm)`
    ///
    /// Parameters:
    /// - `who`: farm's owner.
    /// - `farm_id`: farm id from which liq. pool should be removed.
    /// - `asset_pair`: asset pair identifying liq. pool in the farm.
    /// - `amm_pool_id`: indentifier of the AMM pool.
    #[transactional]
    pub fn remove_liquidity_pool(
        who: AccountIdOf<T>,
        farm_id: GlobalPoolId,
        amm_pool_id: T::AmmPoolId,
    ) -> Result<PoolId, DispatchError> {
        <LiquidityPoolData<T>>::try_mutate_exists(farm_id, amm_pool_id, |maybe_liq_pool| {
            let liq_pool = maybe_liq_pool.as_mut().ok_or(Error::<T>::LiquidityPoolNotFound)?;

            ensure!(liq_pool.canceled, Error::<T>::LiquidityMiningIsNotCanceled);

            <GlobalPoolData<T>>::try_mutate(farm_id, |maybe_global_pool| -> Result<(), DispatchError> {
                let global_pool = maybe_global_pool.as_mut().ok_or(Error::<T>::FarmNotFound)?;

                ensure!(global_pool.owner == who, Error::<T>::Forbidden);

                global_pool.liq_pools_count = global_pool.liq_pools_count.checked_sub(1).ok_or(Error::<T>::Overflow)?;

                //transfer unpaid rewards back to global_pool
                let global_pool_account = Self::pool_account_id(global_pool.id)?;
                let liq_pool_account = Self::pool_account_id(liq_pool.id)?;

                let unpaid_reward = T::MultiCurrency::total_balance(global_pool.reward_currency, &liq_pool_account);
                T::MultiCurrency::transfer(
                    global_pool.reward_currency,
                    &liq_pool_account,
                    &global_pool_account,
                    unpaid_reward,
                )?;

                if let Some((deposits_count, _)) = Self::liq_pool_meta(liq_pool.id) {
                    if deposits_count.is_zero() {
                        <LiquidityPoolMetadata<T>>::remove(liq_pool.id);
                    }
                };

                Ok(())
            })?;

            let liq_pool_id = liq_pool.id;
            *maybe_liq_pool = None;

            Ok(liq_pool_id)
        })
    }

    /// Deposit LP shares to a liq. mining.
    ///
    /// This function create deposits in the lquidity pool yeild farm.
    ///
    /// Returns: `(liq. pool yield farm id to whitch LP shares was deposited, deposit id)`
    ///
    /// Parameters:
    /// - `farm_id`: id of farm to which user want to deposit LP shares.
    /// - `shares_amount`: amount of LP shares user want to deposit.
    /// - `amm_pool_id`: identifier of the AMM pool.
    #[transactional]
    pub fn deposit_shares(
        who: AccountIdOf<T>,
        farm_id: GlobalPoolId,
        shares_amount: Balance,
        amm_pool_id: T::AmmPoolId,
    ) -> Result<(PoolId, DepositId), DispatchError> {
        ensure!(
            shares_amount.ge(&T::MinDeposit::get()),
            Error::<T>::InvalidDepositAmount,
        );

        <LiquidityPoolData<T>>::try_mutate(farm_id, amm_pool_id.clone(), |liq_pool| {
            let liq_pool = liq_pool.as_mut().ok_or(Error::<T>::LiquidityPoolNotFound)?;

            ensure!(!liq_pool.canceled, Error::<T>::LiquidityMiningCanceled);

            <GlobalPoolData<T>>::try_mutate(farm_id, |maybe_global_pool| {
                //something is very wrong if this fail, liq_pool can't exist without global_pool
                let global_pool = maybe_global_pool.as_mut().ok_or(Error::<T>::FarmNotFound)?;

                let now_period = Self::get_now_period(global_pool.blocks_per_period)?;

                Self::maybe_update_pools(global_pool, liq_pool, now_period)?;

                let valued_shares =
                    Self::get_valued_shares(shares_amount, amm_pool_id.clone(), global_pool.incentivized_asset)?;
                let shares_in_global_pool_for_deposit =
                    math::calculate_global_pool_shares(valued_shares, liq_pool.multiplier)
                        .map_err(|_e| Error::<T>::Overflow)?;

                liq_pool.total_shares = liq_pool
                    .total_shares
                    .checked_add(shares_amount)
                    .ok_or(Error::<T>::Overflow)?;

                liq_pool.total_valued_shares = liq_pool
                    .total_valued_shares
                    .checked_add(valued_shares)
                    .ok_or(Error::<T>::Overflow)?;

                global_pool.total_shares_z = global_pool
                    .total_shares_z
                    .checked_add(shares_in_global_pool_for_deposit)
                    .ok_or(Error::<T>::Overflow)?;

                let deposit_id = Self::get_next_deposit_id(liq_pool.id)?;

                let deposit = Deposit::new(shares_amount, valued_shares, liq_pool.accumulated_rpvs, now_period);
                <DepositData<T>>::insert(&deposit_id, deposit);

                <LiquidityPoolMetadata<T>>::try_mutate(liq_pool.id, |maybe_liq_pool_metadata| {
                    //Something is very wrong if this fail. Metadata can exist without liq. pool but liq. pool can't
                    //exist without metadata.
                    let liq_pool_metadata = maybe_liq_pool_metadata
                        .as_mut()
                        .ok_or(Error::<T>::LiquidityPoolMetadataNotFound)?;

                    //Increment deposits count
                    liq_pool_metadata.0 = liq_pool_metadata.0.checked_add(1).ok_or(Error::<T>::Overflow)?;

                    T::Handler::lock_lp_tokens(amm_pool_id, who, shares_amount, deposit_id)?;
                    Ok((liq_pool.id, deposit_id))
                })
            })
        })
    }

    /// Claim rewards from liq. mining for given deposit.
    ///
    /// This function calculate user rewards from liq. mining and transfer rewards to `who`
    /// account. Claiming in the same period is allowed only once.
    ///
    /// WARN: User have to use `withdraw_shares()` if liq. pool is removed or whole
    /// farm is destroyed.
    ///
    /// Returns: `(global pool id, liq. pool yield farm id, claimed amount, unclaimable amount)`
    /// unclaimable rewards is usefull for `withdraw_shares()` - this value is applied only when
    /// user exit liq. mining program.
    ///
    /// Parameters:
    /// - `who`: destination account to receive rewards.
    /// - `deposit_id`: id representing deposit in the liq. pool.
    /// - `amm_pool_id`: identifier of the AMM pool.
    /// - `check_double_claim`: fn failed on double claim if this is set to `true`. `fasle` is
    /// usefull for `withdraw_shares()` where we need `unclaimable_rewards` from this fn.
    #[transactional]
    pub fn claim_rewards(
        who: AccountIdOf<T>,
        deposit_id: DepositId,
        amm_pool_id: T::AmmPoolId,
        check_double_claim: bool,
    ) -> Result<(GlobalPoolId, PoolId, T::CurrencyId, Balance, Balance), DispatchError> {
        let liq_pool_id = Self::get_pool_id_from_deposit_id(deposit_id)?;

        //This is same as liq. pool not found in this case. Liq. pool metadata CAN exist
        //without liq. pool but liq. pool CAN'T exist without metadata.
        let (_, farm_id) = <LiquidityPoolMetadata<T>>::get(liq_pool_id).ok_or(Error::<T>::LiquidityPoolNotFound)?;

        <DepositData<T>>::try_mutate(deposit_id, |maybe_deposit| {
            let deposit = maybe_deposit.as_mut().ok_or(Error::<T>::DepositNotFound)?;

            <LiquidityPoolData<T>>::try_mutate(farm_id, amm_pool_id, |maybe_liq_pool| {
                let liq_pool = maybe_liq_pool.as_mut().ok_or(Error::<T>::LiquidityPoolNotFound)?;

                <GlobalPoolData<T>>::try_mutate(farm_id, |maybe_global_pool| {
                    //Something is very wrong if this fail. Liq. pool can't exist without GlobalPool.
                    let global_pool = maybe_global_pool.as_mut().ok_or(Error::<T>::FarmNotFound)?;

                    let now_period = Self::get_now_period(global_pool.blocks_per_period)?;
                    //Double claim should be allowed in some case e.g for withdraw_shares we need
                    //`unclaimable_rewards` returned by this function.
                    if check_double_claim {
                        ensure!(deposit.updated_at != now_period, Error::<T>::DoubleClaimInThePeriod);
                    }

                    Self::maybe_update_pools(global_pool, liq_pool, now_period)?;

                    let periods = now_period
                        .checked_sub(&deposit.entered_at)
                        .ok_or(Error::<T>::Overflow)?;

                    let loyalty_multiplier = Self::get_loyalty_multiplier(periods, liq_pool.loyalty_curve.clone())?;

                    let (rewards, unclaimable_rewards) = math::calculate_user_reward(
                        deposit.accumulated_rpvs,
                        deposit.valued_shares,
                        deposit.accumulated_claimed_rewards,
                        liq_pool.accumulated_rpvs,
                        loyalty_multiplier,
                    )
                    .map_err(|_e| Error::<T>::Overflow)?;

                    if !rewards.is_zero() {
                        deposit.accumulated_claimed_rewards = deposit
                            .accumulated_claimed_rewards
                            .checked_add(rewards)
                            .ok_or(Error::<T>::Overflow)?;

                        deposit.updated_at = now_period;

                        let liq_pool_account = Self::pool_account_id(liq_pool.id)?;
                        T::MultiCurrency::transfer(global_pool.reward_currency, &liq_pool_account, &who, rewards)?;
                    }
                    Ok((
                        global_pool.id,
                        liq_pool.id,
                        global_pool.reward_currency,
                        rewards,
                        unclaimable_rewards,
                    ))
                })
            })
        })
    }

    /// Withdraw LP shares from liq. mining.
    ///
    /// This function transfer user's unclaimable rewards back to global pool's account.
    ///
    /// Returns: `(global pool id, liq. pool yield farm id, withdrawn amount)`
    ///
    /// Parameters:
    /// - `who`: account to which LP shares should be transfered.
    /// - `deposit_id`: id representing deposit in the liq. pool.
    /// - `amm_pool_id`: identifier of the AMM pool.
    /// - `unclaimable_rewards`: amount of reward will be not claimed anymore. This amount is
    /// transfered from `LiquidityPoolYieldFarm` account to `GlobalPool` account.
    #[transactional]
    pub fn withdraw_shares(
        who: AccountIdOf<T>,
        deposit_id: DepositId,
        amm_pool_id: T::AmmPoolId,
        unclaimable_rewards: Balance,
    ) -> Result<(GlobalPoolId, PoolId, Balance), DispatchError> {
        let liq_pool_id = Self::get_pool_id_from_deposit_id(deposit_id)?;

        <LiquidityPoolMetadata<T>>::try_mutate_exists(liq_pool_id, |maybe_liq_pool_metadata| {
            //This is same as liq pool not found in this case. Liq. pool metadata CAN exist
            //without liq. pool but liq. pool CAN'T exist without metadata.
            //If metadata doesn't exist, the user CAN'T withdraw.
            let (deposits_count, farm_id) = maybe_liq_pool_metadata.ok_or(Error::<T>::LiquidityPoolNotFound)?;

            <DepositData<T>>::try_mutate_exists(deposit_id, |maybe_deposit| {
                let deposit = maybe_deposit.as_mut().ok_or(Error::<T>::DepositNotFound)?;

                //Metadata can be removed only if the liq. pool doesn't exist.
                //Liq. pool can be resumed if it's only canceled.
                let mut can_remove_liq_pool_metadata = false;
                <LiquidityPoolData<T>>::try_mutate(
                    farm_id,
                    amm_pool_id.clone(),
                    |maybe_liq_pool| -> Result<(), DispatchError> {
                        if maybe_liq_pool.is_some() {
                            //This is intentional. This fn should not fail if liq. pool does not
                            //exist, it should only behave differently.
                            let liq_pool = maybe_liq_pool.as_mut().ok_or(Error::<T>::LiquidityPoolNotFound)?;

                            <GlobalPoolData<T>>::try_mutate(
                                farm_id,
                                |maybe_global_pool| -> Result<(), DispatchError> {
                                    //This should never happen. If this happen something is very broken.
                                    let global_pool = maybe_global_pool.as_mut().ok_or(Error::<T>::FarmNotFound)?;

                                    liq_pool.total_shares = liq_pool
                                        .total_shares
                                        .checked_sub(deposit.shares)
                                        .ok_or(Error::<T>::Overflow)?;

                                    liq_pool.total_valued_shares = liq_pool
                                        .total_valued_shares
                                        .checked_sub(deposit.valued_shares)
                                        .ok_or(Error::<T>::Overflow)?;

                                    // liq. pool farm's stake in global pool is set 0 when farm is
                                    // canceled
                                    if !liq_pool.canceled {
                                        let shares_in_global_pool_for_deposit = math::calculate_global_pool_shares(
                                            deposit.valued_shares,
                                            liq_pool.multiplier,
                                        )
                                        .map_err(|_e| Error::<T>::Overflow)?;

                                        global_pool.total_shares_z = global_pool
                                            .total_shares_z
                                            .checked_sub(shares_in_global_pool_for_deposit)
                                            .ok_or(Error::<T>::Overflow)?;
                                    }

                                    if !unclaimable_rewards.is_zero() {
                                        let global_pool_account = Self::pool_account_id(global_pool.id)?;
                                        let liq_pool_account = Self::pool_account_id(liq_pool.id)?;

                                        T::MultiCurrency::transfer(
                                            global_pool.reward_currency,
                                            &liq_pool_account,
                                            &global_pool_account,
                                            unclaimable_rewards,
                                        )?;
                                    }

                                    Ok(())
                                },
                            )?;
                        } else {
                            //Canceled liq. pool can be resumed so metadata can be removed only
                            //if liq pool doesn't exist.
                            can_remove_liq_pool_metadata = true;
                        }
                        Ok(())
                    },
                )?;

                //backup value before cleanup
                let withdrawn_amount = deposit.shares;

                //cleanup
                *maybe_deposit = None;

                //Last withdrawn from removed liq. pool should destroy metadata.
                if deposits_count.is_one() && can_remove_liq_pool_metadata {
                    *maybe_liq_pool_metadata = None;
                } else {
                    *maybe_liq_pool_metadata =
                        Some((deposits_count.checked_sub(1).ok_or(Error::<T>::Overflow)?, farm_id));
                }

                T::Handler::unlock_lp_tokens(amm_pool_id, who, withdrawn_amount, deposit_id)?;

                Ok((farm_id, liq_pool_id, withdrawn_amount))
            })
        })
    }

    /// This function return new unused `PoolId` usable for liq. or global pool or error.
    fn get_next_pool_id() -> Result<PoolId, Error<T>> {
        PoolIdSequencer::<T>::try_mutate(|current_id| {
            *current_id = current_id.checked_add(1).ok_or(Error::<T>::Overflow)?;

            Ok(*current_id)
        })
    }

    /// This function return new unused `DepositId` with encoded `liq_pool_id` into it or
    /// error.
    ///
    /// 4 most significant bytes of `DepositId` are reserved for liq. pool id(`u32`).
    fn get_next_deposit_id(liq_pool_id: PoolId) -> Result<DepositId, Error<T>> {
        Self::validate_pool_id(liq_pool_id)?;

        DepositSequencer::<T>::try_mutate(|current_id| {
            *current_id = current_id.checked_add(1).ok_or(Error::<T>::Overflow)?;

            ensure!(MAX_DEPOSIT_SEQUENCER.ge(current_id), Error::<T>::DepositIdOverflow);

            let mut id_bytes: [u8; POOL_ID_BYTES + DEPOSIT_SEQUENCER_BYTES] =
                [0; POOL_ID_BYTES + DEPOSIT_SEQUENCER_BYTES];

            id_bytes[..POOL_ID_BYTES].copy_from_slice(&liq_pool_id.to_le_bytes());
            id_bytes[POOL_ID_BYTES..].copy_from_slice(&current_id.to_le_bytes()[..DEPOSIT_SEQUENCER_BYTES]);

            Ok(u128::from_le_bytes(id_bytes))
        })
    }

    /// This function return decoded liq. pool id from `DepositId`
    fn get_pool_id_from_deposit_id(deposit_id: DepositId) -> Result<PoolId, Error<T>> {
        //4_294_967_296(largest invalid nft id) = encoded NftInstanceId from (1,1) - 1
        ensure!(4_294_967_296_u128.lt(&deposit_id), Error::<T>::InvalidDepositId);

        let mut pool_id_bytes = [0; POOL_ID_BYTES];

        pool_id_bytes.copy_from_slice(&deposit_id.to_le_bytes()[..POOL_ID_BYTES]);

        Ok(PoolId::from_le_bytes(pool_id_bytes))
    }

    /// This function return account from `PoolId` or error.
    ///
    /// WARN: pool_id = 0 is same as `T::PalletId::get().into_account()`. 0 is not valid value
    pub fn pool_account_id(pool_id: PoolId) -> Result<AccountIdOf<T>, Error<T>> {
        Self::validate_pool_id(pool_id)?;

        Ok(T::PalletId::get().into_sub_account(pool_id))
    }

    /// This function return now period number or error.
    fn get_now_period(blocks_per_period: BlockNumberFor<T>) -> Result<PeriodOf<T>, Error<T>> {
        Self::get_period_number(T::BlockNumberProvider::current_block_number(), blocks_per_period)
    }

    /// This function return period number from block number(`block`) and `blocks_per_period` or error.
    fn get_period_number(
        block: BlockNumberFor<T>,
        blocks_per_period: BlockNumberFor<T>,
    ) -> Result<PeriodOf<T>, Error<T>> {
        block.checked_div(&blocks_per_period).ok_or(Error::<T>::Overflow)
    }

    /// This function return loyalty multiplier or error.
    fn get_loyalty_multiplier(periods: PeriodOf<T>, curve: Option<LoyaltyCurve>) -> Result<FixedU128, Error<T>> {
        let curve = match curve {
            Some(v) => v,
            None => return Ok(FixedU128::one()), //no loyalty curve mean no loyalty multiplier
        };

        //b.is_one() is special case - this case is prevented by loyalty curve params validation
        if FixedPointNumber::is_one(&curve.initial_reward_percentage) {
            return Ok(FixedU128::one());
        }

        math::calculate_loyalty_multiplier(periods, curve.initial_reward_percentage, curve.scale_coef)
            .map_err(|_e| Error::<T>::Overflow)
    }

    /// This function calculate and update `accumulated_rpz` and all associated properties of `GlobalPool` if
    /// conditions are met.
    fn update_global_pool(
        global_pool: &mut GlobalPool<T>,
        now_period: PeriodOf<T>,
        reward_per_period: Balance,
    ) -> Result<(), Error<T>> {
        // Pool should be updated only once in the same period.
        if global_pool.updated_at == now_period {
            return Ok(());
        }

        // Nothing to update if there is no stake in the pool.
        if global_pool.total_shares_z.is_zero() {
            return Ok(());
        }

        // Number of periods since last pool update.
        let periods_since_last_update: Balance = TryInto::<u128>::try_into(
            now_period
                .checked_sub(&global_pool.updated_at)
                .ok_or(Error::<T>::Overflow)?,
        )
        .map_err(|_e| Error::<T>::Overflow)?;

        let global_pool_account = Self::pool_account_id(global_pool.id)?;
        let left_to_distribute = T::MultiCurrency::free_balance(global_pool.reward_currency, &global_pool_account);

        // Calculate reward for all periods since last update capped by balance on `GlobalPool`
        // account.
        let reward = periods_since_last_update
            .checked_mul(reward_per_period)
            .ok_or(Error::<T>::Overflow)?
            .min(left_to_distribute);

        if !reward.is_zero() {
            global_pool.accumulated_rpz =
                math::calculate_accumulated_rps(global_pool.accumulated_rpz, global_pool.total_shares_z, reward)
                    .map_err(|_e| Error::<T>::Overflow)?;
            global_pool.accumulated_rewards = global_pool
                .accumulated_rewards
                .checked_add(reward)
                .ok_or(Error::<T>::Overflow)?;
        }

        global_pool.updated_at = now_period;

        // This should emit event for FE
        T::Handler::on_accumulated_rpz_update(global_pool.id, global_pool.accumulated_rpz, global_pool.total_shares_z);

        Ok(())
    }

    /// This function calculate and return liq. pool's reward from `GlobalPool`.
    fn claim_from_global_pool(
        global_pool: &mut GlobalPool<T>,
        liq_pool: &mut LiquidityPoolYieldFarm<T>,
        stake_in_global_pool: Balance,
    ) -> Result<Balance, Error<T>> {
        let reward = math::calculate_reward(
            liq_pool.accumulated_rpz,
            global_pool.accumulated_rpz,
            stake_in_global_pool,
        )
        .map_err(|_e| Error::<T>::Overflow)?;

        liq_pool.accumulated_rpz = global_pool.accumulated_rpz;

        global_pool.paid_accumulated_rewards = global_pool
            .paid_accumulated_rewards
            .checked_add(reward)
            .ok_or(Error::<T>::Overflow)?;

        global_pool.accumulated_rewards = global_pool
            .accumulated_rewards
            .checked_sub(reward)
            .ok_or(Error::<T>::Overflow)?;

        Ok(reward)
    }

    /// This function calculate and update `accumulated_rpvz` and all associated properties of `LiquidityPoolYieldFarm` if
    /// conditions are met. Function also transfer `pool_rewareds` from `GlobalPool` account to `LiquidityPoolYieldFarm`
    /// account.
    fn update_liq_pool(
        pool: &mut LiquidityPoolYieldFarm<T>,
        pool_rewards: Balance,
        period_now: BlockNumberFor<T>,
        global_pool_id: PoolId,
        reward_currency: T::CurrencyId,
    ) -> DispatchResult {
        if pool.updated_at == period_now {
            return Ok(());
        }

        if pool.total_valued_shares.is_zero() {
            return Ok(());
        }

        pool.accumulated_rpvs =
            math::calculate_accumulated_rps(pool.accumulated_rpvs, pool.total_valued_shares, pool_rewards)
                .map_err(|_e| Error::<T>::Overflow)?;
        pool.updated_at = period_now;

        let global_pool_balance =
            T::MultiCurrency::free_balance(reward_currency, &Self::pool_account_id(global_pool_id)?);

        ensure!(
            global_pool_balance >= pool_rewards,
            Error::<T>::InsufficientBalanceInGlobalPool
        );

        let global_pool_account = Self::pool_account_id(global_pool_id)?;
        let pool_account = Self::pool_account_id(pool.id)?;

        // This should emit event for FE
        T::Handler::on_accumulated_rpvs_update(
            global_pool_id,
            pool.id,
            pool.accumulated_rpvs,
            pool.total_valued_shares,
        );

        T::MultiCurrency::transfer(reward_currency, &global_pool_account, &pool_account, pool_rewards)
    }

    /// This function return error if `pool_id` is not valid.
    fn validate_pool_id(pool_id: PoolId) -> Result<(), Error<T>> {
        if pool_id.is_zero() {
            return Err(Error::<T>::InvalidPoolId);
        }

        Ok(())
    }

    /// This function is used to validate input data before creating new farm (`GlobalPool`).
    fn validate_create_farm_data(
        total_rewards: Balance,
        planned_yielding_periods: PeriodOf<T>,
        blocks_per_period: BlockNumberFor<T>,
        yield_per_period: Permill,
    ) -> DispatchResult {
        ensure!(
            total_rewards >= T::MinTotalFarmRewards::get(),
            Error::<T>::InvalidTotalRewards
        );

        ensure!(
            planned_yielding_periods >= T::MinPlannedYieldingPeriods::get(),
            Error::<T>::InvalidPlannedYieldingPeriods
        );

        ensure!(!blocks_per_period.is_zero(), Error::<T>::InvalidBlocksPerPeriod);

        ensure!(!yield_per_period.is_zero(), Error::<T>::InvalidYieldPerPeriod);

        Ok(())
    }

    /// This function calculate account's valued shares[`Balance`] or error.
    fn get_valued_shares(
        shares: Balance,
        amm: T::AmmPoolId,
        incentivized_asset: T::CurrencyId,
    ) -> Result<Balance, Error<T>> {
        let incentivized_asset_balance = T::Handler::get_balance_in_amm(incentivized_asset, amm);

        shares
            .checked_mul(incentivized_asset_balance)
            .ok_or(Error::<T>::Overflow)
    }

    /// This function update both pools(`GlobalPool` and `LiquidityPoolYieldFarm`) if conditions are met.
    fn maybe_update_pools(
        global_pool: &mut GlobalPool<T>,
        liq_pool: &mut LiquidityPoolYieldFarm<T>,
        now_period: PeriodOf<T>,
    ) -> Result<(), DispatchError> {
        if liq_pool.canceled {
            return Ok(());
        }

        if !liq_pool.total_shares.is_zero() && liq_pool.updated_at != now_period {
            if !global_pool.total_shares_z.is_zero() && global_pool.updated_at != now_period {
                let rewards = math::calculate_global_pool_reward_per_period(
                    global_pool.yield_per_period.into(),
                    global_pool.total_shares_z,
                    global_pool.max_reward_per_period,
                )
                .map_err(|_e| Error::<T>::Overflow)?;

                Self::update_global_pool(global_pool, now_period, rewards)?;
            }

            let stake_in_global_pool =
                math::calculate_global_pool_shares(liq_pool.total_valued_shares, liq_pool.multiplier)
                    .map_err(|_e| Error::<T>::Overflow)?;
            let rewards = Self::claim_from_global_pool(global_pool, liq_pool, stake_in_global_pool)?;
            Self::update_liq_pool(
                liq_pool,
                rewards,
                now_period,
                global_pool.id,
                global_pool.reward_currency,
            )?;
        }
        Ok(())
    }

    pub fn liquidity_pool_farm_exists(deposit_id: DepositId, amm_pool_id: &T::AmmPoolId) -> Result<bool, Error<T>> {
        let liq_pool_id = Self::get_pool_id_from_deposit_id(deposit_id)?;

        if let Some((_, farm_id)) = Self::liq_pool_meta(liq_pool_id) {
            return Ok(Self::liquidity_pool(farm_id, amm_pool_id).is_some());
        }

        Ok(false)
    }
}
