// This file is part of galacticcouncil/warehouse.

// Copyright (C) 2020-2022  Intergalactic, Limited (GIB).
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::*;
use test_ext::*;

#[test]
fn update_global_farm_price_adjustment_should_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        with_transaction(|| {
            let new_price_adjustment = FixedU128::from_float(0.856_f64);
            let global_farm_0 = LiquidityMining::global_farm(GC_FARM).unwrap();

            set_block_number(100_000);

            assert_ok!(LiquidityMining::update_global_farm_price_adjustment(
                GC,
                GC_FARM,
                new_price_adjustment
            ));

            pretty_assertions::assert_eq!(
                LiquidityMining::global_farm(GC_FARM).unwrap(),
                GlobalFarmData {
                    updated_at: 1_000,
                    accumulated_rpz: FixedU128::from_inner(491_000_000_000_000_000_000_u128),
                    price_adjustment: new_price_adjustment,
                    accumulated_rewards: 343_195_125_u128,
                    ..global_farm_0
                },
            );

            frame_system::Pallet::<Test>::assert_has_event(mock::Event::LiquidityMining(
                Event::GlobalFarmAccRPZUpdated {
                    global_farm_id: global_farm_0.id,
                    accumulated_rpz: FixedU128::from_inner(491_000_000_000_000_000_000_u128),
                    total_shares_z: global_farm_0.total_shares_z,
                },
            ));

            TransactionOutcome::Commit(())
        });
    });
}

#[test]
fn update_global_farm_price_adjustment_in_same_period_should_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        with_transaction(|| {
            let new_price_adjustment = FixedU128::from_float(0.856_f64);
            set_block_number(10_000);

            assert_ok!(LiquidityMining::update_global_farm_price_adjustment(
                GC,
                GC_FARM,
                new_price_adjustment
            ));

            frame_system::Pallet::<Test>::assert_has_event(mock::Event::LiquidityMining(
                Event::GlobalFarmAccRPZUpdated {
                    global_farm_id: GC_FARM,
                    accumulated_rpz: FixedU128::from_inner(41_000_000_000_000_000_000_u128),
                    total_shares_z: 703_990_u128,
                },
            ));

            let new_price_adjustment = FixedU128::from_float(0.6_f64);
            let global_farm_0 = LiquidityMining::global_farm(GC_FARM).unwrap();
            assert_ok!(LiquidityMining::update_global_farm_price_adjustment(
                GC,
                GC_FARM,
                new_price_adjustment
            ));

            pretty_assertions::assert_eq!(
                LiquidityMining::global_farm(GC_FARM).unwrap(),
                GlobalFarmData {
                    price_adjustment: new_price_adjustment,
                    ..global_farm_0
                },
            );

            TransactionOutcome::Commit(())
        });
    });
}

#[test]
fn update_global_farm_price_adjustment_not_owner_should_not_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        with_transaction(|| {
            let new_price_adjustment = FixedU128::from_float(0.6_f64);
            let not_owner = ALICE;

            set_block_number(10_000);

            assert_noop!(
                LiquidityMining::update_global_farm_price_adjustment(not_owner, GC_FARM, new_price_adjustment),
                Error::<Test, Instance1>::Forbidden
            );

            TransactionOutcome::Commit(())
        });
    });
}

//global farm not found
#[test]
fn update_global_farm_price_adjustment_not_existing_farm_should_not_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        with_transaction(|| {
            let new_price_adjustment = FixedU128::from_float(0.6_f64);
            let not_existing_farm = GlobalFarmId::MAX;

            set_block_number(10_000);

            assert_noop!(
                LiquidityMining::update_global_farm_price_adjustment(GC, not_existing_farm, new_price_adjustment),
                Error::<Test, Instance1>::GlobalFarmNotFound
            );

            TransactionOutcome::Commit(())
        });
    });
}
