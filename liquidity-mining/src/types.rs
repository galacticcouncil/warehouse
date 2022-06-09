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

use super::*;
use sp_std::vec;

pub type Balance = u128;
pub type FarmId = u32;
pub type GlobalFarmId = FarmId;
pub type YieldFarmId = FarmId;
pub type FarmMultiplier = FixedU128;
pub type DepositId = u128;

/// This struct represents the state a of single liquidity mining program. `YieldFarm`s are rewarded from
/// `GlobalFarm` based on their stake in `GlobalFarm`. `YieldFarm` stake in `GlobalFarm` is derived from
/// users stake in `YieldFarm`.
/// Yield farm is considered live from global farm view if yield farm is `active` or `stopped`.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebugNoBound, TypeInfo)]
pub struct GlobalFarmData<T: Config> {
    pub id: GlobalFarmId,
    pub owner: AccountIdOf<T>,
    pub updated_at: PeriodOf<T>,
    pub total_shares_z: Balance,
    pub accumulated_rpz: Balance,
    pub reward_currency: AssetIdOf<T>,
    pub accumulated_rewards: Balance,
    pub paid_accumulated_rewards: Balance,
    pub yield_per_period: Permill,
    pub planned_yielding_periods: PeriodOf<T>,
    pub blocks_per_period: BlockNumberFor<T>,
    pub incentivized_asset: AssetIdOf<T>,
    pub max_reward_per_period: Balance,
    //live counts includes `active` and `stopped` yield farms.
    //total count includes `active`, `stopped`, `deleted` - this count is decreased only if yield
    //farm is flushed from storage.
    pub yield_farms_count: (u32, u32), //`(live count, total count)`
    pub state: GlobalFarmState,
}

impl<T: Config> GlobalFarmData<T> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: GlobalFarmId,
        updated_at: PeriodOf<T>,
        reward_currency: T::CurrencyId,
        yield_per_period: Permill,
        planned_yielding_periods: PeriodOf<T>,
        blocks_per_period: T::BlockNumber,
        owner: AccountIdOf<T>,
        incentivized_asset: T::CurrencyId,
        max_reward_per_period: Balance,
    ) -> Self {
        Self {
            accumulated_rewards: Zero::zero(),
            accumulated_rpz: Zero::zero(),
            paid_accumulated_rewards: Zero::zero(),
            total_shares_z: Zero::zero(),
            yield_farms_count: (Zero::zero(), Zero::zero()),
            id,
            updated_at,
            reward_currency,
            yield_per_period,
            planned_yielding_periods,
            blocks_per_period,
            owner,
            incentivized_asset,
            max_reward_per_period,
            state: GlobalFarmState::Active,
        }
    }

    /// This function updates yields_farm_count when new yield farm is added into the global farm.
    /// This function should be called only when new yield farm is created/added into the global
    /// farm.
    pub fn increase_yield_farm_counts(&mut self) -> Result<(), ArithmeticError> {
        self.yield_farms_count = (
            self.yield_farms_count
                .0
                .checked_add(1)
                .ok_or(ArithmeticError::Overflow)?,
            self.yield_farms_count
                .1
                .checked_add(1)
                .ok_or(ArithmeticError::Overflow)?,
        );

        Ok(())
    }

    /// This function updates `yield_farms_count` when yield farm is removed from global farm.
    /// This function should be called only when yield farm is removed from global farm.
    pub fn decrease_live_yield_farm_count(&mut self) -> Result<(), ArithmeticError> {
        //Note: only live count should change
        self.yield_farms_count.0 = self
            .yield_farms_count
            .0
            .checked_sub(1)
            .ok_or(ArithmeticError::Underflow)?;

        Ok(())
    }

    /// This function updates `yield_farms_count` when yield farm is flushed from storage.
    /// This function should be called only if yield farm is flushed.
    /// !!! DON'T call this function if yield farm is in stopped or deleted.
    pub fn decrease_total_yield_farm_count(&mut self) -> Result<(), DispatchError> {
        self.yield_farms_count.1 = self
            .yield_farms_count
            .1
            .checked_sub(1)
            .ok_or(ArithmeticError::Underflow)?;

        Ok(())
    }

    /// Function returns `true` if global farm has live yield farms.
    pub fn has_live_farms(&self) -> bool {
        !self.yield_farms_count.0.is_zero()
    }

    /// Function return `true` if global farm can be flushed(removed) from storage.
    pub fn can_be_flushed(&self) -> bool {
        //farm can be flushed only if all yield farms are flushed.
        self.state == GlobalFarmState::Deleted && self.yield_farms_count.1.is_zero()
    }

    /// Function return `true` if global farm is in active state.
    pub fn is_active(&self) -> bool {
        self.state == GlobalFarmState::Active
    }
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebugNoBound, TypeInfo)]
pub struct YieldFarmData<T: Config> {
    pub id: FarmId,
    pub updated_at: PeriodOf<T>,
    pub total_shares: Balance,
    pub total_valued_shares: Balance,
    pub accumulated_rpvs: Balance,
    pub accumulated_rpz: Balance,
    pub loyalty_curve: Option<LoyaltyCurve>,
    pub multiplier: FarmMultiplier,
    pub state: YieldFarmState,
    pub entries_count: u64,
}

impl<T: Config> YieldFarmData<T> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        id: FarmId,
        updated_at: PeriodOf<T>,
        loyalty_curve: Option<LoyaltyCurve>,
        multiplier: FarmMultiplier,
    ) -> Self {
        Self {
            id,
            updated_at,
            loyalty_curve,
            multiplier,
            accumulated_rpvs: Zero::zero(),
            accumulated_rpz: Zero::zero(),
            total_shares: Zero::zero(),
            total_valued_shares: Zero::zero(),
            state: YieldFarmState::Active,
            entries_count: Default::default(),
        }
    }

    /// Function returns `true` if yield farm is in active state.
    pub fn is_active(&self) -> bool {
        self.state == YieldFarmState::Active
    }

    /// Function returns `true` if yield farm is in stopped state.
    pub fn is_stopped(&self) -> bool {
        self.state == YieldFarmState::Stopped
    }

    /// Function returns `true` if yield farm is in deleted state.
    pub fn is_deleted(&self) -> bool {
        self.state == YieldFarmState::Deleted
    }

    /// Returns `true` if yield farm can be removed from storage, `false` otherwise.
    pub fn can_be_flushed(&self) -> bool {
        self.state == YieldFarmState::Deleted && self.entries_count.is_zero()
    }

    /// This function updates entries count in the yield farm. This function should be called if  
    /// entry is removed from the yield farm.
    pub fn decrease_entries_count(&mut self) -> Result<(), ArithmeticError> {
        self.entries_count = self.entries_count.checked_sub(1).ok_or(ArithmeticError::Underflow)?;

        Ok(())
    }

    /// This function updates entries count in the yield farm. This function should be called if
    /// entry is added into the yield farm.
    pub fn increase_entries_count(&mut self) -> Result<(), ArithmeticError> {
        self.entries_count = self.entries_count.checked_add(1).ok_or(ArithmeticError::Overflow)?;

        Ok(())
    }

    /// This function return `true` if yield farm is empty.
    pub fn has_entries(&self) -> bool {
        !self.entries_count.is_zero()
    }
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct LoyaltyCurve {
    pub initial_reward_percentage: FixedU128,
    pub scale_coef: u32,
}

impl Default for LoyaltyCurve {
    fn default() -> Self {
        Self {
            initial_reward_percentage: FixedU128::from_inner(500_000_000_000_000_000), // 0.5
            scale_coef: 100,
        }
    }
}

#[derive(Clone, Encode, Decode, RuntimeDebugNoBound, TypeInfo, PartialEq)]
pub struct DepositData<T: Config> {
    pub shares: Balance,
    pub amm_pool_id: T::AmmPoolId,
    //NOTE: Capacity of this vector MUST BE at least 1.
    pub yield_farm_entries: Vec<YieldFarmEntry<T>>,
}

impl<T: Config> DepositData<T> {
    pub fn new(shares: Balance, amm_pool_id: T::AmmPoolId) -> Self {
        Self {
            shares,
            amm_pool_id,
            //NOTE: Capacity of this vector MUST BE at least 1.
            yield_farm_entries: vec![],
        }
    }

    /// This function add new yield farm entry into the deposit.
    /// This function returns error if deposit reached max entries in the deposit or
    /// `entry.yield_farm_id` is not unique.
    pub fn add_yield_farm_entry(&mut self, entry: YieldFarmEntry<T>) -> Result<(), DispatchError> {
        let len = TryInto::<u8>::try_into(self.yield_farm_entries.len()).map_err(|_e| ArithmeticError::Overflow)?;
        if len >= T::MaxFarmEntriesPerDeposit::get() {
            return Err(Error::<T>::MaxEntriesPerDeposit.into());
        }

        if self.search_yield_farm_entry(entry.yield_farm_id).is_some() {
            return Err(Error::<T>::DoubleLock.into());
        }

        self.yield_farm_entries.push(entry);

        Ok(())
    }

    /// This function remove yield farm entry from the deposit. This function returns error if
    /// yield farm entry in not found in the deposit.
    pub fn remove_yield_farm_entry(&mut self, yield_farm_id: YieldFarmId) -> Result<YieldFarmEntry<T>, Error<T>> {
        if let Some(idx) = self.search_yield_farm_entry(yield_farm_id) {
            return Ok(self.yield_farm_entries.swap_remove(idx));
        }

        Err(Error::<T>::YieldFarmEntryNotFound)
    }

    /// This function return yield farm entry from deposit of `None` if yield farm entry is not
    /// found.
    pub fn get_yield_farm_entry(&mut self, yield_farm_id: FarmId) -> Option<&mut YieldFarmEntry<T>> {
        self.yield_farm_entries
            .iter_mut()
            .find(|e| e.yield_farm_id == yield_farm_id)
    }

    /// This function returns `true` if deposit contains yield farm entry with given yield farm id.
    pub fn search_yield_farm_entry(&self, yield_farm_id: YieldFarmId) -> Option<usize> {
        self.yield_farm_entries
            .iter()
            .position(|e| e.yield_farm_id == yield_farm_id)
    }

    /// This function returns `true` if deposit can be flushed from storage.
    pub fn can_be_flushed(&self) -> bool {
        //NOTE: deposit with no entries should/must be flushed
        self.yield_farm_entries.is_empty()
    }
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebugNoBound, TypeInfo, MaxEncodedLen)]
pub struct YieldFarmEntry<T: Config> {
    pub global_farm_id: GlobalFarmId,
    pub yield_farm_id: FarmId,
    pub valued_shares: Balance,
    pub accumulated_rpvs: Balance,
    pub accumulated_claimed_rewards: Balance,
    pub entered_at: PeriodOf<T>,
    pub updated_at: PeriodOf<T>,
}

impl<T: Config> YieldFarmEntry<T> {
    pub fn new(
        global_farm_id: GlobalFarmId,
        yield_farm_id: YieldFarmId,
        valued_shares: Balance,
        accumulated_rpvs: Balance,
        entered_at: PeriodOf<T>,
    ) -> Self {
        Self {
            global_farm_id,
            yield_farm_id,
            valued_shares,
            accumulated_rpvs,
            accumulated_claimed_rewards: Zero::zero(),
            entered_at,
            updated_at: entered_at,
        }
    }
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebugNoBound, TypeInfo)]
pub enum GlobalFarmState {
    Active,
    Deleted,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebugNoBound, TypeInfo)]
pub enum YieldFarmState {
    Active,
    Stopped,
    Deleted,
}
