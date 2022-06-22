// This file is part of galacticcouncil/warehouse.

// Copyright (C) 2020-2022  Intergalactic, Limited (GIB).
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// Abbr:
//  rpvs - reward per valued share
//  rpz - reward per share in global farm

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
//! factor) and multiple incentives scheme.
//! Users are rewarded for each period they stay in liq. mining program.
//!
//! Reward per one period is derived from the user's loyalty factor which grows with time(periods)
//! the user is in the liq. mining and amount of LP shares user locked into deposit.
//! User's loyalty factor is reset if the user exits and reenters liquidity mining.
//! User can claim rewards without resetting loyalty factor, only withdrawing shares
//! is penalized by loyalty factor reset.
//! User is rewarded from the next period after he enters.
//!
//! Multiple Incentives
//!
//! This feature allow users to redeposit already deposited LP shares to multiple yield farms and
//! receive incentives from this farms.
//! Deposit in yield farm is called "farm entry".
//! Maximal number of redepositing same LP shares is configured by variable: `MaxFarmEntriesPerDeposit`.
//! Set `MaxFarmEntriesPerDeposit` to `1` to disable multiple incentives scheme. !!!NEVER set this
//! variable to `0`.
//! LP shares can be redeposited only to different yield farms running liquidity mining for same
//! pair of assets.
//!
//! Notes:
//! * LP shares are returned ONLY if deposit is destroyed - withdrawing LP shares can
//! be used to "free slot" for re-lock LP shares to different yield farm. Withdrawing LP shares result in
//! resetting loyalty factor for yield farm user is withdrawing from(other farm entries in the
//! deposit are not affected). If deposit has no more farm entries, deposit is destroyed and LP
//! shares are returned back to user.
//! * `YiedlFarm` -  can be in the 3 states: [`Active`, `Stopped`, `Canceled`]
//!     * `Active` - liquidity mining is running, users are able to deposit, claim and withdraw LP
//!     shares. `YiedlFarm` is rewarded from `GlobalFarm` in this state.
//!     * `Stopped` - liquidity mining is stopped. Users can claim and withdraw LP shares from the
//!     farm. Users CAN'T deposit new LP shares to stopped farm. Stopped farm is not rewarded from the
//!     `GlobalFarm`.
//!     Note: stopped farm can be resumed or destroyed.
//!     * `Deleted` - liquidity mining is ended. User's CAN'T deposit or claim rewards from
//!     stopped farm. Users CAN only withdraw LP shares(without rewards).
//!     `YiedlFarm` must be stopped before it can be deleted. Deleted farm stays in the storage
//!     until last farm's entry is withdrawn. Last withdrawn from yield farm will remove deleted
//!     farm from the storage.
//!     Note: Deleted farm CAN'T be resumed.
//! * `GlobalFarm` - can be in the 2 states: [`Active`, `Deleted`]
//!     * `Active` - liquidity mining program is running, new yield farms can be added to the
//!     global farm.
//!     * `Deleted` - liquidity mining program is ended. Yield farms can't be added to the global
//!     farm. Global farm MUST be empty(all yield farms in the global farm must be destroyed)
//!     before it can be destroyed. Destroying global farm transfer undistributed rewards to fram's
//!     owner. Deleted global farm stay in the storage until all yield farms are removed from
//!     the storage. Last yield farm removal from storage triggers global farm removal from
//!     storage.
//!     Note: deleted global farm CAN'T be resumed.
//!
//!

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests;
mod types;

pub use pallet::*;

pub use crate::types::{
    Balance, DepositData, DepositId, FarmId, FarmMultiplier, FarmState, GlobalFarmData, GlobalFarmId, LoyaltyCurve,
    YieldFarmData, YieldFarmEntry, YieldFarmId,
};
use codec::{Decode, Encode, FullCodec};
use frame_support::{
    pallet_prelude::*,
    sp_runtime::{
        traits::{AccountIdConversion, BlockNumberProvider, MaybeSerializeDeserialize, One, Zero},
        RuntimeDebug,
    },
    PalletId,
};

use sp_runtime::ArithmeticError;

use hydra_dx_math::liquidity_mining as math;
use orml_traits::MultiCurrency;
use scale_info::TypeInfo;
use sp_arithmetic::{
    traits::{CheckedDiv, CheckedSub},
    FixedPointNumber, FixedU128, Permill,
};
use sp_std::convert::{From, Into, TryInto};

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
type AssetIdOf<T, I = ()> = <T as pallet::Config<I>>::CurrencyId;
type BlockNumberFor<T> = <T as frame_system::Config>::BlockNumber;
type PeriodOf<T> = <T as frame_system::Config>::BlockNumber;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T, I = ()>(PhantomData<(T, I)>);

    #[pallet::hooks]
    impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {}

    #[pallet::config]
    pub trait Config<I: 'static = ()>: frame_system::Config + TypeInfo {
        type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

        /// Asset type.
        type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord + From<u32>;

        /// Currency for transfers.
        type MultiCurrency: MultiCurrency<Self::AccountId, CurrencyId = Self::CurrencyId, Balance = Balance>;

        /// Pallet id.
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// Minimum total rewards to distribute from global farm during liquidity mining.
        #[pallet::constant]
        type MinTotalFarmRewards: Get<Balance>;

        /// Minimum number of periods to run liquidity mining program.
        #[pallet::constant]
        type MinPlannedYieldingPeriods: Get<Self::BlockNumber>;

        /// The block number provider
        type BlockNumberProvider: BlockNumberProvider<BlockNumber = Self::BlockNumber>;

        /// Id used to identify amm pool in liquidity mining pallet.
        type AmmPoolId: Parameter + Member + Clone + FullCodec;

        /// Maximum number of yield farms same LP shares can be re/deposited into. This value always
        /// MUST BE >= 1.         
        #[pallet::constant]
        type MaxFarmEntriesPerDeposit: Get<u32>;

        /// Max number of yield farms can exist in global farm. This includes all farms in the
        /// storage(active, stopped, deleted).
        #[pallet::constant]
        type MaxYieldFarmsPerGlobalFarm: Get<u32>;
    }

    #[pallet::error]
    #[cfg_attr(test, derive(PartialEq))]
    pub enum Error<T, I = ()> {
        /// Global farm does not exist.
        GlobalFarmNotFound,

        /// Yield farm does not exist.
        YieldFarmNotFound,

        /// Deposit does not exist.
        DepositNotFound,

        /// Multiple claims in the same period is not allowed.
        DoubleClaimInPeriod,

        /// Liquidity mining is canceled.
        LiquidityMiningCanceled,

        /// Liquidity mining is not canceled.
        LiquidityMiningIsActive,

        /// LP shares amount is not valid.
        InvalidDepositAmount,

        /// Account is not allowed to perform action.
        Forbidden,

        /// Yield farm multiplier can't be 0.
        InvalidMultiplier,

        /// Yield farm with given `amm_pool_id` already exists in global farm.
        YieldFarmAlreadyExists,

        /// Loyalty curve's initial reward percentage is not valid. Valid range is: [0, 1).
        InvalidInitialRewardPercentage,

        /// One or more yield farms exist in global farm.
        GlobalFarmIsNotEmpty,

        /// Farm's `incentivized_asset` is missing in provided asset pair.
        MissingIncentivizedAsset,

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

        /// Insufficient reward currency in global farm.
        InsufficientBalanceInGlobalFarm,

        /// Provided farm id is not valid. Valid range is [1, u32::MAX)
        InvalidFarmId,

        /// Maximum number of locks reached for deposit.
        MaxEntriesPerDeposit,

        /// Trying to lock LP shares into already locked yield farm.
        DoubleLock,

        /// Yield farm entry doesn't exist for given deposit.
        YieldFarmEntryNotFound,

        /// Max number of yield farms in global farm was reached. Global farm can't accept new
        /// yield farms until some yield farm is not removed from storage.
        GlobalFarmIsFull,

        /// Invalid min. deposit was set for global farm.
        InvalidMinDeposit,

        /// Price adjustment multiplier can't be 0.
        InvalidPriceAdjustment,
    }

    /// Id sequencer for `GlobalFarm` and `YieldFarm`.
    #[pallet::storage]
    #[pallet::getter(fn farm_id)]
    pub type FarmSequencer<T: Config<I>, I: 'static = ()> = StorageValue<_, FarmId, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn deposit_id)]
    pub type DepositSequencer<T: Config<I>, I: 'static = ()> = StorageValue<_, DepositId, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn global_farm)]
    pub type GlobalFarm<T: Config<I>, I: 'static = ()> =
        StorageMap<_, Blake2_128Concat, GlobalFarmId, GlobalFarmData<T, I>, OptionQuery>;

    /// Yield farm details.
    #[pallet::storage]
    #[pallet::getter(fn yield_farm)]
    pub type YieldFarm<T: Config<I>, I: 'static = ()> = StorageNMap<
        _,
        (
            NMapKey<Blake2_128Concat, T::AmmPoolId>,
            NMapKey<Blake2_128Concat, GlobalFarmId>,
            NMapKey<Blake2_128Concat, YieldFarmId>,
        ),
        YieldFarmData<T, I>,
        OptionQuery,
    >;

    /// Deposit details.
    #[pallet::storage]
    #[pallet::getter(fn deposit)]
    pub type Deposit<T: Config<I>, I: 'static = ()> =
        StorageMap<_, Blake2_128Concat, DepositId, DepositData<T, I>, OptionQuery>;

    /// Active(farms able to receive LP shares deposits) yield farms.
    #[pallet::storage]
    #[pallet::getter(fn active_yield_farm)]
    pub type ActiveYieldFarm<T: Config<I>, I: 'static = ()> =
        StorageDoubleMap<_, Blake2_128Concat, T::AmmPoolId, Blake2_128Concat, GlobalFarmId, YieldFarmId>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config<I>, I: 'static = ()> {
        /// Global farm accumulated reward per share was updated.
        GlobalFarmAccRPZUpdated {
            global_farm_id: GlobalFarmId,
            accumulated_rpz: Balance,
            total_shares_z: Balance,
        },

        /// Yield farm accumulated reward per valued share was updated.
        YieldFarmAccRPVSUpdated {
            global_farm_id: GlobalFarmId,
            yield_farm_id: YieldFarmId,
            accumulated_rpvs: Balance,
            total_valued_shares: Balance,
        },
    }

    #[pallet::call]
    impl<T: Config<I>, I: 'static> Pallet<T, I> {}
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
    /// Create new liquidity mining program with provided parameters.
    ///
    /// `owner` account have to have at least `total_rewards` balance. This funds will be
    /// transferred from `owner` to farm account.
    ///
    /// Returns: `(GlobalFarmId, max reward per period)`
    ///
    /// Parameters:
    /// - `total_rewards`: total rewards planned to distribute. This rewards will be
    /// distributed between all yield farms in the global farm.
    /// - `planned_yielding_periods`: planned number of periods to distribute `total_rewards`.
    /// WARN: THIS IS NOT HARD DEADLINE. Not all rewards have to be distributed in
    /// `planned_yielding_periods`. Rewards are distributed based on the situation in the yield
    /// farm and can be distributed in a longer never in the shorter time frame.
    /// - `blocks_per_period`:  number of blocks in a single period. Min. number of blocks per
    /// period is 1.
    /// - `incentivized_asset`: asset to be incentivized in AMM pools. All yield farms added into
    /// global farm must to have `incentivized_asset` in their pair.
    /// - `reward_currency`: payoff currency of rewards.
    /// - `owner`: liq. mining farm owner.
    /// - `yield_per_period`: percentage return on `reward_currency` of all pools
    #[allow(clippy::too_many_arguments)]
    fn create_global_farm(
        total_rewards: Balance,
        planned_yielding_periods: PeriodOf<T>,
        blocks_per_period: BlockNumberFor<T>,
        incentivized_asset: AssetIdOf<T, I>,
        reward_currency: AssetIdOf<T, I>,
        owner: AccountIdOf<T>,
        yield_per_period: Permill,
        min_deposit: Balance,
        price_adjustment: FixedU128,
    ) -> Result<(GlobalFarmId, Balance), DispatchError> {
        Self::validate_create_global_farm_data(
            total_rewards,
            planned_yielding_periods,
            blocks_per_period,
            yield_per_period,
            min_deposit,
            price_adjustment,
        )?;

        ensure!(
            T::MultiCurrency::free_balance(reward_currency, &owner) >= total_rewards,
            Error::<T, I>::InsufficientRewardCurrencyBalance
        );

        let planned_periods =
            TryInto::<u128>::try_into(planned_yielding_periods).map_err(|_| ArithmeticError::Overflow)?;
        let max_reward_per_period = total_rewards
            .checked_div(planned_periods)
            .ok_or(ArithmeticError::DivisionByZero)?;
        let current_period = Self::get_current_period(blocks_per_period)?;
        let farm_id = Self::get_next_farm_id()?;

        let global_farm = GlobalFarmData::new(
            farm_id,
            current_period,
            reward_currency,
            yield_per_period,
            planned_yielding_periods,
            blocks_per_period,
            owner,
            incentivized_asset,
            max_reward_per_period,
            min_deposit,
            price_adjustment,
        );

        <GlobalFarm<T, I>>::insert(&global_farm.id, &global_farm);

        let global_farm_account = Self::farm_account_id(global_farm.id)?;
        T::MultiCurrency::transfer(reward_currency, &global_farm.owner, &global_farm_account, total_rewards)?;

        Ok((farm_id, max_reward_per_period))
    }

    /// Destroy existing liquidity mining program. Undistributed rewards are transferred to
    /// owner(`who`).
    ///
    /// Only farm's owner can perform this action.
    ///
    /// WARN: To successfully destroy a global farm, farm have to be empty(all yield farms in the
    /// global farm must be destroyed)
    ///
    /// Returns: `(reward currency, undistributed rewards, destination account)`
    ///
    /// Parameters:
    /// - `who`: farm's owner.
    /// - `farm_id`: id of farm to be destroyed.
    fn destroy_global_farm(
        who: AccountIdOf<T>,
        farm_id: GlobalFarmId,
    ) -> Result<(T::CurrencyId, Balance, AccountIdOf<T>), DispatchError> {
        <GlobalFarm<T, I>>::try_mutate_exists(farm_id, |maybe_global_farm| {
            let global_farm = maybe_global_farm.as_mut().ok_or(Error::<T, I>::GlobalFarmNotFound)?;

            ensure!(who == global_farm.owner, Error::<T, I>::Forbidden);

            ensure!(!global_farm.has_live_farms(), Error::<T, I>::GlobalFarmIsNotEmpty);

            let global_farm_account = Self::farm_account_id(global_farm.id)?;
            let undistributed_rewards =
                T::MultiCurrency::free_balance(global_farm.reward_currency, &global_farm_account);

            T::MultiCurrency::transfer(
                global_farm.reward_currency,
                &global_farm_account,
                &who,
                undistributed_rewards,
            )?;

            //Mark for flush from storage on last `YieldFarm` in the farm flush.
            global_farm.state = FarmState::Deleted;

            let reward_currency = global_farm.reward_currency;
            if global_farm.can_be_flushed() {
                *maybe_global_farm = None;
            }

            Ok((reward_currency, undistributed_rewards, who))
        })
    }

    /// Add yield farm to global farm and start liquidity mining for given assets pair.
    ///  
    /// Only farm owner can perform this action.
    ///
    /// One of the AMM assets has to be `incentivized_token`. Same AMM can be
    /// in the same farm only once.
    ///
    /// Parameters:
    /// - `who`: farm's owner
    /// - `global_farm_id`: farm id to which a yield farm will be added.
    /// - `multiplier`: yield farm multiplier.
    /// - `loyalty_curve`: curve to calculate loyalty multiplier to distribute rewards to users
    /// with time incentive. `None` means no loyalty multiplier.
    /// - `amm_pool_id`: identifier of the AMM pool.
    /// - `asset_a`: one of the assets in the AMM.
    /// - `asset_b`: second asset in the AMM.
    fn create_yield_farm(
        who: AccountIdOf<T>,
        global_farm_id: GlobalFarmId,
        multiplier: FarmMultiplier,
        loyalty_curve: Option<LoyaltyCurve>,
        amm_pool_id: T::AmmPoolId,
        asset_a: T::CurrencyId,
        asset_b: T::CurrencyId,
    ) -> Result<YieldFarmId, DispatchError> {
        ensure!(!multiplier.is_zero(), Error::<T, I>::InvalidMultiplier);

        if let Some(ref curve) = loyalty_curve {
            ensure!(
                curve.initial_reward_percentage.lt(&FixedU128::one()),
                Error::<T, I>::InvalidInitialRewardPercentage
            );
        }

        <GlobalFarm<T, I>>::try_mutate(
            global_farm_id,
            |maybe_global_farm| -> Result<YieldFarmId, DispatchError> {
                let global_farm = maybe_global_farm.as_mut().ok_or(Error::<T, I>::GlobalFarmNotFound)?;

                //This is basically same as farm not found.
                ensure!(global_farm.is_active(), Error::<T, I>::GlobalFarmNotFound);

                ensure!(who == global_farm.owner, Error::<T, I>::Forbidden);

                ensure!(!global_farm.is_full(), Error::<T, I>::GlobalFarmIsFull);

                ensure!(
                    asset_a == global_farm.incentivized_asset || asset_b == global_farm.incentivized_asset,
                    Error::<T, I>::MissingIncentivizedAsset
                );

                <ActiveYieldFarm<T, I>>::try_mutate(amm_pool_id.clone(), &global_farm_id, |maybe_active_yield_farm| {
                    ensure!(maybe_active_yield_farm.is_none(), Error::<T, I>::YieldFarmAlreadyExists);

                    // update global farm accumulated RPZ
                    let current_period = Self::get_current_period(global_farm.blocks_per_period)?;
                    let total_shares_z_adjusted = global_farm
                        .price_adjustment
                        .checked_mul_int(global_farm.total_shares_z)
                        .ok_or(ArithmeticError::Overflow)?;

                    if !global_farm.total_shares_z.is_zero() && global_farm.updated_at != current_period {
                        let reward_per_period = math::calculate_global_pool_reward_per_period(
                            global_farm.yield_per_period.into(),
                            total_shares_z_adjusted,
                            global_farm.max_reward_per_period,
                        )
                        .map_err(|_| ArithmeticError::Overflow)?;
                        Self::update_global_farm(global_farm, current_period, reward_per_period)?;
                    }

                    let yield_farm_id = Self::get_next_farm_id()?;

                    let yield_farm =
                        YieldFarmData::new(yield_farm_id, current_period, loyalty_curve.clone(), multiplier);

                    <YieldFarm<T, I>>::insert((amm_pool_id, global_farm_id, yield_farm_id), yield_farm);
                    global_farm.increase_yield_farm_counts()?;

                    *maybe_active_yield_farm = Some(yield_farm_id);

                    Ok(yield_farm_id)
                })
            },
        )
    }

    /// Update yield farm's multiplier.
    ///  
    /// Only farm's owner can perform this action.
    ///
    /// Parameters:
    /// - `who`: farm's owner
    /// - `global_farm_id`: global farm id in which yield farm will be updated.
    /// - `multiplier`: new yield farm multiplier.
    /// - `amm_pool_id`: identifier of the AMM pool.
    fn update_yield_farm_multiplier(
        who: AccountIdOf<T>,
        global_farm_id: GlobalFarmId,
        amm_pool_id: T::AmmPoolId,
        multiplier: FarmMultiplier,
    ) -> Result<YieldFarmId, DispatchError> {
        ensure!(!multiplier.is_zero(), Error::<T, I>::InvalidMultiplier);

        let yield_farm_id =
            Self::active_yield_farm(amm_pool_id.clone(), global_farm_id).ok_or(Error::<T, I>::YieldFarmNotFound)?;

        <YieldFarm<T, I>>::try_mutate((amm_pool_id, global_farm_id, yield_farm_id), |maybe_yield_farm| {
            let yield_farm = maybe_yield_farm.as_mut().ok_or(Error::<T, I>::YieldFarmNotFound)?;

            ensure!(yield_farm.is_active(), Error::<T, I>::LiquidityMiningCanceled);

            <GlobalFarm<T, I>>::try_mutate(global_farm_id, |maybe_global_farm| {
                let global_farm = maybe_global_farm.as_mut().ok_or(Error::<T, I>::GlobalFarmNotFound)?;

                ensure!(who == global_farm.owner, Error::<T, I>::Forbidden);

                let old_stake_in_global_farm =
                    math::calculate_global_pool_shares(yield_farm.total_valued_shares, yield_farm.multiplier)
                        .map_err(|_| ArithmeticError::Overflow)?;

                let current_period = Self::get_current_period(global_farm.blocks_per_period)?;
                Self::maybe_update_farms(global_farm, yield_farm, current_period)?;

                let new_stake_in_global_farm =
                    math::calculate_global_pool_shares(yield_farm.total_valued_shares, multiplier)
                        .map_err(|_| ArithmeticError::Overflow)?;

                global_farm.total_shares_z = global_farm
                    .total_shares_z
                    .checked_sub(old_stake_in_global_farm)
                    .ok_or(ArithmeticError::Overflow)?
                    .checked_add(new_stake_in_global_farm)
                    .ok_or(ArithmeticError::Overflow)?;

                yield_farm.multiplier = multiplier;

                Ok(yield_farm.id)
            })
        })
    }

    /// Stop liquidity mining for specific yield farm.
    ///
    /// This function claims rewards from `GlobalFarm` for the last time and stops yield farm
    /// incentivization from a `GlobalFarm`. Users will be able to only claim and withdraw LP
    /// shares after calling this function.
    /// `deposit_lp_shares()` is not allowed on stopped yield farm.
    ///  
    /// Only farm owner can perform this action.
    ///
    /// Parameters:
    /// - `who`: farm's owner.
    /// - `global_farm_id`: farm id in which yield farm will be stopped.
    /// - `amm_pool_id`: identifier of the AMM pool.
    fn stop_yield_farm(
        who: AccountIdOf<T>,
        global_farm_id: GlobalFarmId,
        amm_pool_id: T::AmmPoolId,
    ) -> Result<YieldFarmId, DispatchError> {
        <ActiveYieldFarm<T, I>>::try_mutate_exists(
            amm_pool_id.clone(),
            global_farm_id,
            |maybe_active_yield_farm_id| -> Result<YieldFarmId, DispatchError> {
                let yield_farm_id = maybe_active_yield_farm_id
                    .as_ref()
                    .ok_or(Error::<T, I>::YieldFarmNotFound)?;

                <YieldFarm<T, I>>::try_mutate(
                    (amm_pool_id, global_farm_id, yield_farm_id),
                    |maybe_yield_farm| -> Result<(), DispatchError> {
                        let yield_farm = maybe_yield_farm.as_mut().ok_or(Error::<T, I>::YieldFarmNotFound)?;

                        ensure!(yield_farm.is_active(), Error::<T, I>::LiquidityMiningCanceled);

                        <GlobalFarm<T, I>>::try_mutate(global_farm_id, |maybe_global_farm| {
                            let global_farm = maybe_global_farm.as_mut().ok_or(Error::<T, I>::GlobalFarmNotFound)?;

                            ensure!(global_farm.owner == who, Error::<T, I>::Forbidden);

                            let current_period = Self::get_current_period(global_farm.blocks_per_period)?;
                            Self::maybe_update_farms(global_farm, yield_farm, current_period)?;

                            let old_stake_in_global_pool = math::calculate_global_pool_shares(
                                yield_farm.total_valued_shares,
                                yield_farm.multiplier,
                            )
                            .map_err(|_| ArithmeticError::Overflow)?;

                            global_farm.total_shares_z = global_farm
                                .total_shares_z
                                .checked_sub(old_stake_in_global_pool)
                                .ok_or(ArithmeticError::Overflow)?;

                            yield_farm.state = FarmState::Stopped;
                            yield_farm.multiplier = FarmMultiplier::default();

                            Ok(())
                        })
                    },
                )?;

                let yield_farm_id = *yield_farm_id;
                //Remove yield farm from active farms storage.
                *maybe_active_yield_farm_id = None;

                Ok(yield_farm_id)
            },
        )
    }

    /// Resume liquidity mining for stopped yield farm.
    ///
    /// This function resume incentivization from `GlobalPool` and restore full functionality
    /// for yield farm. Users will be able to deposit, claim and withdraw again.
    ///
    /// Yield farm is not rewarded for the time it was stopped.
    ///
    /// Only farm's owner can perform this action.
    ///
    /// Parameters:
    /// - `who`: farm's owner
    /// - `global_farm_id`: farm id in which yield farm will be resumed.
    /// - `yield_farm_id`: id of yield farm to resume.
    /// - `amm_pool_id`: identifier of the AMM pool.
    /// - `multiplier`: yield farm's multiplier.
    fn resume_yield_farm(
        who: AccountIdOf<T>,
        global_farm_id: GlobalFarmId,
        yield_farm_id: YieldFarmId,
        amm_pool_id: T::AmmPoolId,
        multiplier: FarmMultiplier,
    ) -> Result<(), DispatchError> {
        ensure!(!multiplier.is_zero(), Error::<T, I>::InvalidMultiplier);

        <ActiveYieldFarm<T, I>>::try_mutate(amm_pool_id.clone(), global_farm_id, |maybe_active_yield_farm_id| {
            ensure!(
                maybe_active_yield_farm_id.is_none(),
                Error::<T, I>::YieldFarmAlreadyExists
            );

            <YieldFarm<T, I>>::try_mutate((amm_pool_id, global_farm_id, yield_farm_id), |maybe_yield_farm| {
                let yield_farm = maybe_yield_farm.as_mut().ok_or(Error::<T, I>::YieldFarmNotFound)?;

                //Active or deleted yield farms can't be resumed.
                ensure!(yield_farm.is_stopped(), Error::<T, I>::LiquidityMiningIsActive);

                <GlobalFarm<T, I>>::try_mutate(global_farm_id, |maybe_global_farm| {
                    let global_farm = maybe_global_farm.as_mut().ok_or(Error::<T, I>::GlobalFarmNotFound)?;

                    ensure!(global_farm.owner == who, Error::<T, I>::Forbidden);

                    //update `GlobalFarm` accumulated_rpz
                    let current_period = Self::get_current_period(global_farm.blocks_per_period)?;
                    if !global_farm.total_shares_z.is_zero() && global_farm.updated_at != current_period {
                        let total_shares_z_adjusted = global_farm
                            .price_adjustment
                            .checked_mul_int(global_farm.total_shares_z)
                            .ok_or(ArithmeticError::Overflow)?;
                        let reward_per_period = math::calculate_global_pool_reward_per_period(
                            global_farm.yield_per_period.into(),
                            total_shares_z_adjusted,
                            global_farm.max_reward_per_period,
                        )
                        .map_err(|_| ArithmeticError::Overflow)?;
                        Self::update_global_farm(global_farm, current_period, reward_per_period)?;
                    }

                    let new_stake_in_global_farm =
                        math::calculate_global_pool_shares(yield_farm.total_valued_shares, multiplier)
                            .map_err(|_| ArithmeticError::Overflow)?;

                    global_farm.total_shares_z = global_farm
                        .total_shares_z
                        .checked_add(new_stake_in_global_farm)
                        .ok_or(ArithmeticError::Overflow)?;

                    yield_farm.accumulated_rpz = global_farm.accumulated_rpz;
                    yield_farm.updated_at = current_period;
                    yield_farm.state = FarmState::Active;
                    yield_farm.multiplier = multiplier;

                    //add yield farm to active farms.
                    *maybe_active_yield_farm_id = Some(yield_farm.id);

                    Ok(())
                })
            })
        })
    }

    /// This function marks an yield farm ready for removal from storage when it's empty. Users will
    /// be able to only withdraw shares(without claiming rewards from yield farm). Unpaid rewards
    /// will be transferred back to global farm and will be used to distribute to other yield farms.
    ///
    /// Yield farm must be stopped before calling this function.
    ///
    /// Only farm's owner can perform this action. Yield farm stays in the storage until it's
    /// empty(all farm entries are withdrawn). Last withdrawn from yield farm trigger removing from
    /// the storage.
    ///
    /// Parameters:
    /// - `who`: farm's owner.
    /// - `global_farm_id`: farm id from which yield farm will be removed.
    /// - `yield_farm_id`: yield farm id of farm to destroy.
    /// - `amm_pool_id`: identifier of the AMM pool.
    fn destroy_yield_farm(
        who: AccountIdOf<T>,
        global_farm_id: GlobalFarmId,
        yield_farm_id: YieldFarmId,
        amm_pool_id: T::AmmPoolId,
    ) -> Result<(), DispatchError> {
        ensure!(
            !<ActiveYieldFarm<T, I>>::contains_key(amm_pool_id.clone(), global_farm_id),
            Error::<T, I>::LiquidityMiningIsActive
        );

        <GlobalFarm<T, I>>::try_mutate_exists(global_farm_id, |maybe_global_farm| {
            let global_farm = maybe_global_farm.as_mut().ok_or(Error::<T, I>::GlobalFarmNotFound)?;

            ensure!(global_farm.owner == who, Error::<T, I>::Forbidden);

            <YieldFarm<T, I>>::try_mutate_exists(
                (amm_pool_id, global_farm_id, yield_farm_id),
                |maybe_yield_farm| -> Result<(), DispatchError> {
                    let yield_farm = maybe_yield_farm.as_mut().ok_or(Error::<T, I>::YieldFarmNotFound)?;

                    ensure!(yield_farm.is_stopped(), Error::<T, I>::LiquidityMiningIsActive);

                    //Transfer unpaid rewards back to global_pool.
                    let global_farm_account = Self::farm_account_id(global_farm.id)?;
                    let yield_farm_account = Self::farm_account_id(yield_farm.id)?;

                    let unpaid_reward =
                        T::MultiCurrency::free_balance(global_farm.reward_currency, &yield_farm_account);
                    T::MultiCurrency::transfer(
                        global_farm.reward_currency,
                        &yield_farm_account,
                        &global_farm_account,
                        unpaid_reward,
                    )?;

                    //Delete yield farm.
                    yield_farm.state = FarmState::Deleted;
                    global_farm.decrease_live_yield_farm_count()?;

                    //Cleanup if it's possible
                    if yield_farm.can_be_flushed() {
                        global_farm.decrease_total_yield_farm_count()?;

                        *maybe_yield_farm = None;
                    }

                    Ok(())
                },
            )?;

            Ok(())
        })
    }

    /// Deposit LP shares to a yield farm.
    ///
    /// This function creates new deposit farm entry in the yield farm.
    ///
    /// Parameters:
    /// - `global_farm_id`: global farm identifier.
    /// - `yield_farm_id`: yield farm identifier depositing to.
    /// - `amm_pool_id`: identifier of the AMM pool.
    /// - `shares_amount`: amount of LP shares user want to deposit.
    /// - `amm_pool_id`: identifier of the AMM pool.
    /// - `get_balance_in_amm`: callback function returning balance of incentivized asset in amm
    /// pool
    fn deposit_lp_shares(
        global_farm_id: GlobalFarmId,
        yield_farm_id: YieldFarmId,
        amm_pool_id: T::AmmPoolId,
        shares_amount: Balance,
        get_balance_in_amm: fn(T::CurrencyId, T::AmmPoolId) -> Result<Balance, DispatchError>,
    ) -> Result<DepositId, DispatchError> {
        let mut deposit = DepositData::new(shares_amount, amm_pool_id);

        Self::do_deposit_lp_shares(&mut deposit, global_farm_id, yield_farm_id, get_balance_in_amm)?;

        //Save deposit to storage.
        let deposit_id = Self::get_next_deposit_id()?;
        <Deposit<T, I>>::insert(deposit_id, deposit);

        Ok(deposit_id)
    }

    /// This function create yield farm entry for existing deposit. LP shares are not transferred
    /// and amount of LP shares is based on existing deposit.
    ///
    /// This function DOESN'T create new deposit.
    ///
    /// Returns: `(redeposited shares amount)`
    ///
    /// Parameters:
    /// - `global_farm_id`: global farm identifier.
    /// - `yield_farm_id`: yield farm identifier redepositing to.
    /// - `deposit_id`: identifier of the AMM pool.
    /// - `get_balance_in_amm`: callback function returning balance of incentivized asset in amm
    /// pool
    fn redeposit_lp_shares(
        global_farm_id: GlobalFarmId,
        yield_farm_id: YieldFarmId,
        deposit_id: DepositId,
        get_balance_in_amm: fn(T::CurrencyId, T::AmmPoolId) -> Result<Balance, DispatchError>,
    ) -> Result<Balance, DispatchError> {
        <Deposit<T, I>>::try_mutate(deposit_id, |maybe_deposit| {
            let deposit = maybe_deposit.as_mut().ok_or(Error::<T, I>::DepositNotFound)?;

            Self::do_deposit_lp_shares(deposit, global_farm_id, yield_farm_id, get_balance_in_amm)?;

            Ok(deposit.shares)
        })
    }

    /// Claim rewards from yield farm for given deposit.
    ///
    /// This function calculate user rewards from yield farm and transfer rewards to `who`
    /// account. Claiming in the same period is configured by `check_double_claim` parameter.
    /// Second claim in the same period result in `0` claims. This is desirable for in case we need
    /// `unclaimable_rewards` e.g. for `withdraw_lp_shares()`
    ///
    /// WARN: User have to use `withdraw_shares()` if yield farm is destroyed.
    ///
    /// Returns: `(GlobalFarmId, reward currency, claimed amount, unclaimable amount)`
    ///
    /// Parameters:
    /// - `who`: destination account to receive rewards.
    /// - `deposit_id`: id representing deposit in the yield farm.
    /// - `yield_farm_id`: identifier of yield farm to withdrawn from.
    /// - `check_double_claim`: fn failed on second claim in the same period if set to `true`.
    fn claim_rewards(
        who: AccountIdOf<T>,
        deposit_id: DepositId,
        yield_farm_id: YieldFarmId,
        fail_on_doubleclaim: bool,
    ) -> Result<(GlobalFarmId, T::CurrencyId, Balance, Balance), DispatchError> {
        <Deposit<T, I>>::try_mutate(deposit_id, |maybe_deposit| {
            let deposit = maybe_deposit.as_mut().ok_or(Error::<T, I>::DepositNotFound)?;

            let amm_pool_id = deposit.amm_pool_id.clone();
            let farm_entry = deposit
                .get_yield_farm_entry(yield_farm_id)
                .ok_or(Error::<T, I>::YieldFarmEntryNotFound)?;

            <YieldFarm<T, I>>::try_mutate(
                (amm_pool_id, farm_entry.global_farm_id, yield_farm_id),
                |maybe_yield_farm| {
                    let yield_farm = maybe_yield_farm.as_mut().ok_or(Error::<T, I>::YieldFarmNotFound)?;

                    //NOTE: claiming from removed yield farm should NOT work. This is same as yield
                    //farm doesn't exist.
                    ensure!(!yield_farm.is_deleted(), Error::<T, I>::YieldFarmNotFound);

                    <GlobalFarm<T, I>>::try_mutate(farm_entry.global_farm_id, |maybe_global_farm| {
                        let global_farm = maybe_global_farm.as_mut().ok_or(Error::<T, I>::GlobalFarmNotFound)?;

                        let current_period = Self::get_current_period(global_farm.blocks_per_period)?;
                        //Double claim should be allowed in some case e.g withdraw_lp_shares need
                        //`unclaimable_rewards` returned by this function.
                        if fail_on_doubleclaim {
                            ensure!(
                                farm_entry.updated_at != current_period,
                                Error::<T, I>::DoubleClaimInPeriod
                            );
                        }

                        Self::maybe_update_farms(global_farm, yield_farm, current_period)?;

                        let periods = current_period
                            .checked_sub(&farm_entry.entered_at)
                            .ok_or(ArithmeticError::Overflow)?;

                        let loyalty_multiplier =
                            Self::get_loyalty_multiplier(periods, yield_farm.loyalty_curve.clone())?;

                        let (rewards, unclaimable_rewards) = math::calculate_user_reward(
                            farm_entry.accumulated_rpvs,
                            farm_entry.valued_shares,
                            farm_entry.accumulated_claimed_rewards,
                            yield_farm.accumulated_rpvs,
                            loyalty_multiplier,
                        )
                        .map_err(|_| ArithmeticError::Overflow)?;

                        if !rewards.is_zero() {
                            farm_entry.accumulated_claimed_rewards = farm_entry
                                .accumulated_claimed_rewards
                                .checked_add(rewards)
                                .ok_or(ArithmeticError::Overflow)?;

                            farm_entry.updated_at = current_period;

                            let yield_farm_account = Self::farm_account_id(yield_farm.id)?;
                            T::MultiCurrency::transfer(
                                global_farm.reward_currency,
                                &yield_farm_account,
                                &who,
                                rewards,
                            )?;
                        }

                        Ok((
                            global_farm.id,
                            global_farm.reward_currency,
                            rewards,
                            unclaimable_rewards,
                        ))
                    })
                },
            )
        })
    }

    /// Withdraw LP shares from yield farm. This function can be used to free slot for yield
    /// farm entry in the deposit or to destroy deposit and return LP shares if deposit has no more
    /// farm entries.
    ///
    /// !!!LP shares are transferred back to user only when deposit is destroyed.
    ///
    /// This function transfer user's unclaimable rewards back to global farm.
    ///
    /// Returns: `(GlobalFarmId, withdrawn amount, true if deposit was destroyed)`
    ///
    /// Parameters:
    /// - `deposit_id`: id representing deposit in the yield farm.
    /// - `yield_farm_id`: identifier yield farm to withdrawn from.
    /// - `unclaimable_rewards`: amount of rewards user will not be able to claim because of early
    /// exit from liquidity mining program.
    fn withdraw_lp_shares(
        deposit_id: DepositId,
        yield_farm_id: YieldFarmId,
        unclaimable_rewards: Balance,
    ) -> Result<(GlobalFarmId, Balance, bool), DispatchError> {
        <Deposit<T, I>>::try_mutate_exists(deposit_id, |maybe_deposit| {
            let deposit = maybe_deposit.as_mut().ok_or(Error::<T, I>::DepositNotFound)?;

            let farm_entry = deposit.remove_yield_farm_entry(yield_farm_id)?;
            let amm_pool_id = deposit.amm_pool_id.clone();

            <GlobalFarm<T, I>>::try_mutate_exists(
                farm_entry.global_farm_id,
                |maybe_global_farm| -> Result<(), DispatchError> {
                    let global_farm = maybe_global_farm.as_mut().ok_or(Error::<T, I>::GlobalFarmNotFound)?;
                    <YieldFarm<T, I>>::try_mutate_exists(
                        (&amm_pool_id, farm_entry.global_farm_id, yield_farm_id),
                        |maybe_yield_farm| -> Result<(), DispatchError> {
                            let yield_farm = maybe_yield_farm.as_mut().ok_or(Error::<T, I>::YieldFarmNotFound)?;

                            yield_farm.total_shares = yield_farm
                                .total_shares
                                .checked_sub(deposit.shares)
                                .ok_or(ArithmeticError::Underflow)?;

                            yield_farm.total_valued_shares = yield_farm
                                .total_valued_shares
                                .checked_sub(farm_entry.valued_shares)
                                .ok_or(ArithmeticError::Underflow)?;

                            // yield farm's stake in global pool is set to `0` when farm is
                            // stopped and yield farm have to be stopped before it's deleted so
                            // this update is only required for active farms.
                            if yield_farm.is_active() {
                                let shares_in_global_farm_for_deposit =
                                    math::calculate_global_pool_shares(farm_entry.valued_shares, yield_farm.multiplier)
                                        .map_err(|_| ArithmeticError::Overflow)?;

                                global_farm.total_shares_z = global_farm
                                    .total_shares_z
                                    .checked_sub(shares_in_global_farm_for_deposit)
                                    .ok_or(ArithmeticError::Underflow)?;
                            }

                            if !unclaimable_rewards.is_zero() {
                                let global_farm_account = Self::farm_account_id(global_farm.id)?;
                                let yield_farm_account = Self::farm_account_id(yield_farm.id)?;

                                T::MultiCurrency::transfer(
                                    global_farm.reward_currency,
                                    &yield_farm_account,
                                    &global_farm_account,
                                    unclaimable_rewards,
                                )?;
                            }

                            yield_farm.decrease_entries_count()?;
                            if yield_farm.can_be_flushed() {
                                global_farm.decrease_total_yield_farm_count()?;

                                *maybe_yield_farm = None;
                            }

                            Ok(())
                        },
                    )?;

                    if global_farm.can_be_flushed() {
                        *maybe_global_farm = None;
                    }

                    Ok(())
                },
            )?;

            let withdrawn_amount = deposit.shares;
            let mut deposit_destroyed = false;
            if deposit.can_be_flushed() {
                *maybe_deposit = None;

                deposit_destroyed = true;
            }

            Ok((farm_entry.global_farm_id, withdrawn_amount, deposit_destroyed))
        })
    }

    /// Helper function to create yield farm entry.
    fn do_deposit_lp_shares(
        deposit: &mut DepositData<T, I>,
        global_farm_id: GlobalFarmId,
        yield_farm_id: YieldFarmId,
        get_balance_in_amm: fn(T::CurrencyId, T::AmmPoolId) -> Result<Balance, DispatchError>,
    ) -> Result<(), DispatchError> {
        //LP shares can be locked only once in the same yield farm.
        ensure!(
            deposit.search_yield_farm_entry(yield_farm_id).is_none(),
            Error::<T, I>::DoubleLock
        );

        <YieldFarm<T, I>>::try_mutate(
            (deposit.amm_pool_id.clone(), global_farm_id, yield_farm_id),
            |maybe_yield_farm| {
                let yield_farm = maybe_yield_farm.as_mut().ok_or(Error::<T, I>::YieldFarmNotFound)?;

                ensure!(yield_farm.is_active(), Error::<T, I>::LiquidityMiningCanceled);

                <GlobalFarm<T, I>>::try_mutate(global_farm_id, |maybe_global_farm| {
                    let global_farm = maybe_global_farm.as_mut().ok_or(Error::<T, I>::GlobalFarmNotFound)?;

                    ensure!(
                        deposit.shares.ge(&global_farm.min_deposit),
                        Error::<T, I>::InvalidDepositAmount,
                    );

                    //This should never fail. If yield farm is active also global farm MUST be
                    //active.
                    ensure!(global_farm.is_active(), Error::<T, I>::GlobalFarmNotFound);

                    let current_period = Self::get_current_period(global_farm.blocks_per_period)?;

                    Self::maybe_update_farms(global_farm, yield_farm, current_period)?;

                    let valued_shares = Self::get_valued_shares(
                        deposit.shares,
                        deposit.amm_pool_id.clone(),
                        global_farm.incentivized_asset,
                        get_balance_in_amm,
                    )?;

                    let deposit_stake_in_global_farm =
                        math::calculate_global_pool_shares(valued_shares, yield_farm.multiplier)
                            .map_err(|_| ArithmeticError::Overflow)?;

                    yield_farm.total_shares = yield_farm
                        .total_shares
                        .checked_add(deposit.shares)
                        .ok_or(ArithmeticError::Overflow)?;

                    yield_farm.total_valued_shares = yield_farm
                        .total_valued_shares
                        .checked_add(valued_shares)
                        .ok_or(ArithmeticError::Overflow)?;

                    global_farm.total_shares_z = global_farm
                        .total_shares_z
                        .checked_add(deposit_stake_in_global_farm)
                        .ok_or(ArithmeticError::Overflow)?;

                    let farm_entry = YieldFarmEntry::new(
                        global_farm.id,
                        yield_farm.id,
                        valued_shares,
                        yield_farm.accumulated_rpvs,
                        current_period,
                    );

                    deposit.add_yield_farm_entry(farm_entry)?;

                    //Increment farm's entries count
                    yield_farm.increase_entries_count()?;

                    Ok(())
                })
            },
        )
    }

    /// This function returns new unused `FarmId` usable for yield global farm or error.
    fn get_next_farm_id() -> Result<FarmId, ArithmeticError> {
        FarmSequencer::<T, I>::try_mutate(|current_id| {
            *current_id = current_id.checked_add(1).ok_or(ArithmeticError::Overflow)?;

            Ok(*current_id)
        })
    }

    /// This function returns new unused `DepositId`or error.
    fn get_next_deposit_id() -> Result<DepositId, ArithmeticError> {
        DepositSequencer::<T, I>::try_mutate(|current_id| {
            *current_id = current_id.checked_add(1).ok_or(ArithmeticError::Overflow)?;

            Ok(*current_id)
        })
    }

    /// This function returns account from `FarmId` or error.
    ///
    /// WARN: farm_id = 0 is same as `T::PalletId::get().into_account()`. 0 is not valid value.
    pub fn farm_account_id(farm_id: FarmId) -> Result<AccountIdOf<T>, Error<T, I>> {
        Self::validate_farm_id(farm_id)?;

        Ok(T::PalletId::get().into_sub_account(farm_id))
    }

    /// This function returns current period number or error.
    fn get_current_period(blocks_per_period: BlockNumberFor<T>) -> Result<PeriodOf<T>, ArithmeticError> {
        Self::get_period_number(T::BlockNumberProvider::current_block_number(), blocks_per_period)
    }

    /// This function returns period number from block number(`block`) and `blocks_per_period` or error.
    fn get_period_number(
        block: BlockNumberFor<T>,
        blocks_per_period: BlockNumberFor<T>,
    ) -> Result<PeriodOf<T>, ArithmeticError> {
        block
            .checked_div(&blocks_per_period)
            .ok_or(ArithmeticError::DivisionByZero)
    }

    /// This function returns loyalty multiplier or error.
    fn get_loyalty_multiplier(periods: PeriodOf<T>, curve: Option<LoyaltyCurve>) -> Result<FixedU128, ArithmeticError> {
        let curve = match curve {
            Some(v) => v,
            None => return Ok(FixedU128::one()), //no loyalty curve mean no loyalty multiplier
        };

        //b.is_one() is special case - this case is prevented by loyalty curve parameters validation
        if FixedPointNumber::is_one(&curve.initial_reward_percentage) {
            return Ok(FixedU128::one());
        }

        math::calculate_loyalty_multiplier(periods, curve.initial_reward_percentage, curve.scale_coef)
            .map_err(|_| ArithmeticError::Overflow)
    }

    /// This function calculate and update `accumulated_rpz` and all associated properties of `GlobalFar` if
    /// conditions are met.
    fn update_global_farm(
        global_farm: &mut GlobalFarmData<T, I>,
        now_period: PeriodOf<T>,
        reward_per_period: Balance,
    ) -> Result<(), DispatchError> {
        // Farm should be updated only once in the same period.
        if global_farm.updated_at == now_period {
            return Ok(());
        }

        // Nothing to update if there is no stake in the farm.
        if global_farm.total_shares_z.is_zero() {
            return Ok(());
        }

        // Number of periods since last farm update.
        let periods_since_last_update: Balance = TryInto::<u128>::try_into(
            now_period
                .checked_sub(&global_farm.updated_at)
                .ok_or(ArithmeticError::Underflow)?,
        )
        .map_err(|_| ArithmeticError::Overflow)?;

        let global_pool_account = Self::farm_account_id(global_farm.id)?;
        let left_to_distribute = T::MultiCurrency::free_balance(global_farm.reward_currency, &global_pool_account);

        // Calculate reward for all periods since last update capped by balance of `GlobalFarm`
        // account.
        let reward = periods_since_last_update
            .checked_mul(reward_per_period)
            .ok_or(ArithmeticError::Overflow)?
            .min(left_to_distribute);

        if !reward.is_zero() {
            global_farm.accumulated_rpz =
                math::calculate_accumulated_rps(global_farm.accumulated_rpz, global_farm.total_shares_z, reward)
                    .map_err(|_| ArithmeticError::Overflow)?;
            global_farm.accumulated_rewards = global_farm
                .accumulated_rewards
                .checked_add(reward)
                .ok_or(ArithmeticError::Overflow)?;
        }

        global_farm.updated_at = now_period;

        Pallet::<T, I>::deposit_event(Event::GlobalFarmAccRPZUpdated {
            global_farm_id: global_farm.id,
            accumulated_rpz: global_farm.accumulated_rpz,
            total_shares_z: global_farm.total_shares_z,
        });

        Ok(())
    }

    /// This function calculate and returns yield farm's reward from `GlobalFarm`.
    fn claim_from_global_farm(
        global_farm: &mut GlobalFarmData<T, I>,
        yield_farm: &mut YieldFarmData<T, I>,
        stake_in_global_pool: Balance,
    ) -> Result<Balance, ArithmeticError> {
        let reward = math::calculate_reward(
            yield_farm.accumulated_rpz,
            global_farm.accumulated_rpz,
            stake_in_global_pool,
        )
        .map_err(|_| ArithmeticError::Overflow)?;

        yield_farm.accumulated_rpz = global_farm.accumulated_rpz;

        global_farm.paid_accumulated_rewards = global_farm
            .paid_accumulated_rewards
            .checked_add(reward)
            .ok_or(ArithmeticError::Overflow)?;

        global_farm.accumulated_rewards = global_farm
            .accumulated_rewards
            .checked_sub(reward)
            .ok_or(ArithmeticError::Overflow)?;

        Ok(reward)
    }

    /// This function calculate and update `accumulated_rpvz` and all associated properties of `YieldFarm` if
    /// conditions are met. Function also transfer `yield_farm_rewards` from `GlobalFarm` account to
    /// `YieldFarm`
    /// account.
    fn update_yield_farm(
        yield_farm: &mut YieldFarmData<T, I>,
        yield_farm_rewards: Balance,
        current_period: BlockNumberFor<T>,
        global_farm_id: FarmId,
        reward_currency: T::CurrencyId,
    ) -> DispatchResult {
        if yield_farm.updated_at == current_period {
            return Ok(());
        }

        if yield_farm.total_valued_shares.is_zero() {
            return Ok(());
        }

        yield_farm.accumulated_rpvs = math::calculate_accumulated_rps(
            yield_farm.accumulated_rpvs,
            yield_farm.total_valued_shares,
            yield_farm_rewards,
        )
        .map_err(|_| ArithmeticError::Overflow)?;
        yield_farm.updated_at = current_period;

        let global_farm_balance =
            T::MultiCurrency::free_balance(reward_currency, &Self::farm_account_id(global_farm_id)?);

        ensure!(
            global_farm_balance >= yield_farm_rewards,
            Error::<T, I>::InsufficientBalanceInGlobalFarm
        );

        let global_farm_account = Self::farm_account_id(global_farm_id)?;
        let yield_farm_account = Self::farm_account_id(yield_farm.id)?;

        Pallet::<T, I>::deposit_event(Event::YieldFarmAccRPVSUpdated {
            global_farm_id,
            yield_farm_id: yield_farm.id,
            accumulated_rpvs: yield_farm.accumulated_rpvs,
            total_valued_shares: yield_farm.total_valued_shares,
        });

        T::MultiCurrency::transfer(
            reward_currency,
            &global_farm_account,
            &yield_farm_account,
            yield_farm_rewards,
        )
    }

    /// This function return error if `farm_id` is not valid.
    fn validate_farm_id(farm_id: FarmId) -> Result<(), Error<T, I>> {
        if farm_id.is_zero() {
            return Err(Error::<T, I>::InvalidFarmId);
        }

        Ok(())
    }

    /// This function is used to validate input data before creating new global farm.
    fn validate_create_global_farm_data(
        total_rewards: Balance,
        planned_yielding_periods: PeriodOf<T>,
        blocks_per_period: BlockNumberFor<T>,
        yield_per_period: Permill,
        min_deposit: Balance,
        price_adjustment: FixedU128,
    ) -> DispatchResult {
        ensure!(min_deposit.ge(&1), Error::<T, I>::InvalidMinDeposit);

        ensure!(!price_adjustment.is_zero(), Error::<T, I>::InvalidPriceAdjustment);

        ensure!(
            total_rewards >= T::MinTotalFarmRewards::get(),
            Error::<T, I>::InvalidTotalRewards
        );

        ensure!(
            planned_yielding_periods >= T::MinPlannedYieldingPeriods::get(),
            Error::<T, I>::InvalidPlannedYieldingPeriods
        );

        ensure!(!blocks_per_period.is_zero(), Error::<T, I>::InvalidBlocksPerPeriod);

        ensure!(!yield_per_period.is_zero(), Error::<T, I>::InvalidYieldPerPeriod);

        Ok(())
    }

    /// This function calculate account's valued shares[`Balance`] or error.
    fn get_valued_shares(
        shares: Balance,
        amm: T::AmmPoolId,
        incentivized_asset: T::CurrencyId,
        get_balance_in_amm: fn(T::CurrencyId, T::AmmPoolId) -> Result<Balance, DispatchError>,
    ) -> Result<Balance, DispatchError> {
        let incentivized_asset_balance = get_balance_in_amm(incentivized_asset, amm)?;

        shares
            .checked_mul(incentivized_asset_balance)
            .ok_or_else(|| ArithmeticError::Overflow.into())
    }

    /// This function update both (global and yield) farms if conditions are met.
    fn maybe_update_farms(
        global_farm: &mut GlobalFarmData<T, I>,
        yield_farm: &mut YieldFarmData<T, I>,
        current_period: PeriodOf<T>,
    ) -> Result<(), DispatchError> {
        if !yield_farm.is_active() {
            return Ok(());
        }

        if !yield_farm.total_shares.is_zero() && yield_farm.updated_at != current_period {
            if !global_farm.total_shares_z.is_zero() && global_farm.updated_at != current_period {
                let total_shares_z_adjusted = global_farm
                    .price_adjustment
                    .checked_mul_int(global_farm.total_shares_z)
                    .ok_or(ArithmeticError::Overflow)?;
                let rewards = math::calculate_global_pool_reward_per_period(
                    global_farm.yield_per_period.into(),
                    total_shares_z_adjusted,
                    global_farm.max_reward_per_period,
                )
                .map_err(|_| ArithmeticError::Overflow)?;

                Self::update_global_farm(global_farm, current_period, rewards)?;
            }

            let stake_in_global_pool =
                math::calculate_global_pool_shares(yield_farm.total_valued_shares, yield_farm.multiplier)
                    .map_err(|_| ArithmeticError::Overflow)?;
            let rewards = Self::claim_from_global_farm(global_farm, yield_farm, stake_in_global_pool)?;
            Self::update_yield_farm(
                yield_farm,
                rewards,
                current_period,
                global_farm.id,
                global_farm.reward_currency,
            )?;
        }
        Ok(())
    }

    // Claiming from `YieldFarm` is not possible(will fail) if yield farm is destroyed or has no
    // entries.
    fn is_yield_farm_claimable(
        global_farm_id: GlobalFarmId,
        yield_farm_id: YieldFarmId,
        amm_pool_id: T::AmmPoolId,
    ) -> bool {
        if let Some(yield_farm) = Self::yield_farm((amm_pool_id, global_farm_id, yield_farm_id)) {
            return !yield_farm.is_deleted() && yield_farm.has_entries();
        }

        false
    }

    // This function resturs `GlobalFarmId` from deposit's farm entry or `None` if deposit or farm
    // entry doesn't exists.
    fn get_global_farm_id(id: DepositId, yield_farm_id: YieldFarmId) -> Option<GlobalFarmId> {
        if let Some(mut deposit) = Self::deposit(id) {
            if let Some(farm_entry) = deposit.get_yield_farm_entry(yield_farm_id) {
                return Some(farm_entry.global_farm_id);
            }
        }

        None
    }
}

impl<T: Config> hydradx_traits::liquidity_mining::Mutate<AccountIdOf<T>, T::CurrencyId, BlockNumberFor<T>>
    for Pallet<T>
{
    type Error = DispatchError;

    type AmmPoolId = T::AmmPoolId;
    type Balance = Balance;
    type Period = PeriodOf<T>;
    type LoyaltyCurve = LoyaltyCurve;

    fn create_global_farm(
        total_rewards: Self::Balance,
        planned_yielding_periods: Self::Period,
        blocks_per_period: BlockNumberFor<T>,
        incentivized_asset: T::CurrencyId,
        reward_currency: T::CurrencyId,
        owner: AccountIdOf<T>,
        yield_per_period: Permill,
        min_deposit: Self::Balance,
        price_adjustment: FixedU128,
    ) -> Result<(u32, Self::Balance), Self::Error> {
        Self::create_global_farm(
            total_rewards,
            planned_yielding_periods,
            blocks_per_period,
            incentivized_asset,
            reward_currency,
            owner,
            yield_per_period,
            min_deposit,
            price_adjustment,
        )
    }

    fn destroy_global_farm(
        who: AccountIdOf<T>,
        global_farm_id: u32,
    ) -> Result<(T::CurrencyId, Self::Balance, AccountIdOf<T>), Self::Error> {
        Self::destroy_global_farm(who, global_farm_id)
    }

    fn create_yield_farm(
        who: AccountIdOf<T>,
        global_farm_id: u32,
        multiplier: FixedU128,
        loyalty_curve: Option<Self::LoyaltyCurve>,
        amm_pool_id: Self::AmmPoolId,
        asset_a: T::CurrencyId,
        asset_b: T::CurrencyId,
    ) -> Result<u32, Self::Error> {
        Self::create_yield_farm(
            who,
            global_farm_id,
            multiplier,
            loyalty_curve,
            amm_pool_id,
            asset_a,
            asset_b,
        )
    }

    fn update_yield_farm_multiplier(
        who: AccountIdOf<T>,
        global_farm_id: u32,
        amm_pool_id: Self::AmmPoolId,
        multiplier: FixedU128,
    ) -> Result<u32, Self::Error> {
        Self::update_yield_farm_multiplier(who, global_farm_id, amm_pool_id, multiplier)
    }

    fn stop_yield_farm(
        who: AccountIdOf<T>,
        global_farm_id: u32,
        amm_pool_id: Self::AmmPoolId,
    ) -> Result<u32, Self::Error> {
        Self::stop_yield_farm(who, global_farm_id, amm_pool_id)
    }

    fn resume_yield_farm(
        who: AccountIdOf<T>,
        global_farm_id: u32,
        yield_farm_id: u32,
        amm_pool_id: Self::AmmPoolId,
        multiplier: FixedU128,
    ) -> Result<(), Self::Error> {
        Self::resume_yield_farm(who, global_farm_id, yield_farm_id, amm_pool_id, multiplier)
    }

    fn destroy_yield_farm(
        who: AccountIdOf<T>,
        global_farm_id: u32,
        yield_farm_id: u32,
        amm_pool_id: Self::AmmPoolId,
    ) -> Result<(), Self::Error> {
        Self::destroy_yield_farm(who, global_farm_id, yield_farm_id, amm_pool_id)
    }

    fn deposit_lp_shares(
        global_farm_id: u32,
        yield_farm_id: u32,
        amm_pool_id: Self::AmmPoolId,
        shares_amount: Self::Balance,
        get_balance_in_amm: fn(T::CurrencyId, Self::AmmPoolId) -> Result<Self::Balance, Self::Error>,
    ) -> Result<u128, Self::Error> {
        Self::deposit_lp_shares(
            global_farm_id,
            yield_farm_id,
            amm_pool_id,
            shares_amount,
            get_balance_in_amm,
        )
    }

    fn redeposit_lp_shares(
        global_farm_id: u32,
        yield_farm_id: u32,
        deposit_id: u128,
        get_balance_in_amm: fn(T::CurrencyId, Self::AmmPoolId) -> Result<Self::Balance, Self::Error>,
    ) -> Result<Self::Balance, Self::Error> {
        Self::redeposit_lp_shares(global_farm_id, yield_farm_id, deposit_id, get_balance_in_amm)
    }

    fn claim_rewards(
        who: AccountIdOf<T>,
        deposit_id: u128,
        yield_farm_id: u32,
        fail_on_doubleclaim: bool,
    ) -> Result<(u32, T::CurrencyId, Self::Balance, Self::Balance), Self::Error> {
        Self::claim_rewards(who, deposit_id, yield_farm_id, fail_on_doubleclaim)
    }

    fn withdraw_lp_shares(
        deposit_id: u128,
        yield_farm_id: u32,
        unclaimable_rewards: Self::Balance,
    ) -> Result<(u32, Self::Balance, bool), Self::Error> {
        Self::withdraw_lp_shares(deposit_id, yield_farm_id, unclaimable_rewards)
    }

    fn is_yield_farm_claimable(global_farm_id: u32, yield_farm_id: u32, amm_pool_id: Self::AmmPoolId) -> bool {
        Self::is_yield_farm_claimable(global_farm_id, yield_farm_id, amm_pool_id)
    }

    fn get_global_farm_id(deposit_id: u128, yield_farm_id: u32) -> Option<u32> {
        Self::get_global_farm_id(deposit_id, yield_farm_id)
    }
}
