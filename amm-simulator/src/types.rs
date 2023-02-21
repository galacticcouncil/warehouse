use sp_arithmetic::FixedU128;

#[derive(Debug)]
pub enum PoolType<AssetId> {
	TwoAsset,
	TwoAssetWith(AssetId, u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeType {
	Any,
	SinglePool,
}

pub struct Config {
	pub pool_type: PoolType<u32>,
	pub trade_type: TradeType,
	pub max_reserve: u128,
	pub asset_ids: Vec<u32>,
	pub max_trade_ratio: u8,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct PoolState {
	pub asset_a: u32,
	pub asset_b: u32,
	pub reserve_a: u128,
	pub reserve_b: u128,
}

impl PoolState {
	pub fn price(&self) -> FixedU128 {
		FixedU128::from_rational(self.reserve_b, self.reserve_a)
	}
}
