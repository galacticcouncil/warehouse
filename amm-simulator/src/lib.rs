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
}

pub enum PoolType<AssetId> {
    TwoAsset,
    TwoAssetWith(AssetId, u32),
}

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
                    &(initial_state_and_trade_amount(&$config)),
                    |((asset_in, asset_out, amount), state)| {
                        <$runtime>::prepare(state);
                        $runtime.before_execute();
                        <$runtime>::execute(asset_in, asset_out, amount);
                        $runtime.after_execute();
                        Ok(())
                    },
                );

                println!("I did this too!");
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
                let p = (asset_reserve(config.max_reserve), asset_reserve_with_prec(config.max_reserve, prec)).prop_map(
                    move |(reserve_a, reserve_b)| PoolState {
                        asset_a: a,
                        asset_b: asset_id,
                        reserve_a,
                        reserve_b,
                    },
                );
                r.push(p);
            }
            r.boxed()
        }
    }
}

fn select_pool(pools: &[PoolState]) -> BoxedStrategy<PoolState> {
    prop_oneof![
        Just(pools[0]),
        Just(pools[1]),
        Just(pools[2]),
        Just(pools[3]),
        Just(pools[4]),
        Just(pools[5]),
        Just(pools[6]),
        Just(pools[7]),
        Just(pools[8]),
        Just(pools[9]),
    ]
    .boxed()
}

fn trade(max_amount: u128, max_ratio: u8) -> impl Strategy<Value = u128> {
    0..max_amount / max_ratio as u128
}

prop_compose! {
    fn trade_params(config: &Config)
                    (state in pools(config))
                    (pool in select_pool(&state), state in Just(state)) -> (PoolState, Vec<PoolState>) {
        (pool, state)
    }
}

prop_compose! {
    fn initial_state_and_trade_amount(config: &Config)
                    ((pool, state) in trade_params(config))
                    (amount in trade(pool.reserve_a , 3 ), (pool,state) in Just((pool, state)))
                    -> ( (u32,u32,u128) ,  Vec<PoolState>) {
        ((pool.asset_a, pool.asset_b, amount), state)
    }
}
fn asset_reserve_with_prec(max_amount: u128, prec: u32) -> impl Strategy<Value = u128> {
    1.. max_amount * 10u128.pow(prec)
}

prop_compose! {
    fn asset_reserve(max_amount: u128)
                    (prec in decimals())
                    (reserve in 1.. max_amount * 10u128.pow(prec)) -> u128 {
        reserve
    }
}
