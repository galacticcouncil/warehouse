use frame_support::dispatch::DispatchError;

pub trait Handler<AssetId, AmmPoolId, GlobalPoolId, PoolId, Balance, DepositId, AccountId> {
    fn get_balance_in_amm(asset: AssetId, amm_pool: AmmPoolId) -> Balance;

    fn on_accumulated_rpz_update(farm_id: GlobalPoolId, accumulated_rpz: Balance, total_shares_z: Balance);

    fn on_accumulated_rpvs_update(
        farm_id: GlobalPoolId,
        liq_pool_farm_id: PoolId,
        accumulated_rpvs: Balance,
        total_valued_shares: Balance,
    );

    fn lock_lp_tokens(
        amm_pool_id: AmmPoolId,
        who: AccountId,
        amount: Balance,
        deposit_id: DepositId,
    ) -> Result<(), DispatchError>;

    fn unlock_lp_tokens(
        amm_pool_id: AmmPoolId,
        who: AccountId,
        amount: Balance,
        deposit_id: DepositId,
    ) -> Result<(), DispatchError>;
}
