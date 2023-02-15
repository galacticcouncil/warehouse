use super::*;
pub struct SomeAmm;

fn sim_config() -> Config {
    Config {
        pool_type: PoolType::TwoAssetWith(1),
        trade_type: TradeType::Any,
        max_reserve: 1,
        asset_ids: vec![0],
    }
}

impl Interface for SomeAmm {
    fn prepare(pool: PoolState) {
        dbg!(pool);
    }

    fn before_execute(&mut self) {}

    fn execute(v: u128) {}

    fn after_execute(&mut self) {}
}

decl_amm!(
    pub struct AmmSim{
        Amm = SomeAmm,
        Config = sim_config(),
    }
);

#[test]
fn test_declare() {
    AmmSim.execute_sell();
}
