#[cfg(test)]
mod tests;

use proptest::prelude::*;
use proptest::prop_oneof;
use proptest::test_runner::{Config as PropConfig, FileFailurePersistence, TestError, TestRunner};

pub trait Interface {
    fn prepare(pool: PoolState);

    fn before_execute(&mut self);
    fn execute(d: u128);
    fn after_execute(&mut self);
}

pub enum PoolType<AssetId> {
    TwoAsset,
    TwoAssetWith(AssetId),
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
}

#[derive(Default, Debug, Clone)]
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
                let result = runner.run(&(pool_state_and_trade_amount(&$config)), |(amount, state)| {
                    <$runtime>::prepare(state);
                    $runtime.before_execute();
                    <$runtime>::execute(amount);
                    $runtime.after_execute();
                    Ok(())
                });

                println!("I did this too!");
            }
        }
    };
}

fn decimals() -> impl Strategy<Value = u32> {
    prop_oneof![Just(6), Just(8), Just(10), Just(12), Just(18),]
}

fn pool_state(config: &Config) -> impl Strategy<Value = PoolState> {
    (asset_reserve(config.max_reserve), asset_reserve(config.max_reserve)).prop_map(|(reserve_a, reserve_b)| {
        PoolState {
            asset_a: 0,
            asset_b: 1,
            reserve_a,
            reserve_b,
        }
    })

    /*
    match config.pool_type {
        PoolType::TwoAsset => todo!(),
        PoolType::TwoAssetWith(asset_id) => {
            asset_reserve(config.max_reserve).prop_map(|reserve|PoolState{
                asset_a: 0,
                asset_b: asset_id.clone(),
                reserve_a: reserve.clone(),
                reserve_b: reserve.clone(),
            })
        }
    }

         */
}

prop_compose! {
    fn asset_reserve(max_amount: u128)
                    (prec in decimals())
                    (reserve in 1.. max_amount * 10u128.pow(prec)) -> u128 {
        reserve
    }
}

prop_compose! {
    fn reserve_and_trade_amount(config: &Config)
                    (reserve in asset_reserve(config.max_reserve))
                    (amount in 1.. reserve / 3, reserve in Just(reserve)) -> (u128, u128) {
        (reserve, amount)
    }
}

prop_compose! {
    fn pool_state_and_trade_amount(config: &Config)
                    (pool in pool_state(config))
                    (amount in 1.. pool.reserve_a / 3, pool in Just(pool)) -> (u128, PoolState) {
        (amount, pool)
    }
}
