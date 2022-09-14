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
fn claim_rewards_should_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        with_transaction(|| {
            const FAIL_ON_DOUBLECLAIM: bool = true;
            const REWARD_CURRENCY: AssetId = BSX;
            let global_farm_id = GC_FARM;
            let alice_bsx_balance = Tokens::free_balance(BSX, &ALICE);
            let bsx_tkn1_yield_farm_account = LiquidityMining::farm_account_id(GC_BSX_TKN1_YIELD_FARM_ID).unwrap();
            let bsx_tkn2_yield_farm_account = LiquidityMining::farm_account_id(GC_BSX_TKN2_YIELD_FARM_ID).unwrap();
            let bsx_tkn1_yield_farm_reward_balance = Tokens::free_balance(BSX, &bsx_tkn1_yield_farm_account);

            let expected_claimed_rewards = 23_306;
            let unclaimable_rewards = 20_444;

            //claim A1.1  (dep. A1 1-th time)
            pretty_assertions::assert_eq!(
                LiquidityMining::claim_rewards(
                    ALICE,
                    PREDEFINED_DEPOSIT_IDS[0],
                    GC_BSX_TKN1_YIELD_FARM_ID,
                    FAIL_ON_DOUBLECLAIM
                )
                .unwrap(),
                (
                    global_farm_id,
                    REWARD_CURRENCY,
                    expected_claimed_rewards,
                    unclaimable_rewards
                )
            );

            pretty_assertions::assert_eq!(
                LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[0]).unwrap(),
                DepositData {
                    shares: 50,
                    amm_pool_id: BSX_TKN1_AMM,
                    yield_farm_entries: vec![YieldFarmEntry {
                        global_farm_id,
                        yield_farm_id: GC_BSX_TKN1_YIELD_FARM_ID,
                        accumulated_rpvs: Zero::zero(),
                        accumulated_claimed_rewards: expected_claimed_rewards,
                        entered_at: 18,
                        updated_at: 25,
                        valued_shares: 2_500,
                        _phantom: PhantomData::default(),
                    }]
                    .try_into()
                    .unwrap(),
                },
            );

            //Check if claimed rewards are transferred.
            pretty_assertions::assert_eq!(
                Tokens::free_balance(BSX, &ALICE),
                alice_bsx_balance + expected_claimed_rewards
            );

            //Check balance on yield farm account.
            pretty_assertions::assert_eq!(
                Tokens::free_balance(BSX, &bsx_tkn1_yield_farm_account),
                bsx_tkn1_yield_farm_reward_balance - expected_claimed_rewards
            );

            // claim B3.1
            set_block_number(3_056);
            let bsx_tkn2_yield_farm_reward_balance = Tokens::free_balance(BSX, &bsx_tkn2_yield_farm_account);
            let alice_bsx_balance = Tokens::free_balance(BSX, &ALICE);

            let expected_claimed_rewards = 3_417;
            let unclaimable_rewards = 3_108;

            pretty_assertions::assert_eq!(
                LiquidityMining::claim_rewards(
                    ALICE,
                    PREDEFINED_DEPOSIT_IDS[4],
                    GC_BSX_TKN2_YIELD_FARM_ID,
                    FAIL_ON_DOUBLECLAIM
                )
                .unwrap(),
                (
                    global_farm_id,
                    REWARD_CURRENCY,
                    expected_claimed_rewards,
                    unclaimable_rewards
                )
            );

            pretty_assertions::assert_eq!(
                LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[4]).unwrap(),
                DepositData {
                    shares: 87,
                    amm_pool_id: BSX_TKN2_AMM,
                    yield_farm_entries: vec![YieldFarmEntry {
                        global_farm_id,
                        yield_farm_id: GC_BSX_TKN2_YIELD_FARM_ID,
                        valued_shares: 261,
                        accumulated_rpvs: FixedU128::from(35),
                        accumulated_claimed_rewards: expected_claimed_rewards,
                        entered_at: 25,
                        updated_at: 30,
                        _phantom: PhantomData::default(),
                    }]
                    .try_into()
                    .unwrap(),
                },
            );

            pretty_assertions::assert_eq!(
                LiquidityMining::global_farm(GC_FARM).unwrap(),
                GlobalFarmData {
                    updated_at: 30,
                    accumulated_rpz: FixedU128::from(6),
                    total_shares_z: 703_990,
                    accumulated_rewards: 569_250,
                    paid_accumulated_rewards: 2_474_275,
                    ..get_predefined_global_farm_ins1(2)
                }
            );

            pretty_assertions::assert_eq!(
                LiquidityMining::yield_farm((BSX_TKN2_AMM, global_farm_id, GC_BSX_TKN2_YIELD_FARM_ID)).unwrap(),
                YieldFarmData {
                    updated_at: 30,
                    accumulated_rpvs: FixedU128::from(60),
                    accumulated_rpz: FixedU128::from(6),
                    total_shares: 960,
                    total_valued_shares: 47_629,
                    entries_count: 4,
                    ..PREDEFINED_YIELD_FARMS_INS1.with(|v| v[1].clone())
                },
            );

            //Check if claimed rewards are transferred.
            pretty_assertions::assert_eq!(
                Tokens::free_balance(BSX, &ALICE),
                alice_bsx_balance + expected_claimed_rewards
            );

            let yield_farm_claim_from_global_farm = 1_190_725;
            pretty_assertions::assert_eq!(
                Tokens::free_balance(BSX, &bsx_tkn2_yield_farm_account),
                bsx_tkn2_yield_farm_reward_balance + yield_farm_claim_from_global_farm - expected_claimed_rewards
            );

            //Run for log time(longer than planned_yielding_periods) without interactions with farms.
            //planned_yielding_periods = 500; 100 blocks per period
            //claim A1.2
            set_block_number(125_879);
            let bst_tkn1_yield_farm_reward_balance = Tokens::free_balance(BSX, &bsx_tkn1_yield_farm_account);
            let alice_bsx_balance = Tokens::free_balance(BSX, &ALICE);

            let expected_claimed_rewards = 7_437_514;
            let unclaimable_rewards = 289_180;

            pretty_assertions::assert_eq!(
                LiquidityMining::claim_rewards(
                    ALICE,
                    PREDEFINED_DEPOSIT_IDS[0],
                    GC_BSX_TKN1_YIELD_FARM_ID,
                    FAIL_ON_DOUBLECLAIM
                )
                .unwrap(),
                (
                    global_farm_id,
                    REWARD_CURRENCY,
                    expected_claimed_rewards,
                    unclaimable_rewards
                )
            );

            pretty_assertions::assert_eq!(
                LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[0]).unwrap(),
                DepositData {
                    shares: 50,
                    amm_pool_id: BSX_TKN1_AMM,
                    yield_farm_entries: vec![YieldFarmEntry {
                        global_farm_id,
                        yield_farm_id: GC_BSX_TKN1_YIELD_FARM_ID,
                        valued_shares: 2_500,
                        accumulated_rpvs: Zero::zero(),
                        accumulated_claimed_rewards: 7_460_820,
                        entered_at: 18,
                        updated_at: 1_258,
                        _phantom: PhantomData::default(),
                    }]
                    .try_into()
                    .unwrap(),
                },
            );

            pretty_assertions::assert_eq!(
                LiquidityMining::global_farm(GC_FARM).unwrap(),
                GlobalFarmData {
                    updated_at: 1_258,
                    max_reward_per_period: 60_000_000,
                    accumulated_rpz: FixedU128::from(620),
                    total_shares_z: 703_990,
                    accumulated_rewards: 292_442_060,
                    paid_accumulated_rewards: 142_851_325,
                    ..get_predefined_global_farm_ins1(2)
                }
            );

            pretty_assertions::assert_eq!(
                LiquidityMining::yield_farm((BSX_TKN1_AMM, global_farm_id, GC_BSX_TKN1_YIELD_FARM_ID)).unwrap(),
                YieldFarmData {
                    updated_at: 1_258,
                    accumulated_rpvs: FixedU128::from(3_100),
                    accumulated_rpz: FixedU128::from(620),
                    total_shares: 616,
                    total_valued_shares: 45_540,
                    entries_count: 3,
                    ..PREDEFINED_YIELD_FARMS_INS1.with(|v| v[0].clone())
                },
            );

            pretty_assertions::assert_eq!(
                LiquidityMining::yield_farm((BSX_TKN2_AMM, global_farm_id, GC_BSX_TKN2_YIELD_FARM_ID)).unwrap(),
                YieldFarmData {
                    updated_at: 30,
                    accumulated_rpvs: FixedU128::from(60),
                    accumulated_rpz: FixedU128::from(6),
                    total_shares: 960,
                    total_valued_shares: 47_629,
                    entries_count: 4,
                    ..PREDEFINED_YIELD_FARMS_INS1.with(|v| v[1].clone())
                },
            );

            //Check if claimed rewards are transferred.
            pretty_assertions::assert_eq!(
                Tokens::free_balance(BSX, &ALICE),
                alice_bsx_balance + expected_claimed_rewards
            );

            let yield_farm_claim_from_global_farm = 140_377_050;
            pretty_assertions::assert_eq!(
                Tokens::free_balance(BSX, &bsx_tkn1_yield_farm_account),
                bst_tkn1_yield_farm_reward_balance + yield_farm_claim_from_global_farm - expected_claimed_rewards
            );

            TransactionOutcome::Commit(())
        });
    });

    //Charlie's farm incentivize KSM and reward currency is ACA.
    //This test check if correct currency is transferred if rewards and incentivized
    //assets are different, otherwise farm behavior is the same as in tests above.
    predefined_test_ext().execute_with(|| {
        with_transaction(|| {
            const FAIL_ON_DOUBLECLAIM: bool = true;
            set_block_number(1_800); //period 18

            let global_farm_id = CHARLIE_FARM;
            let expected_claimed_rewards = 23_306; //ACA
            let unclaimable_rewards = 20_444;
            let deposited_amount = 50;
            let deposit_id = 1;
            assert_ok!(LiquidityMining::deposit_lp_shares(
                CHARLIE_FARM,
                CHARLIE_ACA_KSM_YIELD_FARM_ID,
                ACA_KSM_AMM,
                deposited_amount,
                |_, _| { Ok(50_u128) }
            ));

            pretty_assertions::assert_eq!(
                LiquidityMining::deposit(deposit_id).unwrap(),
                DepositData {
                    shares: 50,
                    amm_pool_id: ACA_KSM_AMM,
                    yield_farm_entries: vec![YieldFarmEntry {
                        global_farm_id,
                        yield_farm_id: CHARLIE_ACA_KSM_YIELD_FARM_ID,
                        accumulated_rpvs: Zero::zero(),
                        accumulated_claimed_rewards: 0,
                        entered_at: 18,
                        updated_at: 18,
                        valued_shares: 2_500,
                        _phantom: PhantomData::default(),
                    }]
                    .try_into()
                    .unwrap(),
                },
            );

            set_block_number(2_596); //period 25

            pretty_assertions::assert_eq!(
                LiquidityMining::claim_rewards(ALICE, deposit_id, CHARLIE_ACA_KSM_YIELD_FARM_ID, FAIL_ON_DOUBLECLAIM)
                    .unwrap(),
                (CHARLIE_FARM, ACA, expected_claimed_rewards, unclaimable_rewards)
            );

            //Alice had 0 ACA before claim.
            pretty_assertions::assert_eq!(Tokens::free_balance(ACA, &ALICE), expected_claimed_rewards);

            TransactionOutcome::Commit(())
        });
    });
}

#[test]
fn claim_rewards_deposit_with_multiple_entries_should_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        with_transaction(|| {
            const FAIL_ON_DOUBLECLAIM: bool = true;
            //predefined_deposit[0] - GC_FARM, BSX_TKN1_AMM
            set_block_number(50_000);
            assert_ok!(LiquidityMining::redeposit_lp_shares(
                EVE_FARM,
                EVE_BSX_TKN1_YIELD_FARM_ID,
                PREDEFINED_DEPOSIT_IDS[0],
                |_, _| { Ok(80_u128) }
            ));

            set_block_number(800_000);
            assert_ok!(LiquidityMining::redeposit_lp_shares(
                DAVE_FARM,
                DAVE_BSX_TKN1_YIELD_FARM_ID,
                PREDEFINED_DEPOSIT_IDS[0],
                |_, _| { Ok(100_u128) }
            ));

            let deposit = LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[0]).unwrap();

            pretty_assertions::assert_eq!(
                deposit.yield_farm_entries,
                vec![
                    YieldFarmEntry {
                        global_farm_id: GC_FARM,
                        valued_shares: 2_500,
                        yield_farm_id: GC_BSX_TKN1_YIELD_FARM_ID,
                        accumulated_claimed_rewards: 0,
                        accumulated_rpvs: Zero::zero(),
                        entered_at: 18,
                        updated_at: 18,
                        _phantom: PhantomData::default(),
                    },
                    YieldFarmEntry {
                        global_farm_id: EVE_FARM,
                        valued_shares: 4_000,
                        yield_farm_id: EVE_BSX_TKN1_YIELD_FARM_ID,
                        accumulated_claimed_rewards: 0,
                        accumulated_rpvs: Zero::zero(),
                        entered_at: 50,
                        updated_at: 50,
                        _phantom: PhantomData::default(),
                    },
                    YieldFarmEntry {
                        global_farm_id: DAVE_FARM,
                        valued_shares: 5_000,
                        yield_farm_id: DAVE_BSX_TKN1_YIELD_FARM_ID,
                        accumulated_claimed_rewards: 0,
                        accumulated_rpvs: Zero::zero(),
                        entered_at: 800,
                        updated_at: 800,
                        _phantom: PhantomData::default(),
                    },
                ]
            );

            set_block_number(1_000_000);
            pretty_assertions::assert_eq!(
                LiquidityMining::claim_rewards(
                    ALICE,
                    PREDEFINED_DEPOSIT_IDS[0],
                    EVE_BSX_TKN1_YIELD_FARM_ID,
                    FAIL_ON_DOUBLECLAIM
                )
                .unwrap(),
                (EVE_FARM, KSM, 7_238_095, 361_905)
            );

            assert_noop!(
                LiquidityMining::claim_rewards(
                    ALICE,
                    PREDEFINED_DEPOSIT_IDS[0],
                    EVE_BSX_TKN1_YIELD_FARM_ID,
                    FAIL_ON_DOUBLECLAIM
                ),
                Error::<Test, Instance1>::DoubleClaimInPeriod
            );

            pretty_assertions::assert_eq!(
                LiquidityMining::claim_rewards(
                    ALICE,
                    PREDEFINED_DEPOSIT_IDS[0],
                    GC_BSX_TKN1_YIELD_FARM_ID,
                    FAIL_ON_DOUBLECLAIM
                )
                .unwrap(),
                (GC_FARM, BSX, 62_078_099, 309_401)
            );

            let deposit = LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[0]).unwrap();
            pretty_assertions::assert_eq!(
                deposit.yield_farm_entries,
                vec![
                    YieldFarmEntry {
                        global_farm_id: GC_FARM,
                        valued_shares: 2_500,
                        yield_farm_id: GC_BSX_TKN1_YIELD_FARM_ID,
                        accumulated_claimed_rewards: 62_078_099,
                        accumulated_rpvs: Zero::zero(),
                        entered_at: 18,
                        updated_at: 10_000,
                        _phantom: PhantomData::default(),
                    },
                    YieldFarmEntry {
                        global_farm_id: EVE_FARM,
                        valued_shares: 4_000,
                        yield_farm_id: EVE_BSX_TKN1_YIELD_FARM_ID,
                        accumulated_claimed_rewards: 7_238_095,
                        accumulated_rpvs: Zero::zero(),
                        entered_at: 50,
                        updated_at: 1_000,
                        _phantom: PhantomData::default(),
                    },
                    YieldFarmEntry {
                        global_farm_id: DAVE_FARM,
                        valued_shares: 5_000,
                        yield_farm_id: DAVE_BSX_TKN1_YIELD_FARM_ID,
                        accumulated_claimed_rewards: 0,
                        accumulated_rpvs: Zero::zero(),
                        entered_at: 800,
                        updated_at: 800,
                        _phantom: PhantomData::default(),
                    },
                ]
            );

            //Same period different block.
            set_block_number(1_000_050);
            assert_noop!(
                LiquidityMining::claim_rewards(
                    ALICE,
                    PREDEFINED_DEPOSIT_IDS[0],
                    EVE_BSX_TKN1_YIELD_FARM_ID,
                    FAIL_ON_DOUBLECLAIM
                ),
                Error::<Test, Instance1>::DoubleClaimInPeriod
            );

            assert_noop!(
                LiquidityMining::claim_rewards(
                    ALICE,
                    PREDEFINED_DEPOSIT_IDS[0],
                    GC_BSX_TKN1_YIELD_FARM_ID,
                    FAIL_ON_DOUBLECLAIM
                ),
                Error::<Test, Instance1>::DoubleClaimInPeriod
            );

            pretty_assertions::assert_eq!(
                LiquidityMining::claim_rewards(
                    ALICE,
                    PREDEFINED_DEPOSIT_IDS[0],
                    DAVE_BSX_TKN1_YIELD_FARM_ID,
                    FAIL_ON_DOUBLECLAIM
                )
                .unwrap(),
                (DAVE_FARM, ACA, 1_666_666, 333_334)
            );

            let deposit = LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[0]).unwrap();
            pretty_assertions::assert_eq!(
                deposit.yield_farm_entries,
                vec![
                    YieldFarmEntry {
                        global_farm_id: GC_FARM,
                        valued_shares: 2_500,
                        yield_farm_id: GC_BSX_TKN1_YIELD_FARM_ID,
                        accumulated_claimed_rewards: 62_078_099,
                        accumulated_rpvs: Zero::zero(),
                        entered_at: 18,
                        updated_at: 10_000,
                        _phantom: PhantomData::default(),
                    },
                    YieldFarmEntry {
                        global_farm_id: EVE_FARM,
                        valued_shares: 4_000,
                        yield_farm_id: EVE_BSX_TKN1_YIELD_FARM_ID,
                        accumulated_claimed_rewards: 7_238_095,
                        accumulated_rpvs: Zero::zero(),
                        entered_at: 50,
                        updated_at: 1_000,
                        _phantom: PhantomData::default(),
                    },
                    YieldFarmEntry {
                        global_farm_id: DAVE_FARM,
                        valued_shares: 5_000,
                        yield_farm_id: DAVE_BSX_TKN1_YIELD_FARM_ID,
                        accumulated_claimed_rewards: 1_666_666,
                        accumulated_rpvs: Zero::zero(),
                        entered_at: 800,
                        updated_at: 1_000,
                        _phantom: PhantomData::default(),
                    },
                ]
            );

            TransactionOutcome::Commit(())
        });
    });
}

#[test]
fn claim_rewards_doubleclaim_in_the_same_period_should_not_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        with_transaction(|| {
            const FAIL_ON_DOUBLECLAIM: bool = true;
            let global_farm_id = GC_FARM;
            let alice_bsx_balance = Tokens::free_balance(BSX, &ALICE);
            let bsx_tkn1_yield_farm_account = LiquidityMining::farm_account_id(GC_BSX_TKN1_YIELD_FARM_ID).unwrap();
            let bsx_tkn1_yield_farm_reward_balance = Tokens::free_balance(BSX, &bsx_tkn1_yield_farm_account);

            //1-th claim should works.
            assert_ok!(LiquidityMining::claim_rewards(
                ALICE,
                PREDEFINED_DEPOSIT_IDS[0],
                GC_BSX_TKN1_YIELD_FARM_ID,
                FAIL_ON_DOUBLECLAIM
            ));

            pretty_assertions::assert_eq!(
                LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[0]).unwrap(),
                DepositData {
                    shares: 50,
                    amm_pool_id: BSX_TKN1_AMM,
                    yield_farm_entries: vec![YieldFarmEntry {
                        global_farm_id,
                        yield_farm_id: GC_BSX_TKN1_YIELD_FARM_ID,
                        valued_shares: 2_500,
                        accumulated_rpvs: Zero::zero(),
                        accumulated_claimed_rewards: 23_306,
                        entered_at: 18,
                        updated_at: 25,
                        _phantom: PhantomData::default(),
                    }]
                    .try_into()
                    .unwrap(),
                },
            );

            pretty_assertions::assert_eq!(Tokens::free_balance(BSX, &ALICE), alice_bsx_balance + 23_306);
            pretty_assertions::assert_eq!(
                Tokens::free_balance(BSX, &bsx_tkn1_yield_farm_account),
                bsx_tkn1_yield_farm_reward_balance - 23_306
            );

            //Second claim should fail.
            assert_noop!(
                LiquidityMining::claim_rewards(
                    ALICE,
                    PREDEFINED_DEPOSIT_IDS[0],
                    GC_BSX_TKN1_YIELD_FARM_ID,
                    FAIL_ON_DOUBLECLAIM
                ),
                Error::<Test, Instance1>::DoubleClaimInPeriod
            );

            TransactionOutcome::Commit(())
        });
    });
}

#[test]
fn claim_rewards_from_canceled_yield_farm_should_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        with_transaction(|| {
            const FAIL_ON_DOUBLECLAIM: bool = true;
            let global_farm_id = GC_FARM;
            let alice_bsx_balance = Tokens::free_balance(BSX, &ALICE);
            let bsx_tkn1_yield_farm_account = LiquidityMining::farm_account_id(GC_BSX_TKN1_YIELD_FARM_ID).unwrap();
            let bsx_tkn1_yield_farm_reward_balance = Tokens::free_balance(BSX, &bsx_tkn1_yield_farm_account);

            //Stop yield farming before claiming.
            assert_ok!(LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN1_AMM));

            let expected_claimed_rewards = 23_306;
            let unclaimable_rewards = 20_444;

            //claim A1.1  (dep. A1 1-th time)
            pretty_assertions::assert_eq!(
                LiquidityMining::claim_rewards(
                    ALICE,
                    PREDEFINED_DEPOSIT_IDS[0],
                    GC_BSX_TKN1_YIELD_FARM_ID,
                    FAIL_ON_DOUBLECLAIM
                )
                .unwrap(),
                (global_farm_id, BSX, expected_claimed_rewards, unclaimable_rewards)
            );

            pretty_assertions::assert_eq!(
                LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[0]).unwrap(),
                DepositData {
                    shares: 50,
                    amm_pool_id: BSX_TKN1_AMM,
                    yield_farm_entries: vec![YieldFarmEntry {
                        global_farm_id,
                        yield_farm_id: GC_BSX_TKN1_YIELD_FARM_ID,
                        valued_shares: 2_500,
                        accumulated_rpvs: Zero::zero(),
                        accumulated_claimed_rewards: expected_claimed_rewards,
                        entered_at: 18,
                        updated_at: 25,
                        _phantom: PhantomData::default(),
                    }]
                    .try_into()
                    .unwrap(),
                },
            );

            //Check if claimed rewards are transferred.
            pretty_assertions::assert_eq!(
                Tokens::free_balance(BSX, &ALICE),
                alice_bsx_balance + expected_claimed_rewards
            );

            //Check balance on yield farm's account.
            pretty_assertions::assert_eq!(
                Tokens::free_balance(BSX, &bsx_tkn1_yield_farm_account),
                bsx_tkn1_yield_farm_reward_balance - expected_claimed_rewards
            );

            TransactionOutcome::Commit(())
        });
    });
}

#[test]
fn claim_rewards_from_removed_yield_farm_should_not_work() {
    const FAIL_ON_DOUBLECLAIM: bool = true;
    predefined_test_ext_with_deposits().execute_with(|| {
        with_transaction(|| {
            //Stop yield farming before removing.
            assert_ok!(LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN1_AMM));

            //Delete yield farm before claim test.
            assert_ok!(LiquidityMining::destroy_yield_farm(
                GC,
                GC_FARM,
                GC_BSX_TKN1_YIELD_FARM_ID,
                BSX_TKN1_AMM
            ));

            assert_noop!(
                LiquidityMining::claim_rewards(
                    ALICE,
                    PREDEFINED_DEPOSIT_IDS[0],
                    GC_BSX_TKN1_YIELD_FARM_ID,
                    FAIL_ON_DOUBLECLAIM
                ),
                Error::<Test, Instance1>::YieldFarmNotFound
            );

            TransactionOutcome::Commit(())
        });
    });
}

#[test]
fn claim_rewards_doubleclaim_should_work() {
    const FAIL_ON_DOUBLECLAIM: bool = true;

    predefined_test_ext_with_deposits().execute_with(|| {
        with_transaction(|| {
            let (_, _, claimable_rewards, unclaimable_rewards) = LiquidityMining::claim_rewards(
                ALICE,
                PREDEFINED_DEPOSIT_IDS[0],
                GC_BSX_TKN1_YIELD_FARM_ID,
                !FAIL_ON_DOUBLECLAIM,
            )
            .unwrap();

            pretty_assertions::assert_eq!(claimable_rewards, 23_306);
            pretty_assertions::assert_eq!(unclaimable_rewards, 20_444);

            //Second claim in the same period should return 0 for `claimable_rewards` and real value for
            //`unclaimable_rewards`
            let (_, _, claimable_rewards, unclaimable_rewards) = LiquidityMining::claim_rewards(
                ALICE,
                PREDEFINED_DEPOSIT_IDS[0],
                GC_BSX_TKN1_YIELD_FARM_ID,
                !FAIL_ON_DOUBLECLAIM,
            )
            .unwrap();

            pretty_assertions::assert_eq!(claimable_rewards, 0);
            pretty_assertions::assert_eq!(unclaimable_rewards, 20_444);

            //check if double claim fails
            assert_noop!(
                LiquidityMining::claim_rewards(
                    ALICE,
                    PREDEFINED_DEPOSIT_IDS[0],
                    GC_BSX_TKN1_YIELD_FARM_ID,
                    FAIL_ON_DOUBLECLAIM,
                ),
                Error::<Test, Instance1>::DoubleClaimInPeriod
            );

            TransactionOutcome::Commit(())
        });
    });
}
