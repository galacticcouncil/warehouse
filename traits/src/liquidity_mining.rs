use sp_arithmetic::{FixedU128, Permill};
use liquidity_mining::{GlobalFarmId, YieldFarmId, DepositId, FarmMultiplier};

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
    ) -> Result<(GlobalFarmId, Self::Balance), Self::Error>;

    fn destroy_global_farm(
        who: AccountId,
        farm_id: GlobalFarmId,
    ) -> Result<(AssetId, Self::Balance, AccountId), Self::Error>;

    fn create_yield_farm(
        who: AccountId,
        global_farm_id: GlobalFarmId,
        multiplier: FarmMultiplier,
        loyalty_curve: Option<LoyaltyCurve>,
        amm_pool_id: Self::AmmPoolId,
        asset_a: AssetId,
        asset_b: AssetId,
    ) -> Result<YieldFarmId, Self::Error>;

    fn update_yield_farm_multiplier(
        who: AccountId,
        global_farm_id: GlobalFarmId,
        amm_pool_id: Self::AmmPoolId,
        multiplier: FarmMultiplier,
    ) -> Result<YieldFarmId, Self::Error>;

    fn stop_yield_farm(
        who: AccountId,
        global_farm_id: GlobalFarmId,
        amm_pool_id: Self::AmmPoolId,
    ) -> Result<Self::YieldFarmId, Self::Error>;

    fn resume_yield_farm(
        who: AccountId,
        global_farm_id: GlobalFarmId,
        yield_farm_id: YieldFarmId,
        amm_pool_id: Self::AmmPoolId,
        multiplier: FarmMultiplier,
    ) -> Result<(), Self::Error>;

    fn destroy_yield_farm(
        who: AccountId,
        global_farm_id: GlobalFarmId,
        yield_farm_id: YieldFarmId,
        amm_pool_id: Self::AmmPoolId,
    ) -> Result<(), Self::Error>;

    fn deposit_lp_shares(
        who: AccountId,
        global_farm_id: GlobalFarmId,
        yield_farm_id: YieldFarmId,
        amm_pool_id: AmmPoolId,
        shares_amount: Balance,
    ) -> Result<DepositId, Self::Error>;

    fn redeposit_lp_shares(
        global_farm_id: GlobalFarmId,
        yield_farm_id: YieldFarmId,
        deposit_id: DepositId,
    ) -> Result<Self::Balance, Self::Error>;

    fn claim_rewards(
        who: AccountId,
        deposit_id: Self::DepositId,
        yield_farm_id: Self::YieldFarmId,
        check_double_claim: bool,
    ) -> Result<(GlobalFarmId, AssetId, Self::Balance, Self::Balance), Self::Error>;

    fn withdraw_lp_shares(
        who: AccountId,
        deposit_id: DepositId,
        yield_farm_id: YieldFarmId,
        unclaimable_rewards: Self::Balance,
    ) -> Result<(GlobalFarmId, Self::Balance), Self::Error>;

    fn is_yield_farm_claimable(
        global_farm_id: GlobalFarmId,
        yield_farm_id: YieldFarmId,
        amm_pool_id: Self::AmmPoolId,
    ) -> bool;

    fn get_global_farm_id(deposit_id: DepositId, yield_farm_id: YieldFarmId) -> Option<GlobalFarmId>;
}
