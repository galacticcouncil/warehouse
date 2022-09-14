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
fn stop_yield_farm_should_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        with_transaction(|| {
            let yield_farm_account = LiquidityMining::farm_account_id(GC_BSX_TKN1_YIELD_FARM_ID).unwrap();
            let global_farm_account = LiquidityMining::farm_account_id(GC_FARM).unwrap();
            let yield_farm_bsx_balance = Tokens::free_balance(BSX, &yield_farm_account);
            let global_farm_bsx_balance = Tokens::free_balance(BSX, &global_farm_account);
            let yield_farm = LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID)).unwrap();
            let global_farm = LiquidityMining::global_farm(GC_FARM).unwrap();

            assert!(yield_farm.state.is_active());

            pretty_assertions::assert_eq!(
                LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN1_AMM).unwrap(),
                yield_farm.id
            );

            let stake_in_global_farm = yield_farm
                .multiplier
                .checked_mul_int(yield_farm.total_valued_shares)
                .unwrap();

            pretty_assertions::assert_eq!(
                LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID)).unwrap(),
                YieldFarmData {
                    state: FarmState::Stopped,
                    multiplier: 0.into(),
                    ..yield_farm
                }
            );

            assert!(LiquidityMining::active_yield_farm(BSX_TKN1_AMM, GC_FARM).is_none());

            pretty_assertions::assert_eq!(
                LiquidityMining::global_farm(GC_FARM).unwrap(),
                GlobalFarmData {
                    total_shares_z: global_farm.total_shares_z.checked_sub(stake_in_global_farm).unwrap(),
                    ..global_farm
                }
            );

            pretty_assertions::assert_eq!(Tokens::free_balance(BSX, &yield_farm_account), yield_farm_bsx_balance);
            pretty_assertions::assert_eq!(Tokens::free_balance(BSX, &global_farm_account), global_farm_bsx_balance);

            TransactionOutcome::Commit(())
        });
    });

    //Cancel yield farming with farms update.
    predefined_test_ext_with_deposits().execute_with(|| {
        with_transaction(|| {
            let yield_farm_account = LiquidityMining::farm_account_id(GC_BSX_TKN1_YIELD_FARM_ID).unwrap();
            let global_farm_account = LiquidityMining::farm_account_id(GC_FARM).unwrap();
            let yield_farm_bsx_balance = Tokens::free_balance(BSX, &yield_farm_account);
            let yield_farm = LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID)).unwrap();
            let global_farm = LiquidityMining::global_farm(GC_FARM).unwrap();

            assert!(yield_farm.state.is_active());

            set_block_number(10_000);

            pretty_assertions::assert_eq!(
                LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN1_AMM).unwrap(),
                yield_farm.id
            );

            let stake_in_global_farm = yield_farm
                .multiplier
                .checked_mul_int(yield_farm.total_valued_shares)
                .unwrap();
            pretty_assertions::assert_eq!(
                LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID)).unwrap(),
                YieldFarmData {
                    updated_at: 100,
                    accumulated_rpvs: FixedU128::from_inner(205_000_000_000_000_000_000_u128),
                    accumulated_rpz: FixedU128::from_inner(41_000_000_000_000_000_000_u128),
                    state: FarmState::Stopped,
                    multiplier: 0.into(),
                    ..yield_farm
                }
            );

            pretty_assertions::assert_eq!(
                LiquidityMining::global_farm(GC_FARM).unwrap(),
                GlobalFarmData {
                    updated_at: 100,
                    accumulated_rpz: FixedU128::from_inner(41_000_000_000_000_000_000_u128),
                    total_shares_z: global_farm.total_shares_z.checked_sub(stake_in_global_farm).unwrap(),
                    accumulated_rewards: 17_860_875,
                    paid_accumulated_rewards: 9_822_300,
                    ..global_farm
                }
            );

            pretty_assertions::assert_eq!(
                Tokens::free_balance(BSX, &yield_farm_account),
                yield_farm_bsx_balance + 8_538_750 //8_538_750 - yield farm's last claim from global farm
            );

            pretty_assertions::assert_eq!(Tokens::free_balance(BSX, &global_farm_account), 29_972_316_825);

            TransactionOutcome::Commit(())
        });
    });
}

#[test]
fn stop_yield_farm_invalid_yield_farm_should_not_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        with_transaction(|| {
            assert_noop!(
                LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_DOT_AMM),
                Error::<Test, Instance1>::YieldFarmNotFound
            );

            TransactionOutcome::Commit(())
        });
    });
}

#[test]
fn stop_yield_farm_liquidity_mining_already_canceled() {
    predefined_test_ext_with_deposits().execute_with(|| {
        with_transaction(|| {
            //1-th stop should pass ok.
            pretty_assertions::assert_eq!(
                LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN1_AMM).unwrap(),
                GC_BSX_TKN1_YIELD_FARM_ID
            );

            assert_noop!(
                LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN1_AMM),
                Error::<Test, Instance1>::YieldFarmNotFound
            );

            TransactionOutcome::Commit(())
        });
    });
}

#[test]
fn stop_yield_farm_not_owner_should_not_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        with_transaction(|| {
            const NOT_FARMS_OWNER: u128 = ALICE;

            assert_noop!(
                LiquidityMining::stop_yield_farm(NOT_FARMS_OWNER, GC_FARM, BSX_TKN1_AMM),
                Error::<Test, Instance1>::Forbidden
            );

            TransactionOutcome::Commit(())
        });
    });
}
