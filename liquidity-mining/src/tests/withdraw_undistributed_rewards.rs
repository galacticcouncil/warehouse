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
fn withdraw_undistributed_rewards_should_work() {
    predefined_test_ext().execute_with(|| {
        //farm have to empty to be able to withdraw undistributed rewards
        assert_ok!(LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN1_AMM));

        assert_ok!(LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN2_AMM));

        assert_ok!(LiquidityMining::kill_yield_farm(GC, GC_FARM, BSX_TKN1_AMM));
        assert_ok!(LiquidityMining::kill_yield_farm(GC, GC_FARM, BSX_TKN2_AMM));

        let farm_owner_bsx_balance = Tokens::total_balance(BSX, &GC);

        let withdrawn_amount = 30_000_000_000;
        assert_eq!(
            LiquidityMining::withdraw_undistributed_rewards(GC, GC_FARM).unwrap(),
            (BSX, withdrawn_amount)
        );

        assert_eq!(
            Tokens::total_balance(BSX, &GC),
            farm_owner_bsx_balance + withdrawn_amount
        );
    });
}

#[test]
fn withdraw_undistributed_rewards_non_existing_farm_should_not_work() {
    const NON_EXISTING_FARM: FarmId = 879_798;

    predefined_test_ext().execute_with(|| {
        assert_noop!(
            LiquidityMining::withdraw_undistributed_rewards(GC, NON_EXISTING_FARM),
            Error::<Test>::GlobalFarmNotFound
        );
    });
}

#[test]
fn withdraw_undistributed_rewards_not_owner_should_not_work() {
    predefined_test_ext().execute_with(|| {
        //farm have to empty to be able to withdraw undistributed rewards
        assert_ok!(LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN1_AMM));
        assert_ok!(LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN2_AMM));

        assert_ok!(LiquidityMining::kill_yield_farm(GC, GC_FARM, BSX_TKN1_AMM));
        assert_ok!(LiquidityMining::kill_yield_farm(GC, GC_FARM, BSX_TKN2_AMM));

        const NOT_OWNER: u128 = ALICE;
        assert_noop!(
            LiquidityMining::withdraw_undistributed_rewards(NOT_OWNER, GC_FARM),
            Error::<Test>::Forbidden
        );
    });
}

#[test]
fn withdraw_undistributed_rewards_not_empty_farm_should_not_work() {
    predefined_test_ext().execute_with(|| {
        //only cancel yield farm, DON'T remove (global farm is not empty)
        assert_ok!(LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN1_AMM));
        assert_ok!(LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN2_AMM));

        assert_ok!(LiquidityMining::kill_yield_farm(GC, GC_FARM, BSX_TKN2_AMM));

        assert_noop!(
            LiquidityMining::withdraw_undistributed_rewards(GC, GC_FARM),
            Error::<Test>::GlobalFarmIsNotEmpty
        );
    });

    predefined_test_ext().execute_with(|| {
        //not all yield farms are canceled
        assert_ok!(LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN1_AMM));
        assert_ok!(LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN2_AMM));

        assert_noop!(
            LiquidityMining::withdraw_undistributed_rewards(GC, GC_FARM),
            Error::<Test>::GlobalFarmIsNotEmpty
        );
    });
}
