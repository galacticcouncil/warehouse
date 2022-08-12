// This file is part of pallet-price-oracle.

// Copyright (C) 2020-2021  Intergalactic, Limited (GIB).
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
    BlockNumber, Event as TestEvent, ExtBuilder, Origin, PriceOracle, System, Test, ACA, DOT, ETH, HDX, PRICE_ENTRY_1,
    PRICE_ENTRY_2,
};
use OraclePeriod::*;

use assert_matches::assert_matches;
use frame_support::assert_storage_noop;
use sp_arithmetic::traits::One;

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

fn last_events(n: usize) -> Vec<TestEvent> {
    frame_system::Pallet::<Test>::events()
        .into_iter()
        .rev()
        .take(n)
        .rev()
        .map(|e| e.event)
        .collect()
}

fn expect_events(e: Vec<TestEvent>) {
    assert_eq!(last_events(e.len()), e);
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

// #[test]
// fn update_data_should_work() {
//     new_test_ext().execute_with(|| {
//         System::set_block_number(3);
//         PriceOracle::on_initialize(3);

//         assert_ok!(PriceOracle::on_create_pool(HDX, ACA));
//         assert_ok!(PriceOracle::on_create_pool(HDX, DOT));

//         PriceOracle::on_finalize(3);
//         System::set_block_number(4);
//         PriceOracle::on_initialize(4);

//         PriceOracle::on_trade(HDX, DOT, PRICE_ENTRY_1);
//         PriceOracle::on_trade(HDX, DOT, PRICE_ENTRY_2);
//         PriceOracle::on_trade(HDX, ACA, PRICE_ENTRY_1);

//         PriceOracle::on_finalize(4);
//         System::set_block_number(5);
//         PriceOracle::on_initialize(5);

//         let data_ten_a = PriceOracle::price_data_ten()
//             .iter()
//             .find(|&x| x.0 == PriceOracle::derive_name(HDX, DOT))
//             .unwrap()
//             .1;
//         let data_ten_b = PriceOracle::price_data_ten()
//             .iter()
//             .find(|&x| x.0 == PriceOracle::derive_name(HDX, ACA))
//             .unwrap()
//             .1;

//         assert_eq!(
//             data_ten_a.get_last(),
//             PriceInfo {
//                 avg_price: 4.into(),
//                 volume: 4_000
//             }
//         );
//         assert_eq!(
//             data_ten_b.get_last(),
//             PriceInfo {
//                 avg_price: 2.into(),
//                 volume: 1_000
//             }
//         );
//     });
// }

// #[test]
// fn update_data_with_incorrect_input_should_not_work() {
//     new_test_ext().execute_with(|| {
//         System::set_block_number(3);
//         PriceOracle::on_initialize(3);

//         assert_ok!(PriceOracle::on_create_pool(HDX, DOT));

//         PriceOracle::on_finalize(3);
//         System::set_block_number(4);
//         PriceOracle::on_initialize(4);

//         PriceOracle::on_trade(
//             HDX,
//             DOT,
//             PriceEntry {
//                 price: Price::from(1),
//                 trade_amount: Zero::zero(),
//                 liquidity_amount: Zero::zero(),
//             },
//         );

//         PriceOracle::on_finalize(4);
//         System::set_block_number(5);
//         PriceOracle::on_initialize(5);

//         let data_ten = PriceOracle::price_data_ten()
//             .iter()
//             .find(|&x| x.0 == PriceOracle::derive_name(HDX, DOT))
//             .unwrap()
//             .1;
//         assert_eq!(
//             data_ten.get_last(),
//             PriceInfo {
//                 avg_price: Zero::zero(),
//                 volume: Zero::zero()
//             }
//         );
//     });
// }

// #[test]
// fn update_empty_data_should_work() {
//     new_test_ext().execute_with(|| {
//         let hdx_dot_pair_name = PriceOracle::derive_name(HDX, DOT);

//         assert_ok!(PriceOracle::on_create_pool(HDX, DOT));

//         for i in 0..1002 {
//             PriceOracle::on_initialize(i);
//             System::set_block_number(i);
//             PriceOracle::on_finalize(i);
//         }

//         let data_ten = PriceOracle::price_data_ten()
//             .iter()
//             .find(|&x| x.0 == hdx_dot_pair_name)
//             .unwrap()
//             .1;
//         assert_eq!(
//             data_ten.get_last(),
//             PriceInfo {
//                 avg_price: Zero::zero(),
//                 volume: Zero::zero()
//             }
//         );

//         let data_hundred = PriceOracle::price_data_hundred(hdx_dot_pair_name.clone());
//         assert_eq!(
//             data_hundred.get_last(),
//             PriceInfo {
//                 avg_price: Zero::zero(),
//                 volume: Zero::zero()
//             }
//         );

//         let data_thousand = PriceOracle::price_data_thousand(hdx_dot_pair_name);
//         assert_eq!(
//             data_thousand.get_last(),
//             PriceInfo {
//                 avg_price: Zero::zero(),
//                 volume: Zero::zero()
//             }
//         );
//     });
// }

// #[test]
// fn bucket_queue_should_work() {
//     let mut queue = BucketQueue::default();
//     for i in 0..BucketQueue::BUCKET_SIZE {
//         assert_eq!(queue[i as usize], PriceInfo::default());
//     }
//     assert_eq!(queue.get_last(), PriceInfo::default());

//     for i in 0..BucketQueue::BUCKET_SIZE {
//         let new_price = Price::from(i as u128);
//         queue.update_last(PriceInfo {
//             avg_price: new_price,
//             volume: 0,
//         });
//         assert_eq!(
//             queue.get_last(),
//             PriceInfo {
//                 avg_price: new_price,
//                 volume: 0
//             }
//         );
//         // for k in 0..BucketQueue::BUCKET_SIZE {
//         //     print!(" {}", queue.bucket[k as usize].avg_price.to_float());
//         // }
//         // println!();

//         for j in 0..BucketQueue::BUCKET_SIZE {
//             if i < j {
//                 assert_eq!(queue[j as usize], PriceInfo::default());
//             } else {
//                 assert_eq!(
//                     queue[j as usize],
//                     PriceInfo {
//                         avg_price: Price::from(j as u128),
//                         volume: 0
//                     }
//                 );
//             }
//         }
//     }

//     for i in BucketQueue::BUCKET_SIZE..BucketQueue::BUCKET_SIZE * 3 {
//         let new_price = Price::from(i as u128);
//         queue.update_last(PriceInfo {
//             avg_price: new_price,
//             volume: 0,
//         });
//         // for k in 0..BucketQueue::BUCKET_SIZE {
//         // 	print!(" {}", queue.bucket[k as usize].avg_price.to_float());
//         // }
//         // println!();

//         for j in 0..BucketQueue::BUCKET_SIZE {
//             if (i % BucketQueue::BUCKET_SIZE) < j {
//                 assert_eq!(
//                     queue[j as usize],
//                     PriceInfo {
//                         avg_price: Price::from((10 * (i / BucketQueue::BUCKET_SIZE).saturating_sub(1) + j) as u128),
//                         volume: 0
//                     }
//                 );
//             } else {
//                 assert_eq!(
//                     queue[j as usize],
//                     PriceInfo {
//                         avg_price: Price::from((j as u128) + 10u128 * (i / BucketQueue::BUCKET_SIZE) as u128),
//                         volume: 0
//                     }
//                 );
//             }
//         }
//     }
// }

// #[test]
// fn continuous_trades_should_work() {
//     ExtBuilder::default().build().execute_with(|| {
//         assert_ok!(PriceOracle::on_create_pool(HDX, DOT));

//         for i in 0..210 {
//             System::set_block_number(i);
//             PriceOracle::on_initialize(System::block_number());

//             PriceOracle::on_trade(
//                 HDX,
//                 DOT,
//                 PriceEntry {
//                     price: Price::from((i + 1) as u128),
//                     trade_amount: (i * 1_000).into(),
//                     liquidity_amount: 1u128,
//                 },
//             );

//             // let ten = PriceOracle::price_data_ten().iter().find(|&x| x.0 == ASSET_PAIR_A).unwrap().1;
//             // let hundred = PriceOracle::price_data_hundred(ASSET_PAIR_A);
//             // let thousand = PriceOracle::price_data_thousand(ASSET_PAIR_A);
//             //
//             // for i in 0..BUCKET_SIZE {
//             // 	print!(" {}", ten[i as usize].avg_price.to_float());
//             // }
//             // println!();
//             //
//             // for i in 0..BUCKET_SIZE {
//             // 	print!(" {}", hundred[i as usize].avg_price.to_float());
//             // }
//             // println!();
//             //
//             // for i in 0..BUCKET_SIZE {
//             // 	print!(" {}", thousand[i as usize].avg_price.to_float());
//             // }
//             // println!("\n");
//         }
//     })
// }

// #[test]
// fn stable_price_should_work() {
//     new_test_ext().execute_with(|| {
//         let hdx_dot_pair_name = PriceOracle::derive_name(HDX, DOT);

//         let num_of_iters = BucketQueue::BUCKET_SIZE.pow(3);
//         assert_ok!(PriceOracle::on_create_pool(HDX, DOT));

//         env_logger::init();

//         for i in num_of_iters - 2..2 * num_of_iters + 2 {
//             PriceOracle::on_initialize(i.into());
//             System::set_block_number(i.into());
//             PriceOracle::on_trade(HDX, DOT, PRICE_ENTRY_1);
//             PriceOracle::on_finalize(i.into());
//         }

//         let data_ten = PriceOracle::price_data_ten()
//             .iter()
//             .find(|&x| x.0 == hdx_dot_pair_name)
//             .unwrap()
//             .1;
//         let data_hundred = PriceOracle::price_data_hundred(hdx_dot_pair_name.clone());
//         let data_thousand = PriceOracle::price_data_thousand(hdx_dot_pair_name.clone());

//         assert_eq!(
//             data_ten.get_last(),
//             PriceInfo {
//                 avg_price: 2.into(),
//                 volume: 1_000
//             }
//         );
//         assert_eq!(
//             data_hundred.get_last(),
//             PriceInfo {
//                 avg_price: 2.into(),
//                 volume: 1_000
//             }
//         );
//         assert_eq!(
//             data_thousand.get_last(),
//             PriceInfo {
//                 avg_price: 2.into(),
//                 volume: 1_000
//             }
//         );
//         assert_eq!(PriceOracle::oracle(hdx_dot_pair_name.clone(), 1), Some(PRICE_ENTRY_1));
//         assert_eq!(PriceOracle::oracle(hdx_dot_pair_name.clone(), 10), Some(PRICE_ENTRY_1));
//         assert_eq!(PriceOracle::oracle(hdx_dot_pair_name.clone(), 50), Some(PRICE_ENTRY_1));
//         assert_eq!(PriceOracle::oracle(hdx_dot_pair_name.clone(), 5), None);

//         for i in num_of_iters..2 * num_of_iters {
//             PriceOracle::on_initialize(i.into());
//             System::set_block_number(i.into());
//             PriceOracle::on_finalize(i.into());
//         }

//         let data_ten = PriceOracle::price_data_ten()
//             .iter()
//             .find(|&x| x.0 == hdx_dot_pair_name)
//             .unwrap()
//             .1;
//         let data_hundred = PriceOracle::price_data_hundred(hdx_dot_pair_name.clone());
//         let data_thousand = PriceOracle::price_data_thousand(hdx_dot_pair_name);

//         assert_eq!(
//             data_ten.get_last(),
//             PriceInfo {
//                 avg_price: 2.into(),
//                 volume: 1_000
//             }
//         );
//         assert_eq!(
//             data_hundred.get_last(),
//             PriceInfo {
//                 avg_price: 2.into(),
//                 volume: 1_000
//             }
//         );
//         assert_eq!(
//             data_thousand.get_last(),
//             PriceInfo {
//                 avg_price: 2.into(),
//                 volume: 1_000
//             }
//         );
//     });
// }

#[test]
fn ema_works() {
    const PERIOD: u32 = 7;
    let alpha = Price::saturating_from_rational(2u32, PERIOD + 1); // EMA with period of 7
    debug_assert!(alpha <= Price::one());
    let inv_alpha = Price::one() - alpha;

    let start_oracle = (Price::saturating_from_integer(4u32), 4u32.into(), 4u32.into());
    let next_value = start_oracle;
    let next_oracle = ema(start_oracle, next_value, alpha, inv_alpha);
    assert_eq!(next_oracle, Some(start_oracle));

    let next_value = (Price::saturating_from_integer(8u32), 8u32.into(), 8u32.into());
    let next_oracle = ema(start_oracle, next_value, alpha, inv_alpha);
    assert_eq!(
        next_oracle,
        Some((Price::saturating_from_integer(5u32), 5u32.into(), 5u32.into()))
    );
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
            env_logger::init();
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
