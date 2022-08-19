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
fn destory_yield_farm_with_deposits_should_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        let global_farm_account = LiquidityMining::farm_account_id(GC_FARM).unwrap();
        let yield_farm_account = LiquidityMining::farm_account_id(GC_BSX_TKN1_YIELD_FARM_ID).unwrap();

        let yield_farm_bsx_balance = Tokens::free_balance(BSX, &yield_farm_account);
        let global_farm_bsx_balance = Tokens::free_balance(BSX, &global_farm_account);

        // Cancel yield farm before removing.
        assert_ok!(LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN1_AMM));

        let global_farm = LiquidityMining::global_farm(GC_FARM).unwrap();
        let yield_farm = LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID)).unwrap();

        assert_ok!(LiquidityMining::destroy_yield_farm(
            GC,
            GC_FARM,
            GC_BSX_TKN1_YIELD_FARM_ID,
            BSX_TKN1_AMM
        ));

        pretty_assertions::assert_eq!(
            LiquidityMining::global_farm(GC_FARM).unwrap(),
            GlobalFarmData {
                yield_farms_count: (
                    global_farm.yield_farms_count.0.checked_sub(1).unwrap(),
                    global_farm.yield_farms_count.1
                ),
                ..global_farm
            }
        );

        //Yield farm is removed from storage only if there are no more farm entries.
        pretty_assertions::assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                state: FarmState::Deleted,
                ..yield_farm
            }
        );

        pretty_assertions::assert_eq!(Tokens::free_balance(BSX, &yield_farm_account), 0);

        //Unpaid rewards from yield farm account should be transferred back to yield farm account.
        pretty_assertions::assert_eq!(
            Tokens::free_balance(BSX, &global_farm_account),
            global_farm_bsx_balance.checked_add(yield_farm_bsx_balance).unwrap()
        );
    });
}

#[test]
fn destory_yield_farm_without_deposits_should_work() {
    predefined_test_ext().execute_with(|| {
        let global_farm_account = LiquidityMining::farm_account_id(GC_FARM).unwrap();
        let yield_farm_acoount = LiquidityMining::farm_account_id(GC_BSX_TKN1_YIELD_FARM_ID).unwrap();

        let yield_farm_bsx_balance = Tokens::free_balance(BSX, &yield_farm_acoount);
        let global_farm_bsx_balance = Tokens::free_balance(BSX, &global_farm_account);

        //Stop yield farm before removing
        assert_ok!(LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN1_AMM));

        let global_farm = LiquidityMining::global_farm(GC_FARM).unwrap();

        assert_ok!(LiquidityMining::destroy_yield_farm(
            GC,
            GC_FARM,
            GC_BSX_TKN1_YIELD_FARM_ID,
            BSX_TKN1_AMM
        ));

        pretty_assertions::assert_eq!(
            LiquidityMining::global_farm(GC_FARM).unwrap(),
            GlobalFarmData {
                yield_farms_count: (
                    global_farm.yield_farms_count.0.checked_sub(1).unwrap(),
                    global_farm.yield_farms_count.1.checked_sub(1).unwrap(), //yield farm was flushed so this should change
                ),
                ..global_farm
            }
        );

        //Yield farm without deposits should be flushed.
        assert!(LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID)).is_none());

        pretty_assertions::assert_eq!(Tokens::free_balance(BSX, &yield_farm_acoount), 0);

        //Unpaid rewards from yield farm account should be transferred back to global farm's account.
        pretty_assertions::assert_eq!(
            Tokens::free_balance(BSX, &global_farm_account),
            global_farm_bsx_balance.checked_add(yield_farm_bsx_balance).unwrap()
        );
    });
}

#[test]
fn destory_yield_farm_non_stopped_yield_farming_should_not_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        assert_noop!(
            LiquidityMining::destroy_yield_farm(GC, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID, BSX_TKN1_AMM),
            Error::<Test, Instance1>::LiquidityMiningIsActive
        );
    });
}

#[test]
fn destory_yield_farm_not_owner_should_not_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        const NOT_OWNER: u128 = ALICE;

        assert_ok!(LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN1_AMM));

        assert_noop!(
            LiquidityMining::destroy_yield_farm(NOT_OWNER, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID, BSX_TKN1_AMM),
            Error::<Test, Instance1>::Forbidden
        );
    });
}

#[test]
fn destory_yield_farm_yield_farm_does_not_exists_should_not_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        assert_noop!(
            LiquidityMining::destroy_yield_farm(GC, GC_FARM, BSX_DOT_YIELD_FARM_ID, BSX_DOT_AMM),
            Error::<Test, Instance1>::YieldFarmNotFound
        );
    });
}
