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

use assert_matches::assert_matches;
use frame_support::assert_storage_noop;
use sp_arithmetic::{traits::One, FixedPointNumber};

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
fn get_accumulator_entry(pair_id: &AssetPairId) -> Option<OracleEntry<BlockNumber>> {
    let a = Accumulator::<Test>::get();
    a.get(pair_id).map(|e| e.clone())
}

fn get_oracle_entry(asset_a: AssetId, asset_b: AssetId, period: &OraclePeriod) -> Option<OracleEntry<BlockNumber>> {
    Oracles::<Test>::get(derive_name(asset_a, asset_b), into_blocks::<Test>(period)).map(|(e, _)| e)
}

#[test]
fn genesis_config_works() {
    ExtBuilder::default()
        .with_price_data(vec![
            ((HDX, DOT), Price::from(1_000_000), 2_000_000),
            ((HDX, ACA), Price::from(3_000_000), 4_000_000),
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
        OnActivityHandler::<Test>::on_trade(HDX, DOT, 1_000, 500, 2_000);
        assert_eq!(get_accumulator_entry(&derive_name(HDX, DOT)), Some(PRICE_ENTRY_1));
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
        assert_eq!(get_accumulator_entry(&derive_name(HDX, DOT)), None);
        OnActivityHandler::<Test>::on_liquidity_changed(HDX, DOT, 1_000, 500, 2_000);
        assert_eq!(get_accumulator_entry(&derive_name(HDX, DOT)), Some(no_volume_entry));
    });
}

#[test]
fn price_normalization_should_exclude_extreme_values() {
    new_test_ext().execute_with(|| {
        assert_storage_noop!(OnActivityHandler::<Test>::on_trade(HDX, DOT, Balance::MAX, 1, 2_000));

        assert_storage_noop!(OnActivityHandler::<Test>::on_trade(HDX, DOT, 1, Balance::MAX, 2_000));

        assert_storage_noop!(OnActivityHandler::<Test>::on_trade(
            HDX,
            DOT,
            Balance::zero(),
            1_000,
            2_000
        ));

        assert_storage_noop!(OnActivityHandler::<Test>::on_trade(
            HDX,
            DOT,
            1_000,
            Balance::zero(),
            2_000
        ));
    });
}

#[test]
fn price_normalization_should_be_independent_of_asset_order() {
    assert_eq!(
        determine_normalized_price(HDX, DOT, 1_000, 500).unwrap(),
        determine_normalized_price(DOT, HDX, 500, 1_000).unwrap()
    );
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
        assert_eq!(get_accumulator_entry(&derive_name(HDX, DOT)), None);

        OnActivityHandler::<Test>::on_trade(HDX, DOT, 2_000_000, 1_000, 2_000);
        // we reverse the order of the arguments
        OnActivityHandler::<Test>::on_trade(DOT, HDX, 1_000, 2_000_000, 2_000);

        let price_entry = get_accumulator_entry(&derive_name(HDX, DOT)).unwrap();
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

        let result = second_entry.accumulate_volume(&first_entry);
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
                get_oracle_entry(HDX, DOT, period),
                Some(PRICE_ENTRY_2.accumulate_volume(&PRICE_ENTRY_1)),
            );
            assert_eq!(get_oracle_entry(HDX, ACA, period), Some(PRICE_ENTRY_1),);
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

    // price
    let start_price = 4.into();
    let incoming_price = 8.into();
    let next_price = price_ema(start_price, complement, incoming_price, alpha).unwrap();
    assert_eq!(next_price, 5.into());

    // balance
    let start_balance = 4u128;
    let incoming_balance = 8u128;
    let next_balance = balance_ema(start_balance, complement, incoming_balance, alpha).unwrap();
    assert_eq!(next_balance, 5u128);

    // volume
    let start_volume = Volume {
        a_in: 4u128,
        b_out: 1u128,
        a_out: 8u128,
        b_in: 0u128,
    };
    let incoming_volume = Volume {
        a_in: 8u128,
        b_out: 1u128,
        a_out: 4u128,
        b_in: 0u128,
    };
    let next_volume = volume_ema(&start_volume, complement, &incoming_volume, alpha).unwrap();
    assert_eq!(
        next_volume,
        Volume {
            a_in: 5u128,
            b_out: 1u128,
            a_out: 7u128,
            b_in: 0u128
        }
    );
}

#[test]
fn ema_does_not_saturate() {
    let alpha = Price::one();
    let complement = Price::zero();

    let start_balance = u128::MAX;
    let incoming_balance = u128::MAX;
    let next_balance = balance_ema(start_balance, complement, incoming_balance, alpha);
    assert_eq!(next_balance, Some(incoming_balance));
}

#[test]
fn calculate_new_ema_entry_works() {
    const PERIOD: u32 = 7;
    let (start_price, start_volume, start_liquidity) = (
        Price::saturating_from_integer(4u32),
        Volume::from_a_in_b_out(1, 4),
        4u32.into(),
    );
    let start_oracle = OracleEntry {
        price: start_price,
        volume: start_volume,
        liquidity: start_liquidity,
        timestamp: 5,
    };
    let next_value = OracleEntry {
        timestamp: 6,
        ..start_oracle.clone()
    };
    let next_oracle = next_value.calculate_new_ema_entry(PERIOD, &start_oracle);
    assert_eq!(next_oracle, Some(next_value));

    let next_value = OracleEntry {
        price: Price::saturating_from_integer(8u32),
        volume: Volume::from_a_in_b_out(1, 8),
        liquidity: 8u32.into(),
        timestamp: 6,
    };
    let next_oracle = next_value.calculate_new_ema_entry(PERIOD, &start_oracle);
    let expected_oracle = OracleEntry {
        price: Price::saturating_from_integer(5u32),
        volume: Volume::from_a_in_b_out(1, 5),
        liquidity: 5u32.into(),
        timestamp: 6,
    };
    assert_eq!(next_oracle, Some(expected_oracle));
}

#[test]
fn calculate_new_ema_should_incorporate_longer_time_deltas() {
    const PERIOD: u32 = 7;
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
    let next_oracle = next_value.calculate_new_ema_entry(PERIOD, &start_oracle).unwrap();
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
    let next_oracle = next_value.calculate_new_ema_entry(PERIOD, &start_oracle);
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
        .with_price_data(vec![((HDX, DOT), Price::from(1_000_000), 2_000_000)])
        .build()
        .execute_with(|| {
            System::set_block_number(2);
            assert_matches!(PriceOracle::get_price(HDX, DOT, LastBlock), (Ok(p), _) if p == Price::from(1_000_000));
            assert_matches!(
                PriceOracle::get_price(HDX, DOT, TenMinutes),
                (Err(OracleError::NotReady), _)
            );
            assert_matches!(PriceOracle::get_price(HDX, DOT, Day), (Err(OracleError::NotReady), _));
            assert_matches!(PriceOracle::get_price(HDX, DOT, Week), (Err(OracleError::NotReady), _));
        });
}

#[test]
fn get_price_returns_updated_price_or_not_ready() {
    ExtBuilder::default()
        .with_price_data(vec![((HDX, DOT), Price::from(1_000_000), 2_000_000)])
        .build()
        .execute_with(|| {
            let on_trade_entry = OracleEntry {
                price: Price::from(500_000),
                volume: Volume::default(),
                liquidity: 2_000_000,
                timestamp: 10_000,
            };
            System::set_block_number(10_000);
            PriceOracle::on_trade(derive_name(HDX, DOT), on_trade_entry);
            PriceOracle::on_finalize(10_000);

            let e = Price::from_float(0.01);
            assert_eq_approx!(
                PriceOracle::get_price(HDX, DOT, LastBlock).0.unwrap(),
                Price::from(500_000),
                e,
                "LastBlock Oracle should have most recent value."
            );
            assert_eq_approx!(
                PriceOracle::get_price(HDX, DOT, TenMinutes).0.unwrap(),
                Price::from(500_000),
                e,
                "TenMinutes Oracle should converge within 1000 blocks."
            );
            assert_eq_approx!(
                PriceOracle::get_price(HDX, DOT, Day).0.unwrap(),
                Price::from_float(531088.261455783831),
                e,
                "Day Oracle should converge somewhat."
            );
            assert_eq!(PriceOracle::get_price(HDX, DOT, Week).0, Err(OracleError::NotReady));
        });
}
