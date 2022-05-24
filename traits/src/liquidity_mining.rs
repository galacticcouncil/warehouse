use frame_support::dispatch::DispatchError;

pub trait Handler<AssetId, AmmPoolId, GlobalPoolId, PoolId, Balance, DepositId, AccountId> {
    /// Returns balance of asset in amm pool
    fn get_balance_in_amm(asset: AssetId, amm_pool: AmmPoolId) -> Balance;

    /// This handler is called where accumulated rpz was updated.
    fn on_accumulated_rpz_update(farm_id: GlobalPoolId, accumulated_rpz: Balance, total_shares_z: Balance);

    /// This handler is called where accumulated rpvs was updated.
    fn on_accumulated_rpvs_update(
        farm_id: GlobalPoolId,
        liq_pool_farm_id: PoolId,
        accumulated_rpvs: Balance,
        total_valued_shares: Balance,
    );

    /// This function should lock LP shares.
    fn lock_lp_tokens(
        amm_pool_id: AmmPoolId,
        who: AccountId,
        amount: Balance,
        deposit_id: DepositId,
    ) -> Result<(), DispatchError>;

    /// This function should unlock LP shares.
    fn unlock_lp_tokens(
        amm_pool_id: AmmPoolId,
        who: AccountId,
        amount: Balance,
        deposit_id: DepositId,
    ) -> Result<(), DispatchError>;
}
