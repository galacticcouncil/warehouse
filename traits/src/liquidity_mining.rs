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
