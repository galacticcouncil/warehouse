// This file is part of pallet-ema-oracle.

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

mod invariants;

use super::*;
pub use crate::mock::{
    BlockNumber, EmaOracle, Event as TestEvent, ExtBuilder, Origin, System, Test, ACA, DOT, HDX, PRICE_ENTRY_1,
    PRICE_ENTRY_2,
};

use frame_support::assert_storage_noop;
use pretty_assertions::assert_eq;
use sp_arithmetic::FixedPointNumber;

/// Default oracle source for tests.
const SOURCE: Source = *b"dummysrc";

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
fn get_accumulator_entry(src: Source, (a, b): (AssetId, AssetId)) -> Option<OracleEntry<BlockNumber>> {
    let acc = Accumulator::<Test>::get();
    acc.get(&(src, ordered_pair(a, b))).cloned()
}

fn get_oracle_entry(a: AssetId, b: AssetId, period: &OraclePeriod) -> Option<OracleEntry<BlockNumber>> {
    Oracles::<Test>::get((SOURCE, ordered_pair(a, b), into_blocks::<Test>(period))).map(|(e, _)| e)
}

#[test]
fn genesis_config_works() {
    ExtBuilder::default()
        .with_price_data(vec![
            (SOURCE, (HDX, DOT), Price::from(1_000_000), 2_000_000),
            (SOURCE, (HDX, ACA), Price::from(3_000_000), 4_000_000),
        ])
        .build()
        .execute_with(|| {
            for period in OraclePeriod::all_periods() {
                assert_eq!(
                    get_oracle_entry(HDX, DOT, period),
                    Some(OracleEntry {
                        price: Price::from(1_000_000),
                        volume: Volume::default(),
                        liquidity: 2_000_000,
                        timestamp: 0,
                    })
                );

                assert_eq!(
                    get_oracle_entry(HDX, ACA, period),
                    Some(OracleEntry {
                        price: Price::from(3_000_000),
                        volume: Volume::default(),
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
        assert_eq!(get_accumulator_entry(SOURCE, (HDX, DOT)), None);
        EmaOracle::on_trade(SOURCE, ordered_pair(HDX, DOT), PRICE_ENTRY_1);
        EmaOracle::on_trade(SOURCE, ordered_pair(HDX, DOT), PRICE_ENTRY_2);
        let price_entry = PRICE_ENTRY_2.with_added_volume_from(&PRICE_ENTRY_1);
        assert_eq!(get_accumulator_entry(SOURCE, (HDX, DOT)).unwrap(), price_entry);
    });
}

#[test]
fn on_trade_handler_should_work() {
    new_test_ext().execute_with(|| {
        System::set_block_number(PRICE_ENTRY_1.timestamp);
        assert_eq!(get_accumulator_entry(SOURCE, (HDX, DOT)), None);
        OnActivityHandler::<Test>::on_trade(SOURCE, HDX, DOT, 1_000, 500, 2_000);
        assert_eq!(get_accumulator_entry(SOURCE, (HDX, DOT)), Some(PRICE_ENTRY_1));
    });
}

#[test]
fn on_liquidity_changed_handler_should_work() {
    new_test_ext().execute_with(|| {
        let timestamp = 5;
        System::set_block_number(timestamp);
        let no_volume_entry = OracleEntry {
            price: Price::saturating_from_integer(2),
            volume: Volume::default(),
            liquidity: 2_000,
            timestamp,
        };
        assert_eq!(get_accumulator_entry(SOURCE, (HDX, DOT)), None);
        OnActivityHandler::<Test>::on_liquidity_changed(SOURCE, HDX, DOT, 1_000, 500, 2_000);
        assert_eq!(get_accumulator_entry(SOURCE, (HDX, DOT)), Some(no_volume_entry));
    });
}

#[test]
fn price_normalization_should_exclude_extreme_values() {
    new_test_ext().execute_with(|| {
        assert_storage_noop!(OnActivityHandler::<Test>::on_trade(
            SOURCE,
            HDX,
            DOT,
            Balance::MAX,
            1,
            2_000
        ));

        assert_storage_noop!(OnActivityHandler::<Test>::on_trade(
            SOURCE,
            HDX,
            DOT,
            1,
            Balance::MAX,
            2_000
        ));

        assert_storage_noop!(OnActivityHandler::<Test>::on_trade(
            SOURCE,
            HDX,
            DOT,
            Balance::zero(),
            1_000,
            2_000
        ));

        assert_storage_noop!(OnActivityHandler::<Test>::on_trade(
            SOURCE,
            HDX,
            DOT,
            1_000,
            Balance::zero(),
            2_000
        ));
    });
}

#[test]
fn volume_normalization_should_factor_in_asset_order() {
    assert_ne!(
        determine_normalized_volume(HDX, DOT, 1_000, 500),
        determine_normalized_volume(DOT, HDX, 500, 1_000)
    );
}

#[test]
fn oracle_volume_should_factor_in_asset_order() {
    new_test_ext().execute_with(|| {
        assert_eq!(get_accumulator_entry(SOURCE, (HDX, DOT)), None);

        OnActivityHandler::<Test>::on_trade(SOURCE, HDX, DOT, 2_000_000, 1_000, 2_000);
        // we reverse the order of the arguments
        OnActivityHandler::<Test>::on_trade(SOURCE, DOT, HDX, 1_000, 2_000_000, 2_000);

        let price_entry = get_accumulator_entry(SOURCE, (HDX, DOT)).unwrap();
        let first_entry = OracleEntry {
            price: Price::saturating_from_rational(2_000_000, 1_000),
            volume: Volume::from_a_in_b_out(2_000_000, 1_000),
            liquidity: 2_000,
            timestamp: 0,
        };
        let second_entry = OracleEntry {
            price: Price::saturating_from_rational(2_000_000, 1_000),
            volume: Volume::from_a_out_b_in(2_000_000, 1_000),
            liquidity: 2_000,
            timestamp: 0,
        };

        let result = second_entry.with_added_volume_from(&first_entry);
        assert_eq!(price_entry, result);
    });
}

#[test]
fn update_data_should_work() {
    new_test_ext().execute_with(|| {
        System::set_block_number(5);
        EmaOracle::on_initialize(5);

        EmaOracle::on_trade(SOURCE, ordered_pair(HDX, DOT), PRICE_ENTRY_1);
        EmaOracle::on_trade(SOURCE, ordered_pair(HDX, DOT), PRICE_ENTRY_2);
        EmaOracle::on_trade(SOURCE, ordered_pair(HDX, ACA), PRICE_ENTRY_1);

        EmaOracle::on_finalize(5);
        System::set_block_number(6);
        EmaOracle::on_initialize(6);

        for period in OraclePeriod::all_periods() {
            assert_eq!(
                get_oracle_entry(HDX, DOT, period),
                Some(PRICE_ENTRY_2.with_added_volume_from(&PRICE_ENTRY_1)),
            );
            assert_eq!(get_oracle_entry(HDX, ACA, period), Some(PRICE_ENTRY_1),);
        }
    });
}

#[test]
fn update_data_should_use_old_last_block_oracle_to_update_to_parent() {
    new_test_ext().execute_with(|| {
        System::set_block_number(5);
        EmaOracle::on_initialize(5);
        EmaOracle::on_trade(SOURCE, ordered_pair(HDX, DOT), PRICE_ENTRY_1);
        EmaOracle::on_finalize(5);

        System::set_block_number(6);
        EmaOracle::on_initialize(6);
        let second_entry = OracleEntry {
            liquidity: 3_000,
            timestamp: 6,
            ..PRICE_ENTRY_1
        };
        EmaOracle::on_trade(SOURCE, ordered_pair(HDX, DOT), second_entry.clone());
        EmaOracle::on_finalize(6);

        System::set_block_number(50);
        EmaOracle::on_initialize(50);
        let third_entry = OracleEntry {
            liquidity: 10,
            timestamp: 50,
            ..PRICE_ENTRY_1
        };
        EmaOracle::on_trade(SOURCE, ordered_pair(HDX, DOT), third_entry.clone());
        EmaOracle::on_finalize(50);

        for period in OraclePeriod::all_periods() {
            let period_num = into_blocks::<Test>(period);
            let second_at_50 = OracleEntry {
                timestamp: 49,
                ..second_entry.clone()
            };
            let mut expected = PRICE_ENTRY_1.clone();
            expected
                .chained_update_via_ema_with(period_num, &second_entry)
                .unwrap()
                .chained_update_via_ema_with(period_num, &second_at_50)
                .unwrap()
                .update_via_ema_with(period_num, &third_entry)
                .unwrap();
            assert_eq!(
                get_oracle_entry(HDX, DOT, period).unwrap(),
                expected,
                "Oracle entry should be updated correctly for {:?}",
                period
            );
        }
    });
}

#[test]
fn combine_via_ema_with_only_updates_timestamp_on_stable_values() {
    let period: u32 = 7;
    let start_oracle = OracleEntry {
        price: 4.into(),
        volume: Volume::from_a_in_b_out(1u128, 4u128),
        liquidity: 4u128,
        timestamp: 5,
    };
    let next_value = OracleEntry {
        timestamp: 6,
        ..start_oracle.clone()
    };
    let next_oracle = start_oracle.combine_via_ema_with(period, &next_value);
    assert_eq!(next_oracle, Some(next_value));
}

#[test]
fn combine_via_ema_with_works() {
    let period: u32 = 7;
    let start_oracle = OracleEntry {
        price: 4.into(),
        volume: Volume::from_a_in_b_out(1u128, 4u128),
        liquidity: 4u128,
        timestamp: 5,
    };

    let next_value = OracleEntry {
        price: 8.into(),
        volume: Volume::from_a_in_b_out(1u128, 8u128),
        liquidity: 8u128,
        timestamp: 6,
    };
    let next_oracle = start_oracle.combine_via_ema_with(period, &next_value);
    let expected_oracle = OracleEntry {
        price: 5.into(),
        volume: Volume::from_a_in_b_out(1u128, 5u128),
        liquidity: 5u128,
        timestamp: 6,
    };
    assert_eq!(next_oracle, Some(expected_oracle));
}

#[test]
fn combine_via_ema_with_last_block_period_returns_new_value() {
    let period: u32 = 1;
    let start_oracle = OracleEntry {
        price: 4.into(),
        volume: Volume::from_a_in_b_out(1u128, 4u128),
        liquidity: 4u128,
        timestamp: 5,
    };

    let next_value = OracleEntry {
        price: 8.into(),
        volume: Volume::from_a_in_b_out(1u128, 8u128),
        liquidity: 8u128,
        timestamp: 6,
    };
    let next_oracle = start_oracle.combine_via_ema_with(period, &next_value);
    let expected_oracle = next_value;
    assert_eq!(next_oracle, Some(expected_oracle));
}

#[test]
fn calculate_new_ema_equals_update_via_ema_with() {
    let period: u32 = 7;
    let mut start_oracle = OracleEntry {
        price: 4.into(),
        volume: Volume::from_a_in_b_out(1u128, 4u128),
        liquidity: 4u128,
        timestamp: 5,
    };

    let next_value = OracleEntry {
        price: 8.into(),
        volume: Volume::from_a_in_b_out(1u128, 8u128),
        liquidity: 8u128,
        timestamp: 6,
    };
    let next_oracle = start_oracle.combine_via_ema_with(period, &next_value);
    let expected_oracle = OracleEntry {
        price: 5.into(),
        volume: Volume::from_a_in_b_out(1u128, 5u128),
        liquidity: 5u128,
        timestamp: 6,
    };
    assert_eq!(next_oracle, Some(expected_oracle.clone()));
    start_oracle.update_via_ema_with(period, &next_value).unwrap();
    assert_eq!(start_oracle, expected_oracle);
}

#[test]
fn calculate_new_ema_should_incorporate_longer_time_deltas() {
    let period: u32 = 7;
    let start_oracle = OracleEntry {
        price: Price::saturating_from_integer(4000u32),
        volume: Volume::from_a_in_b_out(1, 4_000),
        liquidity: 4_000u32.into(),
        timestamp: 5,
    };
    let next_value = OracleEntry {
        price: Price::saturating_from_integer(8000u32),
        volume: Volume::from_a_in_b_out(1, 8_000),
        liquidity: 8_000u32.into(),
        timestamp: 100,
    };
    let next_oracle = start_oracle.combine_via_ema_with(period, &next_value).unwrap();
    assert_eq_approx!(
        next_oracle.price,
        next_value.price,
        Price::from_float(0.0001),
        "Oracle price deviates too much."
    );

    let next_value = OracleEntry {
        price: Price::saturating_from_integer(8000u32),
        volume: Volume::from_a_in_b_out(1, 8_000),
        liquidity: 8_000u32.into(),
        timestamp: 8,
    };
    let next_oracle = start_oracle.combine_via_ema_with(period, &next_value);
    let expected_oracle = OracleEntry {
        price: Price::saturating_from_rational(63125, 10),
        volume: Volume::from_a_in_b_out(1, 6_312),
        liquidity: 6_312u32.into(),
        timestamp: 8,
    };
    assert_eq!(next_oracle, Some(expected_oracle));
}

#[test]
fn get_price_works() {
    ExtBuilder::default()
        .with_price_data(vec![(SOURCE, (HDX, DOT), Price::from(1_000_000), 2_000_000)])
        .build()
        .execute_with(|| {
            System::set_block_number(2);
            let expected = (Price::from(1_000_000), 1);
            assert_eq!(EmaOracle::get_price(HDX, DOT, LastBlock, SOURCE), Ok(expected));
            assert_eq!(EmaOracle::get_price(HDX, DOT, TenMinutes, SOURCE), Ok(expected));
            assert_eq!(EmaOracle::get_price(HDX, DOT, Day, SOURCE), Ok(expected));
            assert_eq!(EmaOracle::get_price(HDX, DOT, Week, SOURCE), Ok(expected));
        });
}

#[test]
fn trying_to_get_price_for_same_asset_should_error() {
    ExtBuilder::default()
        .with_price_data(vec![(SOURCE, (HDX, DOT), Price::from(1_000_000), 2_000_000)])
        .build()
        .execute_with(|| {
            System::set_block_number(2);
            assert_eq!(
                EmaOracle::get_price(HDX, HDX, LastBlock, SOURCE),
                Err(OracleError::SameAsset),
            );
        });
}

#[test]
fn get_entry_works() {
    ExtBuilder::default().build().execute_with(|| {
        System::set_block_number(1);
        OnActivityHandler::<Test>::on_trade(SOURCE, HDX, DOT, 1_000, 500, 2_000);
        EmaOracle::on_finalize(1);
        System::set_block_number(100);
        let expected = AggregatedEntry {
            price: Price::from((1_000, 500)),
            volume: Volume::from_a_in_b_out(1_000, 500),
            liquidity: 2_000,
            oracle_age: 98,
        };
        assert_eq!(EmaOracle::get_entry(HDX, DOT, LastBlock, SOURCE), Ok(expected.clone()));
        assert_eq!(EmaOracle::get_entry(HDX, DOT, TenMinutes, SOURCE), Ok(expected.clone()));
        assert_eq!(EmaOracle::get_entry(HDX, DOT, Day, SOURCE), Ok(expected.clone()));
        assert_eq!(EmaOracle::get_entry(HDX, DOT, Week, SOURCE), Ok(expected));
    });
}

#[test]
fn get_price_returns_updated_price() {
    ExtBuilder::default()
        .with_price_data(vec![(SOURCE, (HDX, DOT), Price::from(1_000_000), 2_000_000)])
        .build()
        .execute_with(|| {
            let on_trade_entry = OracleEntry {
                price: Price::from(500_000),
                volume: Volume::default(),
                liquidity: 2_000_000,
                timestamp: 10_000,
            };
            System::set_block_number(1);
            EmaOracle::on_trade(SOURCE, ordered_pair(HDX, DOT), on_trade_entry);
            EmaOracle::on_finalize(1);

            System::set_block_number(10_001);

            assert_eq!(
                EmaOracle::get_price(HDX, DOT, LastBlock, SOURCE).unwrap().1,
                10_000,
                "Oracle should be 10_000 blocks old."
            );
            assert_eq!(
                EmaOracle::get_price(HDX, DOT, Day, SOURCE).unwrap().1,
                10_000,
                "Oracle should be 10_000 blocks old."
            );

            let e = Price::from_float(0.01);
            assert_eq_approx!(
                EmaOracle::get_price(HDX, DOT, LastBlock, SOURCE).unwrap().0,
                Price::from(500_000),
                e,
                "LastBlock Oracle should have most recent value."
            );
            assert_eq_approx!(
                EmaOracle::get_price(HDX, DOT, TenMinutes, SOURCE).unwrap().0,
                Price::from(500_000),
                e,
                "TenMinutes Oracle should converge within 1000 blocks."
            );
            assert_eq_approx!(
                EmaOracle::get_price(HDX, DOT, Day, SOURCE).unwrap().0,
                Price::from_float(531_088.261_455_784),
                e,
                "Day Oracle should converge somewhat."
            );
            assert_eq_approx!(
                EmaOracle::get_price(HDX, DOT, Week, SOURCE).unwrap().0,
                Price::from_float(836_225.713_750_993),
                e,
                "Week Oracle should converge somewhat."
            );
        });
}

#[test]
fn ema_update_should_return_none_if_new_entry_is_older() {
    let mut entry = OracleEntry {
        timestamp: 10,
        ..PRICE_ENTRY_1
    };
    let original = entry.clone();
    // older than current
    let outdated_entry = OracleEntry {
        timestamp: 9,
        ..PRICE_ENTRY_2
    };
    assert_eq!(entry.combine_via_ema_with(10, &outdated_entry), None);
    assert_eq!(entry.combine_via_ema_with(1, &outdated_entry), None);
    // same timestamp as current
    let outdated_entry = OracleEntry {
        timestamp: 10,
        ..PRICE_ENTRY_2
    };
    assert_eq!(entry.combine_via_ema_with(10, &outdated_entry), None);
    assert_eq!(entry.combine_via_ema_with(1, &outdated_entry), None);

    assert_eq!(entry.update_via_ema_with(10, &outdated_entry), None);
    assert_eq!(entry, original);
}
