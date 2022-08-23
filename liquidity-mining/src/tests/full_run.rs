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

use crate::tests::mock::LiquidityMining2;

//This test test full run LM. Global farm is not full but it's running longer than expected. Users
//should be able to claim expected amount.
//This test case is without loyalty factor.
#[test]
fn non_full_farm_running_longer_than_expected() {
    new_test_ext().execute_with(|| {
        const GLOBAL_FARM: GlobalFarmId = 1;
        const YIELD_FARM_A: YieldFarmId = 2;
        const YIELD_FARM_B: YieldFarmId = 3;

        const ALICE_DEPOSIT: DepositId = 1;
        const BOB_DEPOSIT: DepositId = 2;
        const CHARLIE_DEPOSIT: DepositId = 3;

        //initialize farms
        set_block_number(100);
        assert_ok!(LiquidityMining2::create_global_farm(
            200_000 * ONE,
            20,
            10,
            BSX,
            BSX,
            GC,
            Perquintill::from_float(0.5),
            1_000,
            One::one()
        ));

        assert_ok!(LiquidityMining2::create_yield_farm(
            GC,
            GLOBAL_FARM,
            FixedU128::from(2_u128),
            None,
            BSX_TKN1_AMM,
            BSX,
            TKN1
        ));

        assert_ok!(LiquidityMining2::create_yield_farm(
            GC,
            GLOBAL_FARM,
            FixedU128::from(1_u128),
            None,
            BSX_TKN2_AMM,
            BSX,
            TKN2
        ));

        set_block_number(120);
        //alice
        assert_ok!(LiquidityMining2::deposit_lp_shares(
            GLOBAL_FARM,
            YIELD_FARM_A,
            BSX_TKN1_AMM,
            5_000 * ONE,
            |_, _| { Ok(1_u128) }
        ));

        set_block_number(140);
        //bob
        assert_ok!(LiquidityMining2::deposit_lp_shares(
            GLOBAL_FARM,
            YIELD_FARM_B,
            BSX_TKN2_AMM,
            2_500 * ONE,
            |_, _| { Ok(1_u128) }
        ));

        //charlie
        assert_ok!(LiquidityMining2::deposit_lp_shares(
            GLOBAL_FARM,
            YIELD_FARM_B,
            BSX_TKN2_AMM,
            2_500 * ONE,
            |_, _| { Ok(1_u128) }
        ));

        set_block_number(401);

        let alice_bsx_balance_0 = Tokens::free_balance(BSX, &ALICE);
        let bob_bsx_balance_0 = Tokens::free_balance(BSX, &BOB);
        let charlie_bsx_balance_0 = Tokens::free_balance(BSX, &CHARLIE);

        let (_, _, _, unclaimable) =
            LiquidityMining2::claim_rewards(ALICE, ALICE_DEPOSIT, YIELD_FARM_A, false).unwrap();
        assert_eq!(unclaimable, 0);
        assert_ok!(LiquidityMining2::withdraw_lp_shares(
            ALICE_DEPOSIT,
            YIELD_FARM_A,
            unclaimable
        ));

        let (_, _, _, unclaimable) = LiquidityMining2::claim_rewards(BOB, BOB_DEPOSIT, YIELD_FARM_B, false).unwrap();
        assert_eq!(unclaimable, 0);
        assert_ok!(LiquidityMining2::withdraw_lp_shares(
            BOB_DEPOSIT,
            YIELD_FARM_B,
            unclaimable
        ));

        let (_, _, _, unclaimable) =
            LiquidityMining2::claim_rewards(CHARLIE, CHARLIE_DEPOSIT, YIELD_FARM_B, false).unwrap();
        assert_eq!(unclaimable, 0);
        assert_ok!(LiquidityMining2::withdraw_lp_shares(CHARLIE, YIELD_FARM_B, unclaimable));

        let alice_claimed = Tokens::free_balance(BSX, &ALICE) - alice_bsx_balance_0;
        let bob_claimed = Tokens::free_balance(BSX, &BOB) - bob_bsx_balance_0;
        let charlie_claimed = Tokens::free_balance(BSX, &CHARLIE) - charlie_bsx_balance_0;

        let claimed_total = alice_claimed + bob_claimed + charlie_claimed;

        assert_eq!(claimed_total.abs_diff(200_000 * ONE), 1);
    });
}
