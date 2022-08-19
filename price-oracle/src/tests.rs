// This file is part of pallet-price-oracle.

// Copyright (C) 2022  Intergalactic, Limited (GIB).
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::*;
pub use crate::mock::{
    BlockNumber, Event as TestEvent, ExtBuilder, Origin, PriceOracle, System, Test, ACA, DOT, HDX, PRICE_ENTRY_1,
    PRICE_ENTRY_2,
};
use OraclePeriod::*;

use assert_matches::assert_matches;
use frame_support::assert_storage_noop;
use sp_arithmetic::{traits::One, FixedPointNumber, FixedU128};

#[macro_export]
macro_rules! assert_eq_approx {
    ( $x:expr, $y:expr, $z:expr, $r:expr) => {{
        let diff = if $x >= $y { $x - $y } else { $y - $x };
        if diff > $z {
            panic!("\n{} not equal\n left: {:?}\nright: {:?}\n", $r, $x, $y);
        }
    }};
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    ExtBuilder::default().build()
}

/// Return the entry of an asset pair in the accumulator.
fn get_accumulator_entry(pair_id: &AssetPairId) -> Option<PriceEntry<BlockNumber>> {
    let a = Accumulator::<Test>::get();
    a.get(pair_id).map(|e| e.clone())
}

#[test]
fn genesis_config_works() {
    ExtBuilder::default()
        .with_price_data(vec![
            ((HDX, DOT), Price::from(1_000_000), 2_000_000, 2_000_000),
            ((HDX, ACA), Price::from(3_000_000), 4_000_000, 4_000_000),
        ])
        .build()
        .execute_with(|| {
            for period in OraclePeriod::all_periods() {
                assert_eq!(
                    PriceOracle::oracle(derive_name(HDX, DOT), period.into_num::<Test>()),
                    Some(PriceEntry {
                        price: Price::from(1_000_000),
                        volume: 2_000_000,
                        liquidity: 2_000_000,
                        timestamp: 0,
                    })
                );

                assert_eq!(
                    PriceOracle::oracle(derive_name(HDX, ACA), period.into_num::<Test>()),
                    Some(PriceEntry {
                        price: Price::from(3_000_000),
                        volume: 4_000_000,
                        liquidity: 4_000_000,
                        timestamp: 0,
                    })
                );
            }
        });
}

#[test]
fn on_trade_should_work() {
    new_test_ext().execute_with(|| {
        assert_eq!(get_accumulator_entry(&derive_name(HDX, DOT)), None);
        PriceOracle::on_trade(derive_name(HDX, DOT), PRICE_ENTRY_1);
        PriceOracle::on_trade(derive_name(HDX, DOT), PRICE_ENTRY_2);
        let price_entry = PRICE_ENTRY_2.accumulate_volume(&PRICE_ENTRY_1);
        assert_eq!(get_accumulator_entry(&derive_name(HDX, DOT)).unwrap(), price_entry);
    });
}

#[test]
fn on_trade_handler_should_work() {
    new_test_ext().execute_with(|| {
        System::set_block_number(PRICE_ENTRY_1.timestamp);
        assert_eq!(get_accumulator_entry(&derive_name(HDX, DOT)), None);
        PriceOracleHandler::<Test>::on_trade(HDX, DOT, 1_000, 500, 2_000);
        assert_eq!(get_accumulator_entry(&derive_name(HDX, DOT)), Some(PRICE_ENTRY_1));
    });
}

#[test]
fn on_liquidity_changed_handler_should_work() {
    new_test_ext().execute_with(|| {
        let timestamp = 5;
        System::set_block_number(timestamp);
        let no_volume_entry = PriceEntry {
            price: Price::saturating_from_integer(2),
            volume: 0,
            liquidity: 2_000,
            timestamp,
        };
        assert_eq!(get_accumulator_entry(&derive_name(HDX, DOT)), None);
        PriceOracleHandler::<Test>::on_liquidity_changed(HDX, DOT, 1_000, 500, 2_000);
        assert_eq!(get_accumulator_entry(&derive_name(HDX, DOT)), Some(no_volume_entry));
    });
}

#[test]
fn price_normalization_should_work() {
    new_test_ext().execute_with(|| {
        let hdx_dot_pair_name = derive_name(HDX, DOT);

        assert_eq!(get_accumulator_entry(&hdx_dot_pair_name), None);

        assert_storage_noop!(PriceOracleHandler::<Test>::on_trade(HDX, DOT, Balance::MAX, 1, 2_000));

        assert_storage_noop!(PriceOracleHandler::<Test>::on_trade(HDX, DOT, 1, Balance::MAX, 2_000));

        assert_storage_noop!(PriceOracleHandler::<Test>::on_trade(
            HDX,
            DOT,
            Balance::zero(),
            1_000,
            2_000
        ));

        assert_storage_noop!(PriceOracleHandler::<Test>::on_trade(
            HDX,
            DOT,
            1_000,
            Balance::zero(),
            2_000
        ));

        PriceOracleHandler::<Test>::on_trade(HDX, DOT, 340282366920938463463, 1, 2_000);

        assert_storage_noop!(PriceOracleHandler::<Test>::on_trade(
            HDX,
            DOT,
            1,
            340282366920938463463,
            2_000
        ));

        PriceOracleHandler::<Test>::on_trade(HDX, DOT, 2_000_000, 1_000, 2_000);

        PriceOracleHandler::<Test>::on_trade(HDX, DOT, 1_000, 2_000_000, 2_000);

        let price_entry = get_accumulator_entry(&hdx_dot_pair_name).unwrap();
        let first_entry = PriceEntry {
            price: Price::from(340282366920938463463),
            volume: 340282366920938463463,
            liquidity: 2_000,
            timestamp: 0,
        };

        let second_entry = PriceEntry {
            price: Price::from(2_000),
            volume: 2_000_000,
            liquidity: 2_000,
            timestamp: 0,
        };

        let third_entry = PriceEntry {
            price: Price::from_float(0.0005),
            volume: 1_000,
            liquidity: 2_000,
            timestamp: 0,
        };

        let result = third_entry.accumulate_volume(&second_entry.accumulate_volume(&first_entry));
        assert_eq!(price_entry, result);
    });
}

#[test]
fn update_data_should_work() {
    new_test_ext().execute_with(|| {
        System::set_block_number(5);
        PriceOracle::on_initialize(5);

        PriceOracle::on_trade(derive_name(HDX, DOT), PRICE_ENTRY_1);
        PriceOracle::on_trade(derive_name(HDX, DOT), PRICE_ENTRY_2);
        PriceOracle::on_trade(derive_name(HDX, ACA), PRICE_ENTRY_1);

        PriceOracle::on_finalize(5);
        System::set_block_number(6);
        PriceOracle::on_initialize(6);

        for period in OraclePeriod::all_periods() {
            assert_eq!(
                Oracles::<Test>::get(derive_name(HDX, DOT), period.into_num::<Test>()),
                Some(PRICE_ENTRY_2.accumulate_volume(&PRICE_ENTRY_1)),
            );
            assert_eq!(
                Oracles::<Test>::get(derive_name(HDX, ACA), period.into_num::<Test>()),
                Some(PRICE_ENTRY_1),
            );
        }
    });
}

#[test]
fn ema_stays_stable_if_the_value_does_not_change() {
    const PERIOD: u32 = 7;
    let alpha = Price::saturating_from_rational(2u32, PERIOD + 1); // EMA with period of 7
    debug_assert!(alpha <= Price::one());
    let complement = Price::one() - alpha;

    let start_price = Price::saturating_from_integer(4u32);
    let incoming_price = start_price;
    let next_price = price_ema(start_price, complement, incoming_price, alpha);
    assert_eq!(next_price, Some(start_price));
    let start_balance = 4u32.into();
    let incoming_balance = start_balance;
    let next_balance = balance_ema(start_balance, complement, incoming_balance, alpha);
    assert_eq!(next_balance, Some(start_balance));
}

#[test]
fn ema_works() {
    const PERIOD: u32 = 7;
    let alpha = Price::saturating_from_rational(2u32, PERIOD + 1); // EMA with period of 7
    debug_assert!(alpha <= Price::one());
    let complement = Price::one() - alpha;

    let start_price = Price::saturating_from_integer(4u32);
    let start_balance = 4u32.into();

    // updates by the correct amount if the value changes
    let (incoming_price, incoming_balance) = (Price::saturating_from_integer(8u32), 8u32.into());
    let next_price = price_ema(start_price, complement, incoming_price, alpha);
    assert_eq!(next_price, Some(Price::saturating_from_integer(5u32)));
    let next_balance = balance_ema(start_balance, complement, incoming_balance, alpha);
    assert_eq!(next_balance, Some(5u32.into()));
}

#[test]
fn ema_does_not_saturate_on_values_smaller_than_u64_max() {
    let alpha = Price::one();
    let complement = Price::zero();

    let start_balance = 50_000_000_000_000_000_000_000_u128;
    let incoming_balance = 50_000_000_000_000_000_000_000_u128;
    let next_balance = balance_ema(start_balance, complement, incoming_balance, alpha);
    assert_eq!(next_balance, Some(incoming_balance));
}

#[test]
fn calculate_new_ema_entry_works() {
    const PERIOD: u32 = 7;
    let (start_price, start_volume, start_liquidity) = (Price::saturating_from_integer(4u32), 4u32.into(), 4u32.into());
    let start_oracle = PriceEntry {
        price: start_price,
        volume: start_volume,
        liquidity: start_liquidity,
        timestamp: 5,
    };
    let next_value = PriceEntry {
        timestamp: 6,
        ..start_oracle
    };
    let next_oracle = next_value.calculate_new_ema_entry(PERIOD, &start_oracle);
    assert_eq!(next_oracle, Some(next_value));

    let (next_price, next_volume, next_liquidity) = (Price::saturating_from_integer(8u32), 8u32.into(), 8u32.into());
    let next_value = PriceEntry {
        price: next_price,
        volume: next_volume,
        liquidity: next_liquidity,
        timestamp: 6,
    };
    let next_oracle = next_value.calculate_new_ema_entry(PERIOD, &start_oracle);
    let expected_oracle = PriceEntry {
        price: Price::saturating_from_integer(5u32),
        volume: 5u32.into(),
        liquidity: 5u32.into(),
        timestamp: 6,
    };
    assert_eq!(next_oracle, Some(expected_oracle));
}

#[test]
fn calculate_new_ema_should_incorporate_longer_time_deltas() {
    const PERIOD: u32 = 7;
    let (start_price, start_volume, start_liquidity) =
        (Price::saturating_from_integer(4000u32), 4000u32.into(), 4000u32.into());
    let start_oracle = PriceEntry {
        price: start_price,
        volume: start_volume,
        liquidity: start_liquidity,
        timestamp: 5,
    };
    let (next_price, next_volume, next_liquidity) =
        (Price::saturating_from_integer(8000u32), 8000u32.into(), 8000u32.into());
    let next_value = PriceEntry {
        price: next_price,
        volume: next_volume,
        liquidity: next_liquidity,
        timestamp: 100,
    };
    let next_oracle = next_value.calculate_new_ema_entry(PERIOD, &start_oracle).unwrap();
    assert_eq_approx!(
        next_oracle.price,
        next_value.price,
        Price::from_float(0.0001),
        "Oracle price deviates too much."
    );

    let (next_price, next_volume, next_liquidity) =
        (Price::saturating_from_integer(8000u32), 8000u32.into(), 8000u32.into());
    let next_value = PriceEntry {
        price: next_price,
        volume: next_volume,
        liquidity: next_liquidity,
        timestamp: 8,
    };
    let next_oracle = next_value.calculate_new_ema_entry(PERIOD, &start_oracle);
    let expected_oracle = PriceEntry {
        price: Price::saturating_from_rational(63125, 10),
        volume: 6312u32.into(),
        liquidity: 6312u32.into(),
        timestamp: 8,
    };
    assert_eq!(next_oracle, Some(expected_oracle));
}

use EmaOracle;

#[test]
fn get_price_works() {
    ExtBuilder::default()
        .with_price_data(vec![((HDX, DOT), Price::from(1_000_000), 2_000_000, 2_000_000)])
        .build()
        .execute_with(|| {
            assert_matches!(PriceOracle::get_price(HDX, DOT, Immediate), (Some(p), _) if p == Price::from(1_000_000));
            assert_matches!(PriceOracle::get_price(HDX, DOT, TenMinutes), (Some(p), _) if p == Price::from(1_000_000));
            assert_matches!(PriceOracle::get_price(HDX, DOT, Day), (Some(p), _) if p == Price::from(1_000_000));
            assert_matches!(PriceOracle::get_price(HDX, DOT, Week), (Some(p), _) if p == Price::from(1_000_000));
        });
}

#[test]
fn get_price_returns_updated_price() {
    ExtBuilder::default()
        .with_price_data(vec![((HDX, DOT), Price::from(1_000_000), 2_000_000, 2_000_000)])
        .build()
        .execute_with(|| {
            let on_trade_entry = PriceEntry {
                price: Price::from(500_000),
                volume: 2_000_000,
                liquidity: 2_000_000,
                timestamp: 1_000,
            };
            System::set_block_number(1_000);
            PriceOracle::on_trade(derive_name(HDX, DOT), on_trade_entry);
            PriceOracle::on_finalize(1_000);

            let e = Price::from_float(0.01);
            assert_eq_approx!(
                PriceOracle::get_price(HDX, DOT, Immediate).0.unwrap(),
                Price::from(500_000),
                e,
                "Immediate Oracle should have most recent value."
            );
            assert_eq_approx!(
                PriceOracle::get_price(HDX, DOT, TenMinutes).0.unwrap(),
                Price::from(500_000),
                e,
                "TenMinutes Oracle should converge within 1000 blocks."
            );
            assert_eq_approx!(
                PriceOracle::get_price(HDX, DOT, Day).0.unwrap(),
                Price::from_float(878732.5635),
                e,
                "Day Oracle should converge somewhat."
            );
            assert_eq_approx!(
                PriceOracle::get_price(HDX, DOT, Week).0.unwrap(),
                Price::from_float(980547.25),
                e,
                "Week Oracle should converge a little."
            );
        });
}

#[test]
fn determine_price_convergence() {
    pub fn determine_iterations(period: Period, start: Price, incoming: Price, delta: Price) -> Option<Price> {
        let alpha = Price::saturating_from_rational(2u32, period.saturating_add(1));
        debug_assert!(alpha <= Price::one());
        let complement = Price::one() - alpha;

        let mut next_value = start;
        let mut round = 0;
        let delta = incoming * delta;
        while next_value.saturating_add(delta) < incoming {
            if round % 1000 == 0 {
                log::debug!("round {round}: start {start:?} current {next_value:?} incoming {incoming:?}");
            }
            next_value = price_ema(next_value, complement, incoming, alpha)?;
            round += 1;
        }
        log::debug!(
            "final value reached in {round} rounds: start {start:?} final_value {next_value:?} incoming {incoming:?}"
        );
        Some(next_value)
    }

    env_logger::init();

    let start = Price::saturating_from_integer(1_000u64);
    let target = Price::saturating_from_integer(100_000u64);
    let max_delta = Price::saturating_from_rational(1u32, 100u32);
    let final_value = determine_iterations(7200, start, target, max_delta);
    assert_eq!(final_value, Some(target));
}

#[test]
fn determine_balance_convergence() {
    pub fn determine_iterations(
        period: Period,
        start: Balance,
        incoming: Balance,
        max_deviation: FixedU128,
    ) -> Option<Balance> {
        let alpha = Price::saturating_from_rational(2u32, period.saturating_add(1));
        debug_assert!(alpha <= Price::one());
        let complement = Price::one() - alpha;

        let mut next_value = start;
        let mut round = 0;
        let delta = max_deviation.saturating_mul_int(incoming);
        log::debug!("delta {delta}");
        while next_value.saturating_add(delta) < incoming {
            if round % 1000 == 0 {
                log::debug!("round {round}: start {start:?} current {next_value:?} incoming {incoming:?}");
            }
            if round > 1_000_000 {
                let error1 = Price::one() - Price::saturating_from_rational(next_value, incoming);
                let error2 = Price::saturating_from_rational(next_value - start, incoming - start);
                log::debug!("approximating error: {error1:?} {error2:?}");
                log::debug!("delta {delta}");
                return None;
            }
            next_value = balance_ema(next_value, complement, incoming, alpha)?;
            round += 1;
        }
        log::debug!("delta {delta}");
        log::debug!(
            "final value reached in {round} rounds: start {start:?} final_value {next_value:?} incoming {incoming:?}"
        );
        let error1 = Price::one() - Price::saturating_from_rational(next_value, incoming);
        let error2 = Price::saturating_from_rational(next_value - start, incoming - start);
        log::debug!("approximating error: {error1:?} {error2:?}");
        Some(next_value)
    }

    env_logger::init();

    let start = 500_000_000_000_000_000u128;
    let target = 1_000_000_000_000_000_000u128;
    let max_deviation = FixedU128::saturating_from_rational(1u32, 100u32);
    let final_value = determine_iterations(7200, start, target, max_deviation);
    assert_eq!(final_value, Some(target));
}

#[test]
fn fewer_iterations() {
    pub fn iterative(period: Period, iterations: Period, start: Balance, incoming: Balance) -> Option<Balance> {
        let alpha = Price::saturating_from_rational(2u32, period.saturating_add(1));
        debug_assert!(alpha <= Price::one());
        let complement = Price::one() - alpha;

        let mut next_value = start;
        for _round in 0..iterations {
            next_value = balance_ema(next_value, complement, incoming, alpha)?;
        }
        Some(next_value)
    }

    pub fn exponential(period: Period, iterations: Period, start: Balance, incoming: Balance) -> Option<Balance> {
        let alpha = Price::saturating_from_rational(2u32, period.saturating_add(1));
        debug_assert!(alpha <= Price::one());
        let complement = Price::one() - alpha;

        let exp_complement = complement.saturating_pow(iterations as usize);
        let exp_alpha = Price::one() - exp_complement;

        balance_ema(start, exp_complement, incoming, exp_alpha)
    }

    env_logger::init();

    // (500, 600), (1, 1_000_000),
    for (start, target) in [(500_000_000_000_000_000u128, 1_000_000_000_000_000_000u128)] {
        // 50, 7200, 50400,
        for period in [7200 * 365] {
            // 10, 1_000, 10_000,
            for iterations in [1_000_000_000] {
                // let final_value = iterative(period, iterations, start, target).unwrap();
                let final_exp = exponential(period, iterations, start, target).unwrap();
                let target_diff = target.abs_diff(final_exp);
                let target_percentage_diff = FixedU128::saturating_from_rational(target_diff * 100, target);
                // let diff = final_value.abs_diff(final_exp);
                // let percentage_diff = FixedU128::saturating_from_rational(diff * 100, final_value);
                log::debug!("--------------------------------");
                log::debug!("period: {period}, iterations: {iterations}, start: {start}, target: {target}");
                // log::debug!("target: {target}, iterative: {final_value}, exponential: {final_exp}");
                log::debug!("target: {target}, exponential: {final_exp}");
                log::debug!("exponential diff to target: {target_diff}, percentage diff: {target_percentage_diff:?}");
                // log::debug!("exponential diff to iterative: {diff}, percentage diff: {percentage_diff:?}");
                // assert!(percentage_diff < FixedU128::saturating_from_rational(1u32, 5u32));
            }
        }
    }
    assert!(false);
}
