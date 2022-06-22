use sp_arithmetic::{FixedU128, Permill};

pub trait AmmProvider<AssetId, AmmPoolId, Balance> {
    /// Returns balance of asset in amm pool
    fn get_balance_in_amm(asset: AssetId, amm_pool: AmmPoolId) -> Balance;
}

pub trait OnUpdateHandler<GlobalFarmId, YieldFarmId, Balance> {
    /// This handler is called when accumulated rpz is updated.
    fn on_accumulated_rpz_update(global_farm_id: GlobalFarmId, accumulated_rpz: Balance, total_shares_z: Balance);

    /// This handler is called when accumulated rpvs is updated.
    fn on_accumulated_rpvs_update(
        global_farm_id: GlobalFarmId,
        yield_farm_id: YieldFarmId,
        accumulated_rpvs: Balance,
        total_valued_shares: Balance,
    );
}

pub trait LockableLpShares<AmmPoolId, AccountId, Balance, DepositId> {
    type Error;

    /// This function should lock LP shares.
    fn lock_lp_shares(
        amm_pool_id: AmmPoolId,
        who: AccountId,
        amount: Balance,
        deposit_id: DepositId,
    ) -> Result<(), Self::Error>;

    /// This function should unlock LP shares.
    fn unlock_lp_shares(
        amm_pool_id: AmmPoolId,
        who: AccountId,
        amount: Balance,
        deposit_id: DepositId,
    ) -> Result<(), Self::Error>;
}

pub trait Mutate<AccountId, AssetId, BlockNumber> {
    type Error;

    type AmmPoolId;
    type Balance;
    type Period;
    type LoyaltyCurve;

    #[allow(clippy::too_many_arguments)]
    fn create_global_farm(
        total_rewards: Self::Balance,
        planned_yielding_periods: Self::Period,
        blocks_per_period: BlockNumber,
        incentivized_asset: AssetId,
        reward_currency: AssetId,
        owner: AccountId,
        yield_per_period: Permill,
        min_deposit: Self::Balance,
        price_adjustment: FixedU128,
    ) -> Result<(u32, Self::Balance), Self::Error>;

    fn destroy_global_farm(
        who: AccountId,
        global_farm_id: u32,
    ) -> Result<(AssetId, Self::Balance, AccountId), Self::Error>;

    fn create_yield_farm(
        who: AccountId,
        global_farm_id: u32,
        multiplier: FixedU128,
        loyalty_curve: Option<Self::LoyaltyCurve>,
        amm_pool_id: Self::AmmPoolId,
        asset_a: AssetId,
        asset_b: AssetId,
    ) -> Result<u32, Self::Error>;

    fn update_yield_farm_multiplier(
        who: AccountId,
        global_farm_id: u32,
        amm_pool_id: Self::AmmPoolId,
        multiplier: FixedU128,
    ) -> Result<u32, Self::Error>;

    fn stop_yield_farm(who: AccountId, global_farm_id: u32, amm_pool_id: Self::AmmPoolId) -> Result<u32, Self::Error>;

    fn resume_yield_farm(
        who: AccountId,
        global_farm_id: u32,
        yield_farm_id: u32,
        amm_pool_id: Self::AmmPoolId,
        multiplier: FixedU128,
    ) -> Result<(), Self::Error>;

    fn destroy_yield_farm(
        who: AccountId,
        global_farm_id: u32,
        yield_farm_id: u32,
        amm_pool_id: Self::AmmPoolId,
    ) -> Result<(), Self::Error>;

    fn deposit_lp_shares(
        global_farm_id: u32,
        yield_farm_id: u32,
        amm_pool_id: Self::AmmPoolId,
        shares_amount: Self::Balance,
        get_balance_in_amm: fn(AssetId, Self::AmmPoolId) -> Self::Balance,
    ) -> Result<u128, Self::Error>;

    fn redeposit_lp_shares(
        global_farm_id: u32,
        yield_farm_id: u32,
        deposit_id: u128,
        get_balance_in_amm: fn(AssetId, Self::AmmPoolId) -> Self::Balance,
    ) -> Result<Self::Balance, Self::Error>;

    #[allow(clippy::type_complexity)]
    fn claim_rewards(
        who: AccountId,
        deposit_id: u128,
        yield_farm_id: u32,
        fail_on_doubleclaim: bool,
    ) -> Result<(u32, AssetId, Self::Balance, Self::Balance), Self::Error>;

    fn withdraw_lp_shares(
        deposit_id: u128,
        yield_farm_id: u32,
        unclaimable_rewards: Self::Balance,
    ) -> Result<(u32, Self::Balance, bool), Self::Error>;

    fn is_yield_farm_claimable(global_farm_id: u32, yield_farm_id: u32, amm_pool_id: Self::AmmPoolId) -> bool;

    fn get_global_farm_id(deposit_id: u128, yield_farm_id: u32) -> Option<u32>;
}
