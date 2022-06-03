use super::*;
use sp_std::vec;

pub type Balance = u128;
pub type FarmId = u32;
pub type GlobalFarmId = FarmId;
pub type YieldFarmId = FarmId;
pub type FarmMultiplier = FixedU128;
pub type DepositId = u128;

/// This type represent number of live(active and stopped)` yiled farms in global farm.
pub type LiveFarmsCount = u32;
/// This type represent number of total(active, stopped and deleted)` yiled farms in global farm.
pub type TotalFarmsCount = u32;

/// This struct represents the state a of single liquidity mining program. `YieldFarm`s are rewarded from
/// `GlobalFarm` based on their stake in `GlobalFarm`. `YieldFarm` stake in `GlobalFarm` is derived from
/// users stake in `YieldFarm`.
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
    pub yield_farms_count: (LiveFarmsCount, TotalFarmsCount), //`TotalFarmsCount` includes active, stopped and deleted. Total cound is decreased only if yiled farms is flushed.  `ExistingFarmsCount` includes `active` and `stopped` yield farms
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

    /// Fn update `yield_farms_count`(both `ActiveFarmsCount` and `TotalFarmsCount`). This fn
    /// should be called when new `YieldFarm` is added/created in the `GlobalFarm`
    pub fn yield_farm_added(&mut self) -> Result<(), Error<T>> {
        self.yield_farms_count = (
            self.yield_farms_count.0.checked_add(1).ok_or(Error::<T>::Overflow)?,
            self.yield_farms_count.1.checked_add(1).ok_or(Error::<T>::Overflow)?,
        );

        Ok(())
    }

    /// Fn decrement `LiveFarmsCount`. `YieldFarm` is considered NOT LIVE only if it's in the
    /// `deleted` state. `stopped` farms are considered live because yield farming can be resumed.
    pub fn yield_farm_removed(&mut self) -> Result<(), Error<T>> {
        self.yield_farms_count.0 = self.yield_farms_count.0.checked_sub(1).ok_or(Error::<T>::Overflow)?;

        Ok(())
    }

    /// This fn change(decrement) `TotalFarmsCount` in the `GlobaFarm` and should be called only if
    /// `YieldFarm` was removed from storage.
    /// DON'T call this fn when `YieldFarm` is `Stopped` or `Deleted`
    pub fn yield_farm_flushed(&mut self) -> Result<(), Error<T>> {
        self.yield_farms_count.1 = self.yield_farms_count.1.checked_sub(1).ok_or(Error::<T>::Overflow)?;

        Ok(())
    }

    pub fn has_no_live_farms(&self) -> bool {
        self.yield_farms_count.0.is_zero()
    }

    pub fn can_be_flushed(&self) -> bool {
        //farm can be flushed only if all `YieldFarm`s are flushed.
        self.state == GlobalFarmState::Deleted && self.yield_farms_count.1.is_zero()
    }

    pub fn is_active(&self) -> bool {
        self.state == GlobalFarmState::Active
    }
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebugNoBound, TypeInfo)]
pub struct YieldFarmData<T: Config> {
    pub id: FarmId,
    pub updated_at: PeriodOf<T>,
    pub total_shares: Balance, //try to remove this.
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

    pub fn is_active(&self) -> bool {
        self.state == YieldFarmState::Active
    }

    pub fn is_stopped(&self) -> bool {
        self.state == YieldFarmState::Stopped
    }

    pub fn is_deleted(&self) -> bool {
        self.state == YieldFarmState::Deleted
    }

    /// Returns `true` if yield farm can be removed from storage, `false` otherwise.
    pub fn can_be_flushed(&self) -> bool {
        self.state == YieldFarmState::Deleted && self.entries_count.is_zero()
    }

    pub fn entry_removed(&mut self) -> Result<(), Error<T>> {
        self.entries_count = self.entries_count.checked_sub(1).ok_or(Error::<T>::Overflow)?;

        Ok(())
    }

    pub fn entry_added(&mut self) -> Result<(), Error<T>> {
        self.entries_count = self.entries_count.checked_add(1).ok_or(Error::<T>::Overflow)?;

        Ok(())
    }

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
    //NOTE: capacity of yield_farm_entries always MUST BE at least 1.
    pub yield_farm_entries: Vec<YieldFarmEntry<T>>,
}

impl<T: Config> DepositData<T> {
    pub fn new(shares: Balance, amm_pool_id: T::AmmPoolId) -> Self {
        Self {
            shares,
            amm_pool_id,
            yield_farm_entries: vec![],
            //NOTE: capacity of `yield_farm_entries` MUST BE always at least 1.
        }
    }

    pub fn add_yield_farm_entry(&mut self, entry: YieldFarmEntry<T>) -> Result<(), Error<T>> {
        let len = TryInto::<u8>::try_into(self.yield_farm_entries.len()).map_err(|_e| Error::<T>::Overflow)?;
        if len >= T::MaxFarmEntriesPerDeposit::get() {
            return Err(Error::<T>::MaxEntriesPerDeposit);
        }

        let idx = match self
            .yield_farm_entries
            .binary_search_by(|e| e.yield_farm_id.cmp(&entry.yield_farm_id))
        {
            Ok(_) => return Err(Error::<T>::DoubleLock),
            Err(idx) => idx,
        };

        self.yield_farm_entries.insert(idx, entry);

        Ok(())
    }

    pub fn remove_yield_farm_entry(&mut self, yield_farm_id: FarmId) -> Result<YieldFarmEntry<T>, Error<T>> {
        let idx = match self
            .yield_farm_entries
            .binary_search_by(|e| e.yield_farm_id.cmp(&yield_farm_id))
        {
            Ok(idx) => idx,
            Err(_) => return Err(Error::<T>::YieldFarmEntryNotFound),
        };

        Ok(self.yield_farm_entries.remove(idx))
    }

    pub fn get_yield_farm_entry(&mut self, yield_farm_id: FarmId) -> Option<&mut YieldFarmEntry<T>> {
        match self
            .yield_farm_entries
            .binary_search_by(|e| e.yield_farm_id.cmp(&yield_farm_id))
        {
            Ok(idx) => self.yield_farm_entries.get_mut(idx),
            Err(_) => None,
        }
    }

    pub fn contains_yield_farm_entry(&self, yield_farm_id: FarmId) -> bool {
        self.yield_farm_entries
            .binary_search_by(|e| e.yield_farm_id.cmp(&yield_farm_id))
            .is_ok()
    }

    pub fn has_no_yield_farm_entries(&self) -> bool {
        self.yield_farm_entries.is_empty()
    }

    /// This fn return `true` if deposit can be flushed from storage.
    pub fn can_be_flushed(&self) -> bool {
        //NOTE: deposit with no entries should/must be flushed
        self.has_no_yield_farm_entries()
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
