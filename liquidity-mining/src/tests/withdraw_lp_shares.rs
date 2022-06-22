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
fn withdraw_shares_should_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        const REWARD_CURRENCY: u32 = BSX;
        const GLOBAL_FARM_ID: GlobalFarmId = GC_FARM;

        let bsx_tn1_yield_farm_account = LiquidityMining::farm_account_id(GC_BSX_TKN1_YIELD_FARM_ID).unwrap();
        let bsx_tkn2_yield_farm_account = LiquidityMining::farm_account_id(GC_BSX_TKN2_YIELD_FARM_ID).unwrap();
        let global_farm_account = LiquidityMining::farm_account_id(GC_FARM).unwrap();

        // This balance is used to transfer unclaimable_rewards from yield farm to global farm.
        // Claiming is not part of withdraw_shares() so some balance need to be set.
        Tokens::set_balance(Origin::root(), bsx_tn1_yield_farm_account, BSX, 100_000_000_000, 0).unwrap();
        Tokens::set_balance(Origin::root(), bsx_tkn2_yield_farm_account, BSX, 100_000_000_000, 0).unwrap();

        // withdraw 1A
        let global_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &global_farm_account);
        let bsx_tkn2_yield_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_yield_farm_account);

        let unclaimable_rewards = 100_000;
        let withdrawn_amount = 50;
        let expected_deposit_destroyed = true;
        assert_eq!(
            LiquidityMining::withdraw_lp_shares(
                PREDEFINED_DEPOSIT_IDS[0],
                GC_BSX_TKN1_YIELD_FARM_ID,
                unclaimable_rewards,
            )
            .unwrap(),
            (GLOBAL_FARM_ID, withdrawn_amount, expected_deposit_destroyed,)
        );

        assert_eq!(
            LiquidityMining::global_farm(GC_FARM).unwrap(),
            GlobalFarmData {
                updated_at: 25,
                reward_currency: BSX,
                accumulated_rpz: 12,
                total_shares_z: 691_490,
                accumulated_rewards: 231_650,
                paid_accumulated_rewards: 1_164_400,
                ..get_predefined_global_farm_ins1(2)
            }
        );

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                updated_at: 25,
                accumulated_rpvs: 60,
                accumulated_rpz: 12,
                total_shares: 566,
                total_valued_shares: 43_040,
                entries_count: 2,
                ..get_predefined_yield_farm_ins1(0)
            },
        );

        //Yield farm balance checks.
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tn1_yield_farm_account),
            bsx_tkn2_yield_farm_bsx_balance - unclaimable_rewards
        );
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_yield_farm_account),
            bsx_tkn2_yield_farm_bsx_balance
        );

        //Global farm balance checks.
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &global_farm_account),
            global_farm_bsx_balance + unclaimable_rewards
        );

        assert_eq!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[0]), None);

        set_block_number(12_800);

        // withdraw 3B
        let global_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &global_farm_account);
        let bsx_tkn1_yield_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tn1_yield_farm_account);
        let bsx_tkn2_yield_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_yield_farm_account);

        let unclaimable_rewards = 32_786;
        let withdrawn_amount = 87;
        let expected_deposit_destroyed = true;
        assert_eq!(
            LiquidityMining::withdraw_lp_shares(
                PREDEFINED_DEPOSIT_IDS[4],
                GC_BSX_TKN2_YIELD_FARM_ID,
                unclaimable_rewards
            )
            .unwrap(),
            (GLOBAL_FARM_ID, withdrawn_amount, expected_deposit_destroyed)
        );

        assert_eq!(
            LiquidityMining::global_farm(GC_FARM).unwrap(),
            GlobalFarmData {
                updated_at: 25,
                accumulated_rpz: 12,
                accumulated_rewards: 231_650,
                paid_accumulated_rewards: 1_164_400,
                total_shares_z: 688_880,
                ..get_predefined_global_farm_ins1(2)
            }
        );

        // This farm should not change.
        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                updated_at: 25,
                accumulated_rpvs: 60,
                accumulated_rpz: 12,
                total_shares: 566,
                total_valued_shares: 43_040,
                entries_count: 2,
                ..get_predefined_yield_farm_ins1(0)
            },
        );

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN2_AMM, GC_FARM, GC_BSX_TKN2_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                updated_at: 25,
                accumulated_rpvs: 120,
                accumulated_rpz: 12,
                total_shares: 873,
                total_valued_shares: 47_368,
                entries_count: 3,
                ..get_predefined_yield_farm_ins1(1)
            },
        );

        //Yield farm balance checks.
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tn1_yield_farm_account),
            bsx_tkn1_yield_farm_bsx_balance
        );

        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_yield_farm_account),
            (bsx_tkn2_yield_farm_bsx_balance - unclaimable_rewards)
        );

        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &global_farm_account),
            global_farm_bsx_balance + unclaimable_rewards
        );

        assert!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[4]).is_none());

        // withdraw 3A
        let global_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &global_farm_account);
        let bsx_tkn1_yield_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tn1_yield_farm_account);
        let bsx_tkn2_yield_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_yield_farm_account);

        let unclaimable_rewards = 2_441_971;
        let withdrawn_amount = 486;
        let expected_deposit_destroyed = true;
        assert_eq!(
            LiquidityMining::withdraw_lp_shares(
                PREDEFINED_DEPOSIT_IDS[6],
                GC_BSX_TKN1_YIELD_FARM_ID,
                unclaimable_rewards,
            )
            .unwrap(),
            (GLOBAL_FARM_ID, withdrawn_amount, expected_deposit_destroyed)
        );

        assert_eq!(
            LiquidityMining::global_farm(GC_FARM).unwrap(),
            GlobalFarmData {
                updated_at: 25,
                accumulated_rpz: 12,
                accumulated_rewards: 231_650,
                paid_accumulated_rewards: 1_164_400,
                total_shares_z: 494480,
                ..get_predefined_global_farm_ins1(2)
            }
        );

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                updated_at: 25,
                accumulated_rpvs: 60,
                accumulated_rpz: 12,
                total_shares: 80,
                total_valued_shares: 4_160,
                entries_count: 1,
                ..get_predefined_yield_farm_ins1(0)
            },
        );

        //Yield farm balance checks.
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tn1_yield_farm_account),
            bsx_tkn1_yield_farm_bsx_balance - unclaimable_rewards
        );

        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_yield_farm_account),
            bsx_tkn2_yield_farm_bsx_balance
        );

        //Global farm balance checks.
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &global_farm_account),
            global_farm_bsx_balance + unclaimable_rewards
        );

        assert!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[6]).is_none());

        // withdraw 2A
        let global_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &global_farm_account);
        let bsx_tkn1_yield_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tn1_yield_farm_account);
        let bsx_tkn2_yield_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_yield_farm_account);

        let unclaimable_rewards = 267_429;
        let withdrawn_amount = 80;
        let expected_deposit_destroyed = true;
        assert_eq!(
            LiquidityMining::withdraw_lp_shares(
                PREDEFINED_DEPOSIT_IDS[1],
                GC_BSX_TKN1_YIELD_FARM_ID,
                unclaimable_rewards,
            )
            .unwrap(),
            (GLOBAL_FARM_ID, withdrawn_amount, expected_deposit_destroyed)
        );

        assert_eq!(
            LiquidityMining::global_farm(GC_FARM).unwrap(),
            GlobalFarmData {
                updated_at: 25,
                accumulated_rpz: 12,
                accumulated_rewards: 231_650,
                paid_accumulated_rewards: 1_164_400,
                total_shares_z: 473_680,
                ..get_predefined_global_farm_ins1(2)
            }
        );

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                updated_at: 25,
                accumulated_rpvs: 60,
                accumulated_rpz: 12,
                total_shares: 0,
                total_valued_shares: 0,
                entries_count: 0,
                ..get_predefined_yield_farm_ins1(0)
            },
        );

        //Yield farm balance checks.
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tn1_yield_farm_account),
            bsx_tkn1_yield_farm_bsx_balance - unclaimable_rewards
        );

        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_yield_farm_account),
            bsx_tkn2_yield_farm_bsx_balance
        );

        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &global_farm_account),
            global_farm_bsx_balance + unclaimable_rewards
        );

        assert!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[1]).is_none());

        // withdraw 1B
        let global_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &global_farm_account);
        let bsx_tkn1_yield_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tn1_yield_farm_account);
        let bsx_tkn2_yield_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_yield_farm_account);

        let unclaimable_rewards = 30_001;
        assert_ok!(LiquidityMining::withdraw_lp_shares(
            PREDEFINED_DEPOSIT_IDS[2],
            GC_BSX_TKN2_YIELD_FARM_ID,
            unclaimable_rewards
        ));

        assert_eq!(
            LiquidityMining::global_farm(GC_FARM).unwrap(),
            GlobalFarmData {
                updated_at: 25,
                accumulated_rpz: 12,
                accumulated_rewards: 231_650,
                paid_accumulated_rewards: 1_164_400,
                total_shares_z: 471_680,
                ..get_predefined_global_farm_ins1(2)
            }
        );

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                updated_at: 25,
                accumulated_rpvs: 60,
                accumulated_rpz: 12,
                total_shares: 0,
                total_valued_shares: 0,
                ..PREDEFINED_YIELD_FARMS_INS1.with(|v| v[0].clone())
            },
        );

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN2_AMM, GC_FARM, GC_BSX_TKN2_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                updated_at: 25,
                accumulated_rpvs: 120,
                accumulated_rpz: 12,
                total_shares: 848,
                total_valued_shares: 47_168,
                entries_count: 2,
                ..get_predefined_yield_farm_ins1(1)
            },
        );

        //Yield farm balance checks.
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tn1_yield_farm_account),
            bsx_tkn1_yield_farm_bsx_balance
        );

        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_yield_farm_account),
            bsx_tkn2_yield_farm_bsx_balance - unclaimable_rewards
        );

        //Global farm balance checks.
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &global_farm_account),
            global_farm_bsx_balance + unclaimable_rewards
        );

        assert_eq!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[2]), None);

        // withdraw 4B
        let global_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &global_farm_account);
        let bsx_tkn1_yield_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tn1_yield_farm_account);
        let bsx_tkn2_yield_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_yield_farm_account);

        let unclaimable_rewards = 96_473;
        let withdrawn_shares = 48;
        let expected_deposit_destroyed = true;
        assert_eq!(
            LiquidityMining::withdraw_lp_shares(
                PREDEFINED_DEPOSIT_IDS[5],
                GC_BSX_TKN2_YIELD_FARM_ID,
                unclaimable_rewards,
            )
            .unwrap(),
            (GLOBAL_FARM_ID, withdrawn_shares, expected_deposit_destroyed)
        );

        assert_eq!(
            LiquidityMining::global_farm(GC_FARM).unwrap(),
            GlobalFarmData {
                updated_at: 25,
                accumulated_rpz: 12,
                accumulated_rewards: 231_650,
                paid_accumulated_rewards: 1_164_400,
                total_shares_z: 464_000,
                ..get_predefined_global_farm_ins1(2)
            }
        );

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN2_AMM, GC_FARM, GC_BSX_TKN2_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                updated_at: 25,
                accumulated_rpvs: 120,
                accumulated_rpz: 12,
                total_shares: 800,
                total_valued_shares: 46_400,
                entries_count: 1,
                ..get_predefined_yield_farm_ins1(1)
            },
        );

        //Yield farm balances checks.
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tn1_yield_farm_account),
            bsx_tkn1_yield_farm_bsx_balance
        );
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_yield_farm_account),
            bsx_tkn2_yield_farm_bsx_balance - unclaimable_rewards
        );

        //Global farm balance checks.
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &global_farm_account),
            global_farm_bsx_balance + unclaimable_rewards
        );

        assert!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[5]).is_none());

        // withdraw 2B
        let global_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &global_farm_account);
        let bsx_tkn1_yield_farm_lp_shares_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tn1_yield_farm_account);

        let unclaimable_rewards = 5_911_539;
        let withdrawn_shares = 800;
        let expected_deposit_destroyed = true;
        assert_eq!(
            LiquidityMining::withdraw_lp_shares(
                PREDEFINED_DEPOSIT_IDS[3],
                GC_BSX_TKN2_YIELD_FARM_ID,
                unclaimable_rewards
            )
            .unwrap(),
            (GLOBAL_FARM_ID, withdrawn_shares, expected_deposit_destroyed)
        );

        assert_eq!(
            LiquidityMining::global_farm(GC_FARM).unwrap(),
            GlobalFarmData {
                updated_at: 25,
                accumulated_rpz: 12,
                accumulated_rewards: 231_650,
                paid_accumulated_rewards: 1_164_400,
                total_shares_z: 0,
                ..get_predefined_global_farm_ins1(2)
            }
        );

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN2_AMM, GC_FARM, GC_BSX_TKN2_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                updated_at: 25,
                accumulated_rpvs: 120,
                accumulated_rpz: 12,
                total_shares: 0,
                total_valued_shares: 0,
                entries_count: 0,
                ..get_predefined_yield_farm_ins1(1)
            },
        );

        //Yield farm balances checks.
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tn1_yield_farm_account),
            bsx_tkn1_yield_farm_lp_shares_balance
        );

        //Global farm balance checks.
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &global_farm_account),
            global_farm_bsx_balance + 5_911_539 //5_911_539 unclaimable rewards after withdrawn
        );

        assert!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[2]).is_none());
    });

    //Charlie's farm incentivize KSM and reward currency is ACA
    //This test check if correct currency is transferred if rewards and incentvized
    //assets are different, otherwise farm behavior is the same as in test above.
    predefined_test_ext().execute_with(|| {
        set_block_number(1_800); //period 18

        let deposited_amount = 50;
        assert_ok!(LiquidityMining::deposit_lp_shares(
            CHARLIE_FARM,
            CHARLIE_ACA_KSM_YIELD_FARM_ID,
            ACA_KSM_AMM,
            deposited_amount,
            |_, _| { Ok(50_u128) }
        ));

        const DEPOSIT_ID: DepositId = 1;
        let global_farm_id = CHARLIE_FARM;
        assert_eq!(
            LiquidityMining::deposit(DEPOSIT_ID).unwrap(),
            DepositData {
                shares: 50,
                amm_pool_id: ACA_KSM_AMM,
                yield_farm_entries: vec![YieldFarmEntry {
                    global_farm_id,
                    yield_farm_id: CHARLIE_ACA_KSM_YIELD_FARM_ID,
                    accumulated_rpvs: 0,
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

        let expected_deposit_destroyed = true;
        assert_eq!(
            LiquidityMining::withdraw_lp_shares(DEPOSIT_ID, CHARLIE_ACA_KSM_YIELD_FARM_ID, 0).unwrap(),
            (CHARLIE_FARM, deposited_amount, expected_deposit_destroyed)
        );
    });
}

#[test]
fn withdraw_with_multiple_entries_and_flush_should_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        let alice_bsx_tkn1_lp_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE);

        //Redeposit to multiple yield farms.
        assert_ok!(LiquidityMining::redeposit_lp_shares(
            DAVE_FARM,
            DAVE_BSX_TKN1_YIELD_FARM_ID,
            PREDEFINED_DEPOSIT_IDS[0],
            |_, _| { Ok(10_u128) },
        ));

        assert_ok!(LiquidityMining::redeposit_lp_shares(
            EVE_FARM,
            EVE_BSX_TKN1_YIELD_FARM_ID,
            PREDEFINED_DEPOSIT_IDS[0],
            |_, _| { Ok(10_u128) },
        ));
        //NOTE: predefined_deposit_ids[0] is deposited in 3 yield farms now.

        //Stop yield farm.
        assert_ok!(LiquidityMining::stop_yield_farm(EVE, EVE_FARM, BSX_TKN1_AMM));
        //Stop and destroy all yield farms so it can be flushed.
        assert_ok!(LiquidityMining::stop_yield_farm(DAVE, DAVE_FARM, BSX_TKN1_AMM));
        assert_ok!(LiquidityMining::destroy_yield_farm(
            DAVE,
            DAVE_FARM,
            DAVE_BSX_TKN1_YIELD_FARM_ID,
            BSX_TKN1_AMM
        ));

        assert_ok!(LiquidityMining::destroy_global_farm(DAVE, DAVE_FARM));

        let unclaimable_rewards = 0;
        let shares_amount = 50;
        let expected_deposit_destroyed = false;
        assert_eq!(
            LiquidityMining::withdraw_lp_shares(
                PREDEFINED_DEPOSIT_IDS[0],
                GC_BSX_TKN1_YIELD_FARM_ID,
                unclaimable_rewards
            )
            .unwrap(),
            (GC_FARM, shares_amount, expected_deposit_destroyed)
        );

        assert_eq!(
            LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[0])
                .unwrap()
                .yield_farm_entries
                .len(),
            2
        );

        //LP tokens should not be unlocked.
        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE),
            alice_bsx_tkn1_lp_shares_balance
        );

        //This withdraw should flush yield and global farms.
        let expected_deposit_destroyed = false;
        assert_eq!(
            LiquidityMining::withdraw_lp_shares(
                PREDEFINED_DEPOSIT_IDS[0],
                DAVE_BSX_TKN1_YIELD_FARM_ID,
                unclaimable_rewards
            )
            .unwrap(),
            (DAVE_FARM, shares_amount, expected_deposit_destroyed)
        );

        assert_eq!(
            LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[0])
                .unwrap()
                .yield_farm_entries
                .len(),
            1
        );

        //LP tokens should not be unlocked.
        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE),
            alice_bsx_tkn1_lp_shares_balance
        );

        assert!(LiquidityMining::yield_farm((BSX_TKN1_AMM, DAVE_FARM, DAVE_BSX_TKN1_YIELD_FARM_ID)).is_none());
        assert!(LiquidityMining::global_farm(DAVE_FARM).is_none());

        //This withdraw should flush yield and global farms.
        let expected_deposit_destroyed = true;
        assert_eq!(
            LiquidityMining::withdraw_lp_shares(
                PREDEFINED_DEPOSIT_IDS[0],
                EVE_BSX_TKN1_YIELD_FARM_ID,
                unclaimable_rewards
            )
            .unwrap(),
            (EVE_FARM, shares_amount, expected_deposit_destroyed)
        );

        //Last withdraw from deposit should flush deposit.
        assert!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[0]).is_none());
    });
}

#[test]
fn withdraw_shares_from_destroyed_farm_should_work() {
    //This is the case when yield farm is removed and global farm is destroyed.
    //In this case only amm shares should be withdrawn.

    predefined_test_ext_with_deposits().execute_with(|| {
        //Stop all yield farms in the global farm.
        assert_ok!(LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN1_AMM));
        assert_ok!(LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN2_AMM));

        //Remove all yield farms from global farm.
        assert_ok!(LiquidityMining::destroy_yield_farm(
            GC,
            GC_FARM,
            GC_BSX_TKN1_YIELD_FARM_ID,
            BSX_TKN1_AMM
        ));
        assert_ok!(LiquidityMining::destroy_yield_farm(
            GC,
            GC_FARM,
            GC_BSX_TKN2_YIELD_FARM_ID,
            BSX_TKN2_AMM
        ));

        //Destroy farm.
        assert_ok!(LiquidityMining::destroy_global_farm(GC, GC_FARM));

        assert!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID))
                .unwrap()
                .is_deleted()
        );
        assert!(
            LiquidityMining::yield_farm((BSX_TKN2_AMM, GC_FARM, GC_BSX_TKN2_YIELD_FARM_ID))
                .unwrap()
                .is_deleted()
        );
        assert_eq!(LiquidityMining::global_farm(GC_FARM).unwrap().state, FarmState::Deleted);

        let test_data = vec![
            (
                ALICE,
                0,
                50,
                2_u64,
                GC_BSX_TKN1_YIELD_FARM_ID,
                BSX_TKN1_SHARE_ID,
                BSX_TKN1_AMM,
            ),
            (
                BOB,
                1,
                80,
                1_u64,
                GC_BSX_TKN1_YIELD_FARM_ID,
                BSX_TKN1_SHARE_ID,
                BSX_TKN1_AMM,
            ),
            (
                BOB,
                2,
                25,
                3_u64,
                GC_BSX_TKN2_YIELD_FARM_ID,
                BSX_TKN2_SHARE_ID,
                BSX_TKN2_AMM,
            ),
            (
                BOB,
                3,
                800,
                2_u64,
                GC_BSX_TKN2_YIELD_FARM_ID,
                BSX_TKN2_SHARE_ID,
                BSX_TKN2_AMM,
            ),
            (
                ALICE,
                4,
                87,
                1_u64,
                GC_BSX_TKN2_YIELD_FARM_ID,
                BSX_TKN2_SHARE_ID,
                BSX_TKN2_AMM,
            ),
            (
                ALICE,
                5,
                48,
                0_u64,
                GC_BSX_TKN2_YIELD_FARM_ID,
                BSX_TKN2_SHARE_ID,
                BSX_TKN2_AMM,
            ),
            (
                ALICE,
                6,
                486,
                0_u64,
                GC_BSX_TKN1_YIELD_FARM_ID,
                BSX_TKN1_SHARE_ID,
                BSX_TKN1_AMM,
            ),
        ];

        for (_caller, deposit_idx, withdrawn_shares, _farm_entries_left, yield_farm_id, _lp_token, _amm_pool_id) in
            test_data
        {
            let expected_deposit_destroyed = true;
            assert_eq!(
                LiquidityMining::withdraw_lp_shares(PREDEFINED_DEPOSIT_IDS[deposit_idx], yield_farm_id, 0,).unwrap(),
                (GC_FARM, withdrawn_shares, expected_deposit_destroyed)
            );

            //check if deposit was removed.
            assert!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[deposit_idx]).is_none());
        }
    });
}

#[test]
fn withdraw_shares_from_canceled_yield_farm_should_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        set_block_number(10_000);

        // Stop yield farm before withdraw test.
        assert_ok!(LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN1_AMM));

        let global_farm_account = LiquidityMining::farm_account_id(GC_FARM).unwrap();
        let yield_farm_account = LiquidityMining::farm_account_id(GC_BSX_TKN1_YIELD_FARM_ID).unwrap();

        //1-th withdraw
        let yield_farm_bsx_balance = Tokens::free_balance(BSX, &yield_farm_account);
        let global_farm_bsx_balance = Tokens::free_balance(BSX, &global_farm_account);

        let global_farm = LiquidityMining::global_farm(GC_FARM).unwrap();
        let yield_farm = LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID)).unwrap();

        let unclaimable_rewards = 168_270;
        let withdrawn_amount = 50;
        let expected_deposit_destroyed = true;
        assert_eq!(
            LiquidityMining::withdraw_lp_shares(
                PREDEFINED_DEPOSIT_IDS[0],
                GC_BSX_TKN1_YIELD_FARM_ID,
                unclaimable_rewards
            )
            .unwrap(),
            (GC_FARM, withdrawn_amount, expected_deposit_destroyed)
        );

        assert_eq!(LiquidityMining::global_farm(GC_FARM).unwrap(), global_farm);

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                total_shares: yield_farm.total_shares - withdrawn_amount,
                total_valued_shares: yield_farm.total_valued_shares - 2500,
                entries_count: 2,
                ..yield_farm
            }
        );

        assert!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[0]).is_none());

        assert_eq!(
            Tokens::free_balance(BSX, &global_farm_account),
            global_farm_bsx_balance + unclaimable_rewards
        );

        assert_eq!(
            Tokens::free_balance(BSX, &yield_farm_account),
            yield_farm_bsx_balance - unclaimable_rewards
        );

        //2-nd withdraw
        let yield_farm_bsx_balance = Tokens::free_balance(BSX, &yield_farm_account);
        let global_farm_bsx_balance = Tokens::free_balance(BSX, &global_farm_account);

        let global_farm = LiquidityMining::global_farm(GC_FARM).unwrap();
        let yield_farm = LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID)).unwrap();

        let unclaimable_rewards = 2_055_086;
        let shares_amount = 486;
        let valued_shares_amount = 38_880;

        let expected_deposit_destroyed = true;
        assert_eq!(
            LiquidityMining::withdraw_lp_shares(
                PREDEFINED_DEPOSIT_IDS[6],
                GC_BSX_TKN1_YIELD_FARM_ID,
                unclaimable_rewards
            )
            .unwrap(),
            (GC_FARM, shares_amount, expected_deposit_destroyed)
        );

        assert_eq!(LiquidityMining::global_farm(GC_FARM).unwrap(), global_farm);

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                total_shares: yield_farm.total_shares - shares_amount,
                total_valued_shares: yield_farm.total_valued_shares - valued_shares_amount,
                entries_count: 1,
                ..yield_farm
            }
        );

        assert!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[6]).is_none());

        assert_eq!(
            Tokens::free_balance(BSX, &global_farm_account),
            global_farm_bsx_balance + unclaimable_rewards
        );

        assert_eq!(
            Tokens::free_balance(BSX, &yield_farm_account),
            yield_farm_bsx_balance - unclaimable_rewards
        );

        //3-th withdraw
        let yield_farm_bsx_balance = Tokens::free_balance(BSX, &yield_farm_account);
        let global_farm_bsx_balance = Tokens::free_balance(BSX, &global_farm_account);

        let global_farm = LiquidityMining::global_farm(GC_FARM).unwrap();
        let yield_farm = LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID)).unwrap();

        let unclaimable_rewards = 228_572;
        let shares_amount = 80;

        let expected_deposit_destroyed = true;
        assert_eq!(
            LiquidityMining::withdraw_lp_shares(
                PREDEFINED_DEPOSIT_IDS[1],
                GC_BSX_TKN1_YIELD_FARM_ID,
                unclaimable_rewards
            )
            .unwrap(),
            (GC_FARM, shares_amount, expected_deposit_destroyed)
        );

        assert_eq!(LiquidityMining::global_farm(GC_FARM).unwrap(), global_farm);

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                total_shares: 0,
                total_valued_shares: 0,
                entries_count: 0,
                ..yield_farm
            }
        );

        assert!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[1]).is_none());

        assert_eq!(
            Tokens::free_balance(BSX, &global_farm_account),
            global_farm_bsx_balance + unclaimable_rewards
        );

        assert_eq!(
            Tokens::free_balance(BSX, &yield_farm_account),
            yield_farm_bsx_balance - unclaimable_rewards
        );
    });
}

#[test]
fn withdraw_shares_from_removed_pool_should_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        set_block_number(10_000);

        //Stop yield farm before removing.
        assert_ok!(LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN1_AMM));

        //Destroy yield farm before test
        assert_ok!(LiquidityMining::destroy_yield_farm(
            GC,
            GC_FARM,
            GC_BSX_TKN1_YIELD_FARM_ID,
            BSX_TKN1_AMM
        ));

        assert!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID))
                .unwrap()
                .is_deleted(),
        );

        let global_farm = LiquidityMining::global_farm(GC_FARM).unwrap();

        let global_farm_account = LiquidityMining::farm_account_id(GC_FARM).unwrap();
        let global_farm_bsx_balance = Tokens::free_balance(BSX, &global_farm_account);
        let alice_bsx_balance = Tokens::free_balance(BSX, &ALICE);

        let yield_farm = LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID)).unwrap();
        let shares_amount = 50;
        //1-th withdraw
        let expected_deposit_destroyed = true;
        assert_eq!(
            LiquidityMining::withdraw_lp_shares(PREDEFINED_DEPOSIT_IDS[0], GC_BSX_TKN1_YIELD_FARM_ID, 0).unwrap(),
            (GC_FARM, shares_amount, expected_deposit_destroyed)
        );

        assert!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[0]).is_none());

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                total_shares: 566,
                total_valued_shares: 43_040,
                entries_count: 2,
                ..yield_farm
            }
        );

        assert_eq!(LiquidityMining::global_farm(GC_FARM).unwrap(), global_farm);

        //Removed yield farm don't pay rewards, only transfers amm shares.
        assert_eq!(Tokens::free_balance(BSX, &ALICE), alice_bsx_balance);
        assert_eq!(Tokens::free_balance(BSX, &global_farm_account), global_farm_bsx_balance);

        //2-nd withdraw
        let alice_bsx_balance = Tokens::free_balance(BSX, &ALICE);
        let shares_amount = 486;

        let yield_farm = LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID)).unwrap();
        let expected_deposit_destroyed = true;
        assert_eq!(
            LiquidityMining::withdraw_lp_shares(PREDEFINED_DEPOSIT_IDS[6], GC_BSX_TKN1_YIELD_FARM_ID, 0,).unwrap(),
            (GC_FARM, shares_amount, expected_deposit_destroyed)
        );

        assert!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[6]).is_none());

        assert_eq!(LiquidityMining::global_farm(GC_FARM).unwrap(), global_farm);

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                total_shares: 80,
                total_valued_shares: 4_160,
                entries_count: 1,
                ..yield_farm
            }
        );

        //removed yield farm don't pay rewards, only return LP shares
        assert_eq!(Tokens::free_balance(BSX, &ALICE), alice_bsx_balance);
        assert_eq!(Tokens::free_balance(BSX, &global_farm_account), global_farm_bsx_balance);

        //3-th withdraw
        let bob_bsx_balance = Tokens::free_balance(BSX, &BOB);
        let shares_amount = 80;

        let expected_deposit_destroyed = true;
        assert_eq!(
            LiquidityMining::withdraw_lp_shares(PREDEFINED_DEPOSIT_IDS[1], GC_BSX_TKN1_YIELD_FARM_ID, 0).unwrap(),
            (GC_FARM, shares_amount, expected_deposit_destroyed)
        );

        //Last withdraw should flush yield farm if it's deleted
        assert!(LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID)).is_none());

        assert!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[1]).is_none());

        assert_eq!(
            LiquidityMining::global_farm(GC_FARM).unwrap(),
            GlobalFarmData {
                yield_farms_count: (1, 1), //this value changed because last deposit flushed deleted yield farm
                ..global_farm
            }
        );

        //Removed yield farm don't pay rewards, only return LP shares.
        assert_eq!(Tokens::free_balance(BSX, &BOB), bob_bsx_balance);
        assert_eq!(Tokens::free_balance(BSX, &global_farm_account), global_farm_bsx_balance);
    });
}

#[test]
fn withdraw_shares_yield_farm_entry_not_found_should_not_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        const DEPOSIT_ID: DepositId = 1;
        const NOT_FOUND_ENTRY_ID: YieldFarmId = 999_999;
        assert_noop!(
            LiquidityMining::withdraw_lp_shares(DEPOSIT_ID, NOT_FOUND_ENTRY_ID, 0),
            Error::<Test, Instance1>::YieldFarmEntryNotFound
        );
    });
}

#[test]
fn withdraw_shares_deposit_not_found_should_not_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        assert_noop!(
            LiquidityMining::withdraw_lp_shares(72_334_321_125_861_359_621, GC_BSX_TKN1_YIELD_FARM_ID, 0),
            Error::<Test, Instance1>::DepositNotFound
        );
    });
}
