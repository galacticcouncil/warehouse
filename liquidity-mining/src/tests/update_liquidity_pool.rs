// This file is part of Basilisk-node.

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
use test_ext::*;

#[test]
fn update_liquidity_yield_farm_should_work() {
    //yield farm without deposits
    predefined_test_ext().execute_with(|| {
        let new_multiplier: FarmMultiplier = FixedU128::from(5_000_u128);
        let yield_farm = LiquidityMining::yield_farm(BSX_TKN1_AMM, GC_FARM).unwrap();
        let global_farm = LiquidityMining::global_farm(GC_FARM).unwrap();

        assert_eq!(
            LiquidityMining::updated_yield_farm(GC, GC_FARM, new_multiplier, BSX_TKN1_AMM).unwrap(),
            BSX_TKN1_YIELD_FARM_ID
        );

        assert_eq!(
            LiquidityMining::yield_farm(BSX_TKN1_AMM, GC_FARM).unwrap(),
            YieldFarmData {
                multiplier: new_multiplier,
                ..yield_farm
            }
        );

        assert_eq!(LiquidityMining::global_farm(GC_FARM).unwrap(), global_farm);
    });

    //yield farm with deposits
    predefined_test_ext_with_deposits().execute_with(|| {
        //same period as last yield farm update so no farms(global or yield) need to be updated
        let new_multiplier: FarmMultiplier = FixedU128::from(10_000_u128);
        let yield_farm = LiquidityMining::yield_farm(BSX_TKN1_AMM, GC_FARM).unwrap();
        let global_farm = LiquidityMining::global_farm(GC_FARM).unwrap();

        assert_ok!(LiquidityMining::updated_yield_farm(
            GC,
            GC_FARM,
            new_multiplier,
            BSX_TKN1_AMM
        ));

        assert_eq!(
            LiquidityMining::yield_farm(BSX_TKN1_AMM, GC_FARM).unwrap(),
            YieldFarmData {
                multiplier: new_multiplier,
                ..yield_farm
            }
        );

        assert_eq!(
            LiquidityMining::global_farm(GC_FARM).unwrap(),
            GlobalFarmData {
                total_shares_z: 455_876_290,
                ..global_farm
            }
        );

        //different period so farms update should happen
        set_block_number(5_000);
        let new_multiplier: FarmMultiplier = FixedU128::from(5_000_u128);
        let yield_farm = LiquidityMining::yield_farm(BSX_TKN1_AMM, GC_FARM).unwrap();
        let global_farm = LiquidityMining::global_farm(GC_FARM).unwrap();

        let global_farm_account = LiquidityMining::farm_account_id(GC_FARM).unwrap();
        let yield_farm_account = LiquidityMining::farm_account_id(BSX_TKN1_YIELD_FARM_ID).unwrap();

        let global_farm_bsx_balance = Tokens::free_balance(BSX, &global_farm_account);
        let yield_farm_bsx_balance = Tokens::free_balance(BSX, &yield_farm_account);

        assert_ok!(LiquidityMining::updated_yield_farm(
            GC,
            GC_FARM,
            new_multiplier,
            BSX_TKN1_AMM
        ));

        assert_eq!(
            LiquidityMining::yield_farm(BSX_TKN1_AMM, GC_FARM).unwrap(),
            YieldFarmData {
                updated_at: 50,
                accumulated_rpvs: 30_060,
                accumulated_rpz: 15,
                multiplier: new_multiplier,
                ..yield_farm
            }
        );

        assert_eq!(
            LiquidityMining::global_farm(GC_FARM).unwrap(),
            GlobalFarmData {
                updated_at: 50,
                accumulated_rpz: 15,
                total_shares_z: 228_176_290,
                accumulated_rewards: global_farm.accumulated_rewards + 133_800_000,
                paid_accumulated_rewards: global_farm.paid_accumulated_rewards + 1_366_200_000,
                ..global_farm
            }
        );

        assert_eq!(
            Tokens::free_balance(BSX, &global_farm_account),
            global_farm_bsx_balance - 1_366_200_000 //1_366_200_000 - yield farm from global farm
        );
        assert_eq!(
            Tokens::free_balance(BSX, &yield_farm_account),
            yield_farm_bsx_balance + 1_366_200_000 //1_366_200_000 - yield farm claim from global farm
        );
    });
}

#[test]
fn update_yield_farm_zero_multiplier_should_not_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        assert_noop!(
            LiquidityMining::updated_yield_farm(GC, GC_FARM, FixedU128::from(0_u128), BSX_TKN1_AMM,),
            Error::<Test>::InvalidMultiplier
        );
    });
}

#[test]
fn update_yield_farm_canceled_farm_should_not_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        assert_ok!(LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN1_AMM));

        assert_noop!(
            LiquidityMining::updated_yield_farm(GC, GC_FARM, FixedU128::from(10_001), BSX_TKN1_AMM,),
            Error::<Test>::LiquidityMiningCanceled
        );
    });
}

#[test]
fn update_yield_farm_not_owner_should_not_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        assert_ok!(LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN1_AMM));

        let not_owner = ALICE;
        assert_noop!(
            LiquidityMining::updated_yield_farm(not_owner, GC_FARM, FixedU128::from(10_001_u128), BSX_TKN1_AMM),
            Error::<Test>::LiquidityMiningCanceled
        );
    });
}
