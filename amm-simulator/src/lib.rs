#[cfg(test)]
mod tests;

use proptest::prelude::*;
use proptest::prop_oneof;
use proptest::test_runner::{Config as PropConfig, FileFailurePersistence, TestError, TestRunner};

pub trait Interface {
    fn prepare(pool: Vec<PoolState>);

    fn before_execute(&mut self);
    fn execute(asset_in: u32, asset_out: u32, amount: u128);
    fn after_execute(&mut self);
    fn validate_sell(&self);
}

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
    pool_type: PoolType<u32>,
    trade_type: TradeType,
    max_reserve: u128,
    asset_ids: Vec<u32>,
    max_trade_ratio: u8,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct PoolState {
    asset_a: u32,
    asset_b: u32,
    reserve_a: u128,
    reserve_b: u128,
}

#[macro_export]
macro_rules! decl_amm {
    (
		pub struct $name:ident {
			Amm = $runtime:path,
			Config = $config:expr,
		}
	) => {
        pub struct $name;

        impl $name {
            pub fn execute_sell(&self) {
                let mut runner = TestRunner::new(PropConfig {
                    // Turn failure persistence off for demonstration
                    failure_persistence: Some(Box::new(FileFailurePersistence::Off)),
                    ..PropConfig::default()
                });
                let result = runner.run(
                    &(initial_state_and_trade_amount(&$config, $config.trade_type)),
                    |((asset_in, asset_out, amount), state)| {
                        <$runtime>::prepare(state);
                        $runtime.before_execute();
                        <$runtime>::execute(asset_in, asset_out, amount);
                        $runtime.after_execute();
                        $runtime.validate_sell();
                        Ok(())
                    },
                );
            }
        }
    };
}

fn decimals() -> impl Strategy<Value = u32> {
    prop_oneof![Just(6), Just(8), Just(10), Just(12), Just(18),]
}

fn pools(config: &Config) -> BoxedStrategy<Vec<PoolState>> {
    match config.pool_type {
        PoolType::TwoAsset => {
            let mut r = vec![];
            for assets in config.asset_ids.windows(2) {
                let a = assets[0];
                let b = assets[1];
                let p = (asset_reserve(config.max_reserve), asset_reserve(config.max_reserve)).prop_map(
                    move |(reserve_a, reserve_b)| PoolState {
                        asset_a: a,
                        asset_b: b,
                        reserve_a,
                        reserve_b,
                    },
                );
                r.push(p);
            }
            r.boxed()
        }
        PoolType::TwoAssetWith(asset_id, prec) => {
            let mut r = vec![];
            for asset in config.asset_ids.iter().filter(|id| **id != asset_id) {
                let a = *asset;
                let p = (
                    asset_reserve(config.max_reserve),
                    asset_reserve_with_prec(config.max_reserve, prec),
                )
                    .prop_map(move |(reserve_a, reserve_b)| PoolState {
                        asset_a: a,
                        asset_b: asset_id,
                        reserve_a,
                        reserve_b,
                    });
                r.push(p);
            }
            r.boxed()
        }
    }
}

fn select_trade_assets(
    state: Vec<PoolState>,
    trade_type: TradeType,
) -> impl Strategy<Value = (u32, u32, u128, Vec<PoolState>)> {
    let len = state.len();

    (0..len, 0..len).prop_map(move |(idx1, idx2)| match trade_type {
        TradeType::SinglePool => (
            state[idx1].asset_a,
            state[idx1].asset_b,
            state[idx1].reserve_a,
            state.clone(),
        ),
        TradeType::Any => (
            state[idx1].asset_a,
            state[idx2].asset_a,
            state[idx1].reserve_a,
            state.clone(),
        ),
    })
}

fn trade(max_amount: u128, max_ratio: u8) -> impl Strategy<Value = u128> {
    0..max_amount / max_ratio as u128
}

prop_compose! {
    fn initial_state_and_trade_assets(config: &Config, trade_type: TradeType)
                    (state in pools(config))
                    ((asset_in, asset_out, max_in, state) in select_trade_assets(state, trade_type)) -> ((u32,u32,u128), Vec<PoolState>) {
        ((asset_in,asset_out, max_in), state)
    }
}

prop_compose! {
    fn initial_state_and_trade_amount(config: &Config, trade_type: TradeType)
                    (((asset_in,asset_out, max_in), state) in initial_state_and_trade_assets(config, trade_type))
                    (amount in trade(max_in, 3 ), (asset_in, asset_out,state) in Just((asset_in, asset_out, state)))
                    -> ( (u32,u32,u128) ,  Vec<PoolState>) {
        ((asset_in, asset_out, amount), state)
    }
}
fn asset_reserve_with_prec(max_amount: u128, prec: u32) -> impl Strategy<Value = u128> {
    1..max_amount * 10u128.pow(prec)
}

prop_compose! {
    fn asset_reserve(max_amount: u128)
                    (prec in decimals())
                    (reserve in 1.. max_amount * 10u128.pow(prec)) -> u128 {
        reserve
    }
}
