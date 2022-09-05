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
fn update_yield_farm_should_work() {
    //Yield farm without deposits.
    predefined_test_ext().execute_with(|| {
        let new_multiplier: FarmMultiplier = FixedU128::from(5_000_u128);
        let yield_farm = LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID)).unwrap();
        let global_farm = LiquidityMining::global_farm(GC_FARM).unwrap();

        pretty_assertions::assert_eq!(
            LiquidityMining::update_yield_farm_multiplier(GC, GC_FARM, BSX_TKN1_AMM, new_multiplier).unwrap(),
            GC_BSX_TKN1_YIELD_FARM_ID
        );

        pretty_assertions::assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                multiplier: new_multiplier,
                ..yield_farm
            }
        );

        pretty_assertions::assert_eq!(LiquidityMining::global_farm(GC_FARM).unwrap(), global_farm);
    });

    //Yield farm with deposits.
    predefined_test_ext_with_deposits().execute_with(|| {
        //Same period as last yield farm update so no farms(global or yield) need to be updated.
        let new_multiplier: FarmMultiplier = FixedU128::from(10_000_u128);
        let yield_farm = LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID)).unwrap();
        let global_farm = LiquidityMining::global_farm(GC_FARM).unwrap();

        assert_ok!(LiquidityMining::update_yield_farm_multiplier(
            GC,
            GC_FARM,
            BSX_TKN1_AMM,
            new_multiplier,
        ));

        pretty_assertions::assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                multiplier: new_multiplier,
                ..yield_farm
            }
        );

        pretty_assertions::assert_eq!(
            LiquidityMining::global_farm(GC_FARM).unwrap(),
            GlobalFarmData {
                total_shares_z: 455_876_290,
                ..global_farm
            }
        );

        //Different period so farms update should happen.
        set_block_number(5_000);
        let new_multiplier: FarmMultiplier = FixedU128::from(5_000_u128);
        let yield_farm = LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID)).unwrap();
        let global_farm = LiquidityMining::global_farm(GC_FARM).unwrap();

        let global_farm_account = LiquidityMining::farm_account_id(GC_FARM).unwrap();
        let yield_farm_account = LiquidityMining::farm_account_id(GC_BSX_TKN1_YIELD_FARM_ID).unwrap();

        let yield_farm_bsx_balance = Tokens::free_balance(BSX, &yield_farm_account);

        assert_ok!(LiquidityMining::update_yield_farm_multiplier(
            GC,
            GC_FARM,
            BSX_TKN1_AMM,
            new_multiplier,
        ));

        pretty_assertions::assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                updated_at: 50,
                accumulated_rpvs: FixedU128::from_inner(32_921_163_394_817_742_643_829_u128),
                accumulated_rpz: FixedU128::from_inner(6_790_366_340_394_671_545_u128),
                multiplier: new_multiplier,
                ..yield_farm
            }
        );

        pretty_assertions::assert_eq!(
            LiquidityMining::global_farm(GC_FARM).unwrap(),
            GlobalFarmData {
                updated_at: 50,
                accumulated_rpz: FixedU128::from_inner(6_790_366_340_394_671_545_u128),
                total_shares_z: 228_176_290,
                accumulated_rewards: global_farm.accumulated_rewards + 1_567_169,
                paid_accumulated_rewards: global_farm.paid_accumulated_rewards + 1_498_432_831,
                ..global_farm
            }
        );

        pretty_assertions::assert_eq!(Tokens::free_balance(BSX, &global_farm_account), 28_498_716_450);
        pretty_assertions::assert_eq!(
            Tokens::free_balance(BSX, &yield_farm_account),
            yield_farm_bsx_balance + 1_498_432_831 //1_498_432_831 - yield farm claim from global farm
        );
    });
}

#[test]
fn update_yield_farm_zero_multiplier_should_not_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        assert_noop!(
            LiquidityMining::update_yield_farm_multiplier(GC, GC_FARM, BSX_TKN1_AMM, FixedU128::from(0_u128),),
            Error::<Test, Instance1>::InvalidMultiplier
        );
    });
}

#[test]
fn update_yield_farm_stopped_farm_should_not_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        assert_ok!(LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN1_AMM));

        //Yield farm must be in the active yield farm storage to update works.
        assert_noop!(
            LiquidityMining::update_yield_farm_multiplier(GC, GC_FARM, BSX_TKN1_AMM, FixedU128::from(10_001),),
            Error::<Test, Instance1>::YieldFarmNotFound
        );
    });
}

#[test]
fn update_yield_farm_deleted_farm_should_not_work() {
    //NOTE: yield farm is in the storage but it's deleted.
    predefined_test_ext_with_deposits().execute_with(|| {
        assert_ok!(LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN1_AMM));

        assert_ok!(LiquidityMining::destroy_yield_farm(
            GC,
            GC_FARM,
            GC_BSX_TKN1_YIELD_FARM_ID,
            BSX_TKN1_AMM
        ));

        assert!(LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID)).is_some());

        //Yield farm must be in the active yield farm storage to update works
        assert_noop!(
            LiquidityMining::update_yield_farm_multiplier(GC, GC_FARM, BSX_TKN1_AMM, FixedU128::from(10_001),),
            Error::<Test, Instance1>::YieldFarmNotFound
        );
    });
}

#[test]
fn update_yield_farm_not_owner_should_not_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        let not_owner = ALICE;
        assert_noop!(
            LiquidityMining::update_yield_farm_multiplier(
                not_owner,
                GC_FARM,
                BSX_TKN1_AMM,
                FixedU128::from(10_001_u128),
            ),
            Error::<Test, Instance1>::Forbidden
        );
    });
}