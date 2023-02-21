use super::*;
pub struct SomeAmm;

fn sim_config() -> Config {
	Config {
		pool_type: PoolType::TwoAssetWith(1, 12),
		trade_type: TradeType::Any,
		max_reserve: 1,
		asset_ids: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
		max_trade_ratio: 3,
	}
}

impl Interface for SomeAmm {
	fn execute(_state: Vec<PoolState>, _asset_in: u32, _asset_out: u32, _amount: u128) {
		//dbg!(asset_in, asset_out, amount);
	}
}

decl_amm_sim!(
	pub struct AmmSim{
		Amm = SomeAmm,
		Config = sim_config(),
	}
);

#[test]
fn test_declare() {
	AmmSim.execute();
}
