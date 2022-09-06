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
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T, I))]
pub struct GlobalFarmData<T: Config<I>, I: 'static = ()> {
    pub(super) id: GlobalFarmId,
    pub(super) owner: T::AccountId,
    pub(super) updated_at: PeriodOf<T>,
    pub(super) total_shares_z: Balance,
    pub(super) accumulated_rpz: FixedU128,
    pub(super) reward_currency: T::AssetId,
    pub(super) accumulated_rewards: Balance,
    pub(super) paid_accumulated_rewards: Balance,
    pub(super) yield_per_period: Perquintill,
    pub(super) planned_yielding_periods: PeriodOf<T>,
    pub(super) blocks_per_period: BlockNumberFor<T>,
    pub(super) incentivized_asset: T::AssetId,
    pub(super) max_reward_per_period: Balance,
    // min. LP shares user must deposit to start yield farming.
    pub(super) min_deposit: Balance,
    //live counts includes `active` and `stopped` yield farms.
    //total count includes `active`, `stopped`, `deleted` - this count is decreased only if yield
    //farm is flushed from storage.
    pub(super) yield_farms_count: (u32, u32), //`(live farms count, total farms count)`
    pub(super) price_adjustment: FixedU128,
    pub(super) state: FarmState,
}

impl<T: Config<I>, I: 'static> GlobalFarmData<T, I> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: GlobalFarmId,
        updated_at: PeriodOf<T>,
        reward_currency: T::AssetId,
        yield_per_period: Perquintill,
        planned_yielding_periods: PeriodOf<T>,
        blocks_per_period: T::BlockNumber,
        owner: T::AccountId,
        incentivized_asset: T::AssetId,
        max_reward_per_period: Balance,
        min_deposit: Balance,
        price_adjustment: FixedU128,
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
            min_deposit,
            price_adjustment,
            state: FarmState::Active,
        }
    }

    pub fn live_farms_count(&self) -> u32 {
        self.yield_farms_count.0
    }

    pub fn total_farms_count(&self) -> u32 {
        self.yield_farms_count.1
    }

    /// This function updates yields_farm_count when new yield farm is added into the global farm.
    /// This function should be called only when new yield farm is created/added into the global
    /// farm.
    pub fn increase_yield_farm_counts(&mut self) -> Result<(), ArithmeticError> {
        self.yield_farms_count = (
            self.live_farms_count()
                .checked_add(1)
                .ok_or(ArithmeticError::Overflow)?,
            self.total_farms_count()
                .checked_add(1)
                .ok_or(ArithmeticError::Overflow)?,
        );

        Ok(())
    }

    /// This function updates `yield_farms_count` when yield farm is removed from global farm.
    /// This function should be called only when yield farm is removed from global farm.
    pub fn decrease_live_yield_farm_count(&mut self) -> Result<(), ArithmeticError> {
        // Note: only live count should change
        self.yield_farms_count.0 = self
            .live_farms_count()
            .checked_sub(1)
            .ok_or(ArithmeticError::Underflow)?;

        Ok(())
    }

    /// This function updates `yield_farms_count` when yield farm is flushed from storage.
    /// This function should be called only if yield farm is flushed.
    /// !!! DON'T call this function if yield farm is in stopped or deleted.
    pub fn decrease_total_yield_farm_count(&mut self) -> Result<(), DispatchError> {
        self.yield_farms_count.1 = self
            .total_farms_count()
            .checked_sub(1)
            .ok_or(ArithmeticError::Underflow)?;

        Ok(())
    }

    /// Function returns `true` if global farm has live yield farms.
    pub fn has_live_farms(&self) -> bool {
        !self.live_farms_count().is_zero()
    }

    /// Function return `true` if global farm can be flushed(removed) from storage.
    pub fn can_be_flushed(&self) -> bool {
        //farm can be flushed only if all yield farms are flushed.
        self.state == FarmState::Deleted && self.total_farms_count().is_zero()
    }

    /// Function return `true` if global farm is in active state.
    pub fn is_active(&self) -> bool {
        self.state == FarmState::Active
    }

    /// This function returns `true` if farm has no capacity for next yield farm(yield farm can't
    /// be added into global farm until some yield farm is not removed from storage).
    pub fn is_full(&self) -> bool {
        self.total_farms_count().ge(&<T>::MaxYieldFarmsPerGlobalFarm::get())
    }
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T, I))]
pub struct YieldFarmData<T: Config<I>, I: 'static = ()> {
    pub(super) id: FarmId,
    pub(super) updated_at: PeriodOf<T>,
    pub(super) total_shares: Balance,
    pub(super) total_valued_shares: Balance,
    pub(super) accumulated_rpvs: FixedU128,
    pub(super) accumulated_rpz: FixedU128,
    pub(super) loyalty_curve: Option<LoyaltyCurve>,
    pub(super) multiplier: FarmMultiplier,
    pub(super) state: FarmState,
    pub(super) entries_count: u64,
    pub(super) _phantom: PhantomData<I>, //pub because of tests
}

impl<T: Config<I>, I: 'static> YieldFarmData<T, I> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
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
            state: FarmState::Active,
            entries_count: Default::default(),
            _phantom: PhantomData::default(),
        }
    }

    /// Function returns `true` if yield farm is in active state.
    pub fn is_active(&self) -> bool {
        self.state == FarmState::Active
    }

    /// Function returns `true` if yield farm is in stopped state.
    pub fn is_stopped(&self) -> bool {
        self.state == FarmState::Stopped
    }

    /// Function returns `true` if yield farm is in deleted state.
    pub fn is_deleted(&self) -> bool {
        self.state == FarmState::Deleted
    }

    /// Returns `true` if yield farm can be removed from storage, `false` otherwise.
    pub fn can_be_flushed(&self) -> bool {
        self.state == FarmState::Deleted && self.entries_count.is_zero()
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

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T, I))]
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

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, MaxEncodedLen)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T, I))]
pub struct DepositData<T: Config<I>, I: 'static = ()> {
    pub(super) shares: Balance,
    pub(super) amm_pool_id: T::AmmPoolId,
    // NOTE: Capacity of this vector MUST BE at least 1.
    pub(super) yield_farm_entries: BoundedVec<YieldFarmEntry<T, I>, T::MaxFarmEntriesPerDeposit>,
}

impl<T: Config<I>, I: 'static> DepositData<T, I> {
    pub fn new(shares: Balance, amm_pool_id: T::AmmPoolId) -> Self {
        Self {
            shares,
            amm_pool_id,
            //NOTE: Capacity of this vector MUST BE at least 1.
            yield_farm_entries: BoundedVec::default(),
        }
    }

    /// This function add new yield farm entry into the deposit.
    /// This function returns error if deposit reached max entries in the deposit or
    /// `entry.yield_farm_id` is not unique.
    pub fn add_yield_farm_entry(&mut self, entry: YieldFarmEntry<T, I>) -> Result<(), DispatchError> {
        if self.search_yield_farm_entry(entry.yield_farm_id).is_some() {
            return Err(Error::<T, I>::DoubleLock.into());
        }

        self.yield_farm_entries
            .try_push(entry)
            .map_err(|_| Error::<T, I>::MaxEntriesPerDeposit)?;

        Ok(())
    }

    /// This function remove yield farm entry from the deposit. This function returns error if
    /// yield farm entry in not found in the deposit.
    pub fn remove_yield_farm_entry(&mut self, yield_farm_id: YieldFarmId) -> Result<YieldFarmEntry<T, I>, Error<T, I>> {
        if let Some(idx) = self.search_yield_farm_entry(yield_farm_id) {
            return Ok(self.yield_farm_entries.swap_remove(idx));
        }

        Err(Error::<T, I>::YieldFarmEntryNotFound)
    }

    /// This function return yield farm entry from deposit of `None` if yield farm entry is not
    /// found.
    pub fn get_yield_farm_entry(&mut self, yield_farm_id: YieldFarmId) -> Option<&mut YieldFarmEntry<T, I>> {
        if let Some(idx) = self.search_yield_farm_entry(yield_farm_id) {
            return self.yield_farm_entries.get_mut(idx);
        }

        None
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

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T, I))]
pub struct YieldFarmEntry<T: Config<I>, I: 'static = ()> {
    pub(super) global_farm_id: GlobalFarmId,
    pub(super) yield_farm_id: YieldFarmId,
    pub(super) valued_shares: Balance,
    pub(super) accumulated_rpvs: FixedU128,
    pub(super) accumulated_claimed_rewards: Balance,
    pub(super) entered_at: PeriodOf<T>,
    pub(super) updated_at: PeriodOf<T>,
    pub(super) _phantom: PhantomData<I>, //pub because of tests
}

impl<T: Config<I>, I: 'static> YieldFarmEntry<T, I> {
    pub fn new(
        global_farm_id: GlobalFarmId,
        yield_farm_id: YieldFarmId,
        valued_shares: Balance,
        accumulated_rpvs: FixedU128,
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
            _phantom: PhantomData,
        }
    }
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum FarmState {
    Active,
    Stopped,
    Deleted,
}
