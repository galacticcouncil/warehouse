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
fn remove_yield_farm_with_deposits_should_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        let global_farm_account = LiquidityMining::farm_account_id(GC_FARM).unwrap();
        let yield_farm_account = LiquidityMining::farm_account_id(BSX_TKN1_YIELD_FARM_ID).unwrap();

        let yield_farm_bsx_balance = Tokens::free_balance(BSX, &yield_farm_account);
        let global_farm_bsx_balance = Tokens::free_balance(BSX, &global_farm_account);

        // cancel yield farm before removing
        assert_ok!(LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN1_AMM));

        let global_farm = LiquidityMining::global_farm(GC_FARM).unwrap();

        assert_eq!(
            LiquidityMining::kill_yield_farm(GC, GC_FARM, BSX_TKN1_AMM).unwrap(),
            (BSX_TKN1_YIELD_FARM_ID)
        );

        assert_eq!(
            LiquidityMining::global_farm(GC_FARM).unwrap(),
            GlobalFarmData {
                yield_farms_count: global_farm.yield_farms_count.checked_sub(1).unwrap(),
                ..global_farm
            }
        );

        //yield farm should be removed from storage
        assert_eq!(LiquidityMining::yield_farm(BSX_TKN1_AMM, GC_FARM), None);

        //yield farm meta should stay in storage until all deposits are withdrawn
        assert_eq!(LiquidityMining::yield_farm_metadata(BSX_TKN1_YIELD_FARM_ID).unwrap(), 3);

        assert_eq!(Tokens::free_balance(BSX, &yield_farm_account), 0);

        //unpaid rewards from yield farm account should be transfered back to yield farm account
        assert_eq!(
            Tokens::free_balance(BSX, &global_farm_account),
            global_farm_bsx_balance.checked_add(yield_farm_bsx_balance).unwrap()
        );
    });
}

#[test]
fn remove_yield_farm_without_deposits_should_work() {
    predefined_test_ext().execute_with(|| {
        let global_farm_account = LiquidityMining::farm_account_id(GC_FARM).unwrap();
        let yield_farm_acoount = LiquidityMining::farm_account_id(BSX_TKN1_YIELD_FARM_ID).unwrap();

        let yield_farm_bsx_balance = Tokens::free_balance(BSX, &yield_farm_acoount);
        let global_farm_bsx_balance = Tokens::free_balance(BSX, &global_farm_account);

        //cancel yield farm before removing
        assert_ok!(LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN1_AMM));

        let global_farm = LiquidityMining::global_farm(GC_FARM).unwrap();

        assert_eq!(
            LiquidityMining::kill_yield_farm(GC, GC_FARM, BSX_TKN1_AMM).unwrap(),
            BSX_TKN1_YIELD_FARM_ID
        );

        assert_eq!(
            LiquidityMining::global_farm(GC_FARM).unwrap(),
            GlobalFarmData {
                yield_farms_count: global_farm.yield_farms_count.checked_sub(1).unwrap(),
                ..global_farm
            }
        );

        //yield farm should be removed from storage
        assert_eq!(LiquidityMining::yield_farm(BSX_TKN1_AMM, GC_FARM), None);

        //yield farm metadata should be removed from storage if no deposits are left
        assert_eq!(LiquidityMining::yield_farm_metadata(BSX_TKN1_YIELD_FARM_ID), None);

        assert_eq!(Tokens::free_balance(BSX, &yield_farm_acoount), 0);

        //unpaid rewards from yield farm account should be transfered back to global farm account
        assert_eq!(
            Tokens::free_balance(BSX, &global_farm_account),
            global_farm_bsx_balance.checked_add(yield_farm_bsx_balance).unwrap()
        );
    });
}

#[test]
fn remove_yield_farm_non_canceled_yield_farming_should_not_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        assert_noop!(
            LiquidityMining::kill_yield_farm(GC, GC_FARM, BSX_TKN1_AMM),
            Error::<Test>::LiquidityMiningIsNotCanceled
        );
    });
}

#[test]
fn remove_yield_farm_not_owner_should_not_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        const NOT_OWNER: u128 = ALICE;

        assert_ok!(LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN1_AMM));

        assert_noop!(
            LiquidityMining::kill_yield_farm(NOT_OWNER, GC_FARM, BSX_TKN1_AMM),
            Error::<Test>::Forbidden
        );
    });
}

#[test]
fn remove_yield_farm_yield_farm_does_not_exists_should_not_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        assert_noop!(
            LiquidityMining::kill_yield_farm(GC, GC_FARM, BSX_DOT_AMM),
            Error::<Test>::YieldFarmNotFound
        );
    });
}
