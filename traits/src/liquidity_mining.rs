use sp_arithmetic::{FixedU128, Perquintill};

/// Trait for providing interface for liquidity mining.
pub trait Mutate<AccountId, AssetId, BlockNumber> {
    type Error;

    type AmmPoolId;
    type Balance;
    type Period;
    type LoyaltyCurve;

    /// Create new global farm.
    #[allow(clippy::too_many_arguments)]
    fn create_global_farm(
        total_rewards: Self::Balance,
        planned_yielding_periods: Self::Period,
        blocks_per_period: BlockNumber,
        incentivized_asset: AssetId,
        reward_currency: AssetId,
        owner: AccountId,
        yield_per_period: Perquintill,
        min_deposit: Self::Balance,
        price_adjustment: FixedU128,
    ) -> Result<(u32, Self::Balance), Self::Error>;

    /// Update price adjustment of the existing global farm.
    fn update_global_farm_price_adjustment(
        who: AccountId,
        global_farm_id: u32,
        price_adjustment: FixedU128,
    ) -> Result<(), Self::Error>;

    /// Destroy existing global farm.
    fn destroy_global_farm(
        who: AccountId,
        global_farm_id: u32,
    ) -> Result<(AssetId, Self::Balance, AccountId), Self::Error>;

    /// Crate new yield farm in the global farm.
    fn create_yield_farm(
        who: AccountId,
        global_farm_id: u32,
        multiplier: FixedU128,
        loyalty_curve: Option<Self::LoyaltyCurve>,
        amm_pool_id: Self::AmmPoolId,
        asset_a: AssetId,
        asset_b: AssetId,
    ) -> Result<u32, Self::Error>;

    /// Update multiplier of the existing yield farm.
    fn update_yield_farm_multiplier(
        who: AccountId,
        global_farm_id: u32,
        amm_pool_id: Self::AmmPoolId,
        multiplier: FixedU128,
    ) -> Result<u32, Self::Error>;

    /// Stop yield farming for amm pool in the global farm.
    fn stop_yield_farm(who: AccountId, global_farm_id: u32, amm_pool_id: Self::AmmPoolId) -> Result<u32, Self::Error>;

    /// Resume yield farming for amm pool in the global farm.
    fn resume_yield_farm(
        who: AccountId,
        global_farm_id: u32,
        yield_farm_id: u32,
        amm_pool_id: Self::AmmPoolId,
        multiplier: FixedU128,
    ) -> Result<(), Self::Error>;

    /// Destroy existing yield farm.
    fn destroy_yield_farm(
        who: AccountId,
        global_farm_id: u32,
        yield_farm_id: u32,
        amm_pool_id: Self::AmmPoolId,
    ) -> Result<(), Self::Error>;

    /// Deposit new LP shares.
    #[allow(clippy::type_complexity)]
    fn deposit_lp_shares(
        global_farm_id: u32,
        yield_farm_id: u32,
        amm_pool_id: Self::AmmPoolId,
        shares_amount: Self::Balance,
        get_balance_in_amm: fn(AssetId, Self::AmmPoolId) -> Result<Self::Balance, Self::Error>,
    ) -> Result<u128, Self::Error>;

    /// Redeposit already locked LP shares to another yield farm.
    #[allow(clippy::type_complexity)]
    fn redeposit_lp_shares(
        global_farm_id: u32,
        yield_farm_id: u32,
        deposit_id: u128,
        get_balance_in_amm: fn(AssetId, Self::AmmPoolId) -> Result<Self::Balance, Self::Error>,
    ) -> Result<Self::Balance, Self::Error>;

    /// Claim rewards for given deposit.
    #[allow(clippy::type_complexity)]
    fn claim_rewards(
        who: AccountId,
        deposit_id: u128,
        yield_farm_id: u32,
        fail_on_doubleclaim: bool,
    ) -> Result<(u32, AssetId, Self::Balance, Self::Balance), Self::Error>;

    /// Withdraw LP shares from yield farm.
    fn withdraw_lp_shares(
        deposit_id: u128,
        yield_farm_id: u32,
        unclaimable_rewards: Self::Balance,
    ) -> Result<(u32, Self::Balance, bool), Self::Error>;

    /// Returns true if rewards claiming from yield farm is possible.
    fn is_yield_farm_claimable(global_farm_id: u32, yield_farm_id: u32, amm_pool_id: Self::AmmPoolId) -> bool;

    /// Returns `Some(global_farm_id)` for given `deposit_id` and `yield_farm_id` or `None`.
    fn get_global_farm_id(deposit_id: u128, yield_farm_id: u32) -> Option<u32>;
}
