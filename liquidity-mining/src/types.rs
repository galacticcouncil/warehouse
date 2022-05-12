use super::*;

pub type Balance = u128;
pub type PoolId = u32;
pub type GlobalPoolId = PoolId;
pub type PoolMultiplier = FixedU128;
pub type DepositId = u128;

/// This struct represents the state a of single liquidity mining program. `LiquidityPoolYieldFarm`s are rewarded from
/// `GlobalPool` based on their stake in `GlobalPool`. `LiquidityPoolYieldFarm` stake in `GlobalPool` is derived from
/// users stake in `LiquidityPoolYieldFarm`.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebugNoBound, TypeInfo)]
pub struct GlobalPool<T: Config> {
    pub(super) id: GlobalPoolId,
    pub(super) owner: AccountIdOf<T>,
    pub(super) updated_at: PeriodOf<T>,
    pub(super) total_shares_z: Balance,
    pub(super) accumulated_rpz: Balance,
    pub(super) reward_currency: AssetIdOf<T>,
    pub(super) accumulated_rewards: Balance,
    pub(super) paid_accumulated_rewards: Balance,
    pub(super) yield_per_period: Permill,
    pub(super) planned_yielding_periods: PeriodOf<T>,
    pub(super) blocks_per_period: BlockNumberFor<T>,
    pub(super) incentivized_asset: AssetIdOf<T>,
    pub(super) max_reward_per_period: Balance,
    pub(super) liq_pools_count: u32,
}

impl<T: Config> GlobalPool<T> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: GlobalPoolId,
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
            liq_pools_count: Zero::zero(),
            id,
            updated_at,
            reward_currency,
            yield_per_period,
            planned_yielding_periods,
            blocks_per_period,
            owner,
            incentivized_asset,
            max_reward_per_period,
        }
    }
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebugNoBound, TypeInfo)]
pub struct LiquidityPoolYieldFarm<T: Config> {
    pub(super) id: PoolId,
    pub(super) updated_at: PeriodOf<T>,
    pub(super) total_shares: Balance,
    pub(super) total_valued_shares: Balance,
    pub(super) accumulated_rpvs: Balance,
    pub(super) accumulated_rpz: Balance,
    pub(super) loyalty_curve: Option<LoyaltyCurve>,
    pub(super) stake_in_global_pool: Balance, //NOTE: may be replaced with: total_valued_shares * multiplier
    pub(super) multiplier: PoolMultiplier,
    pub(super) canceled: bool,
}

impl<T: Config> LiquidityPoolYieldFarm<T> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        id: PoolId,
        updated_at: PeriodOf<T>,
        loyalty_curve: Option<LoyaltyCurve>,
        multiplier: PoolMultiplier,
    ) -> Self {
        Self {
            accumulated_rpvs: Zero::zero(),
            accumulated_rpz: Zero::zero(),
            stake_in_global_pool: Zero::zero(),
            total_shares: Zero::zero(),
            total_valued_shares: Zero::zero(),
            canceled: false,
            id,
            updated_at,
            loyalty_curve,
            multiplier,
        }
    }
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct LoyaltyCurve {
    pub(super) initial_reward_percentage: FixedU128,
    pub(super) scale_coef: u32,
}

impl Default for LoyaltyCurve {
    fn default() -> Self {
        Self {
            initial_reward_percentage: FixedU128::from_inner(500_000_000_000_000_000), // 0.5
            scale_coef: 100,
        }
    }
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebugNoBound, TypeInfo)]
pub struct Deposit<T: Config> {
    pub(super) shares: Balance,
    pub(super) valued_shares: Balance,
    pub(super) accumulated_rpvs: Balance,
    pub(super) accumulated_claimed_rewards: Balance,
    pub(super) entered_at: PeriodOf<T>,
    pub(super) updated_at: PeriodOf<T>,
}

impl<T: Config> Deposit<T> {
    pub(super) fn new(
        shares: Balance,
        valued_shares: Balance,
        accumulated_rpvs: Balance,
        entered_at: PeriodOf<T>,
    ) -> Self {
        Self {
            updated_at: entered_at,
            entered_at,
            shares,
            valued_shares,
            accumulated_rpvs,
            accumulated_claimed_rewards: Zero::zero(),
        }
    }
}
