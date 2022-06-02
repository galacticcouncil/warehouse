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
fn withdraw_shares_should_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        const REWARD_CURRENCY: u32 = BSX;
        const GLOBAL_FARM_ID: GlobalFarmId = GC_FARM;

        let bsx_tn1_yield_farm_account = LiquidityMining::farm_account_id(BSX_TKN1_YIELD_FARM_ID).unwrap();
        let bsx_tkn2_yield_farm_account = LiquidityMining::farm_account_id(BSX_TKN2_YIELD_FARM_ID).unwrap();
        let global_farm_account = LiquidityMining::farm_account_id(GC_FARM).unwrap();

        // This balance is used to transfer unclaimable_rewards from yield farm to global farm.
        // Claiming is not part of withdraw_shares() so balance need to be set.
        Tokens::set_balance(Origin::root(), bsx_tn1_yield_farm_account, BSX, 100_000_000_000, 0).unwrap();
        Tokens::set_balance(Origin::root(), bsx_tkn2_yield_farm_account, BSX, 100_000_000_000, 0).unwrap();

        // withdraw 1A
        let bsx_tkn1_alice_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE);
        let bsx_tkn1_stash_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH);
        let bsx_tkn2_stash_shares_balance = Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH);
        let global_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &global_farm_account);
        let bsx_tkn2_yield_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_yield_farm_account);

        let unclaimable_rewards = 100_000;
        let withdrawn_amount = 50;
        assert_eq!(
            LiquidityMining::withdraw_lp_shares(
                ALICE,
                PREDEFINED_DEPOSIT_IDS[0],
                BSX_TKN1_YIELD_FARM_ID,
                unclaimable_rewards,
            )
            .unwrap(),
            (GLOBAL_FARM_ID, BSX_TKN1_YIELD_FARM_ID, withdrawn_amount,)
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
                ..PREDEFINED_GLOBAL_FARMS[2].clone()
            }
        );

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, BSX_TKN1_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                updated_at: 25,
                accumulated_rpvs: 60,
                accumulated_rpz: 12,
                total_shares: 566,
                total_valued_shares: 43_040,
                entries_count: 2,
                ..PREDEFINED_YIELD_FARMS.with(|v| v[0].clone())
            },
        );

        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE),
            bsx_tkn1_alice_amm_shares_balance + withdrawn_amount
        );

        //stash shares account balances checks
        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH),
            bsx_tkn1_stash_shares_balance - withdrawn_amount
        );
        assert_eq!(
            Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH),
            bsx_tkn2_stash_shares_balance
        );

        //yield farm balance checks
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tn1_yield_farm_account),
            bsx_tkn2_yield_farm_bsx_balance - unclaimable_rewards
        );
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_yield_farm_account),
            bsx_tkn2_yield_farm_bsx_balance
        );

        //global farm balance checks
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &global_farm_account),
            global_farm_bsx_balance + unclaimable_rewards
        );

        assert_eq!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[0]), None);

        set_block_number(12_800);

        // withdraw 3B
        let bsx_tkn2_alice_amm_shares_balance = Tokens::free_balance(BSX_TKN2_SHARE_ID, &ALICE);
        let bsx_tkn1_pallet_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH);
        let bsx_tkn2_pallet_amm_shares_balance = Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH);
        let global_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &global_farm_account);
        let bsx_tkn1_yield_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tn1_yield_farm_account);
        let bsx_tkn2_yield_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_yield_farm_account);

        let unclaimable_rewards = 32_786;
        let withdrawn_amount = 87;
        assert_eq!(
            LiquidityMining::withdraw_lp_shares(
                ALICE,
                PREDEFINED_DEPOSIT_IDS[4],
                BSX_TKN2_YIELD_FARM_ID,
                unclaimable_rewards
            )
            .unwrap(),
            (GLOBAL_FARM_ID, BSX_TKN2_YIELD_FARM_ID, withdrawn_amount)
        );

        assert_eq!(
            LiquidityMining::global_farm(GC_FARM).unwrap(),
            GlobalFarmData {
                updated_at: 25,
                accumulated_rpz: 12,
                accumulated_rewards: 231_650,
                paid_accumulated_rewards: 1_164_400,
                total_shares_z: 688_880,
                ..PREDEFINED_GLOBAL_FARMS[2].clone()
            }
        );

        // this farm should not change
        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, BSX_TKN1_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                updated_at: 25,
                accumulated_rpvs: 60,
                accumulated_rpz: 12,
                total_shares: 566,
                total_valued_shares: 43_040,
                entries_count: 2,
                ..PREDEFINED_YIELD_FARMS.with(|v| v[0].clone())
            },
        );

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN2_AMM, GC_FARM, BSX_TKN2_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                updated_at: 25,
                accumulated_rpvs: 120,
                accumulated_rpz: 12,
                total_shares: 873,
                total_valued_shares: 47_368,
                entries_count: 3,
                ..PREDEFINED_YIELD_FARMS.with(|v| v[1].clone())
            },
        );

        //user balances checks
        assert_eq!(
            Tokens::free_balance(BSX_TKN2_SHARE_ID, &ALICE),
            bsx_tkn2_alice_amm_shares_balance + withdrawn_amount
        );

        //stash shares account balances checks
        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH),
            bsx_tkn1_pallet_amm_shares_balance
        );
        assert_eq!(
            Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH),
            bsx_tkn2_pallet_amm_shares_balance - withdrawn_amount
        );

        //yield farm balance checks
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

        assert_eq!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[4]), None);

        // withdraw 3A
        let bsx_tkn1_alice_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE);
        let bsx_tkn1_pallet_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH);
        let bsx_tkn2_pallet_amm_shares_balance = Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH);
        let global_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &global_farm_account);
        let bsx_tkn1_yield_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tn1_yield_farm_account);
        let bsx_tkn2_yield_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_yield_farm_account);

        let unclaimable_rewards = 2_441_971;
        let withdrawn_amount = 486;
        assert_eq!(
            LiquidityMining::withdraw_lp_shares(
                ALICE,
                PREDEFINED_DEPOSIT_IDS[6],
                BSX_TKN1_YIELD_FARM_ID,
                unclaimable_rewards,
            )
            .unwrap(),
            (GLOBAL_FARM_ID, BSX_TKN1_YIELD_FARM_ID, withdrawn_amount)
        );

        assert_eq!(
            LiquidityMining::global_farm(GC_FARM).unwrap(),
            GlobalFarmData {
                updated_at: 25,
                accumulated_rpz: 12,
                accumulated_rewards: 231_650,
                paid_accumulated_rewards: 1_164_400,
                total_shares_z: 494480,
                ..PREDEFINED_GLOBAL_FARMS[2].clone()
            }
        );

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, BSX_TKN1_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                updated_at: 25,
                accumulated_rpvs: 60,
                accumulated_rpz: 12,
                total_shares: 80,
                total_valued_shares: 4_160,
                entries_count: 1,
                ..PREDEFINED_YIELD_FARMS.with(|v| v[0].clone())
            },
        );

        //user balances checks
        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE),
            bsx_tkn1_alice_amm_shares_balance + withdrawn_amount
        );

        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH),
            bsx_tkn1_pallet_amm_shares_balance - withdrawn_amount
        );
        assert_eq!(
            Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH),
            bsx_tkn2_pallet_amm_shares_balance
        );

        //yield farm balance checks
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tn1_yield_farm_account),
            bsx_tkn1_yield_farm_bsx_balance - unclaimable_rewards
        );

        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_yield_farm_account),
            bsx_tkn2_yield_farm_bsx_balance
        );

        //global farm balance checks
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &global_farm_account),
            global_farm_bsx_balance + unclaimable_rewards
        );

        assert_eq!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[6]), None);

        // withdraw 2A
        let bsx_tkn1_bob_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &BOB);
        let bsx_tkn2_pallet_amm_shares_balance = Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH);
        let global_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &global_farm_account);
        let bsx_tkn1_yield_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tn1_yield_farm_account);
        let bsx_tkn2_yield_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_yield_farm_account);

        let unclaimable_rewards = 267_429;
        let withdrawn_amount = 80;
        assert_eq!(
            LiquidityMining::withdraw_lp_shares(
                BOB,
                PREDEFINED_DEPOSIT_IDS[1],
                BSX_TKN1_YIELD_FARM_ID,
                unclaimable_rewards,
            )
            .unwrap(),
            (GLOBAL_FARM_ID, BSX_TKN1_YIELD_FARM_ID, withdrawn_amount)
        );

        assert_eq!(
            LiquidityMining::global_farm(GC_FARM).unwrap(),
            GlobalFarmData {
                updated_at: 25,
                accumulated_rpz: 12,
                accumulated_rewards: 231_650,
                paid_accumulated_rewards: 1_164_400,
                total_shares_z: 473_680,
                ..PREDEFINED_GLOBAL_FARMS[2].clone()
            }
        );

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, BSX_TKN1_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                updated_at: 25,
                accumulated_rpvs: 60,
                accumulated_rpz: 12,
                total_shares: 0,
                total_valued_shares: 0,
                entries_count: 0,
                ..PREDEFINED_YIELD_FARMS.with(|v| v[0].clone())
            },
        );

        //user balances checks
        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &BOB),
            bsx_tkn1_bob_amm_shares_balance + withdrawn_amount
        );

        assert_eq!(Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH), 0);
        assert_eq!(
            Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH),
            bsx_tkn2_pallet_amm_shares_balance
        );

        //yield farm balance checks
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

        assert_eq!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[1]), None);

        // withdraw 1B
        let bsx_tkn2_bob_amm_shares_balance = Tokens::free_balance(BSX_TKN2_SHARE_ID, &BOB);
        let bsx_tkn2_pallet_amm_shares_balance = Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH);
        let global_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &global_farm_account);
        let bsx_tkn1_yield_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tn1_yield_farm_account);
        let bsx_tkn2_yield_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_yield_farm_account);

        let unclaimable_rewards = 30_001;
        let withdrawn_shares = 25;
        assert_ok!(LiquidityMining::withdraw_lp_shares(
            BOB,
            PREDEFINED_DEPOSIT_IDS[2],
            BSX_TKN2_YIELD_FARM_ID,
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
                ..PREDEFINED_GLOBAL_FARMS[2].clone()
            }
        );

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, BSX_TKN1_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                updated_at: 25,
                accumulated_rpvs: 60,
                accumulated_rpz: 12,
                total_shares: 0,
                total_valued_shares: 0,
                ..PREDEFINED_YIELD_FARMS.with(|v| v[0].clone())
            },
        );

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN2_AMM, GC_FARM, BSX_TKN2_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                updated_at: 25,
                accumulated_rpvs: 120,
                accumulated_rpz: 12,
                total_shares: 848,
                total_valued_shares: 47_168,
                entries_count: 2,
                ..PREDEFINED_YIELD_FARMS.with(|v| v[1].clone())
            },
        );

        //user balances checks
        assert_eq!(
            Tokens::free_balance(BSX_TKN2_SHARE_ID, &BOB),
            bsx_tkn2_bob_amm_shares_balance + withdrawn_shares
        );

        assert_eq!(Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH), 0);
        assert_eq!(
            Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH),
            bsx_tkn2_pallet_amm_shares_balance - withdrawn_shares
        );

        //yield farm balance checks
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tn1_yield_farm_account),
            bsx_tkn1_yield_farm_bsx_balance
        );

        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_yield_farm_account),
            bsx_tkn2_yield_farm_bsx_balance - unclaimable_rewards
        );

        //global farm balance checks
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &global_farm_account),
            global_farm_bsx_balance + unclaimable_rewards
        );

        assert_eq!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[2]), None);

        // withdraw 4B
        let bsx_tkn2_alice_amm_shares_balance = Tokens::free_balance(BSX_TKN2_SHARE_ID, &ALICE);
        let bsx_tkn2_pallet_amm_shares_balance = Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH);
        let global_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &global_farm_account);
        let bsx_tkn1_yield_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tn1_yield_farm_account);
        let bsx_tkn2_yield_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_yield_farm_account);

        let unclaimable_rewards = 96_473;
        let withdrawn_shares = 48;
        assert_eq!(
            LiquidityMining::withdraw_lp_shares(
                ALICE,
                PREDEFINED_DEPOSIT_IDS[5],
                BSX_TKN2_YIELD_FARM_ID,
                unclaimable_rewards,
            )
            .unwrap(),
            (GLOBAL_FARM_ID, BSX_TKN2_YIELD_FARM_ID, withdrawn_shares,)
        );

        assert_eq!(
            LiquidityMining::global_farm(GC_FARM).unwrap(),
            GlobalFarmData {
                updated_at: 25,
                accumulated_rpz: 12,
                accumulated_rewards: 231_650,
                paid_accumulated_rewards: 1_164_400,
                total_shares_z: 464_000,
                ..PREDEFINED_GLOBAL_FARMS[2].clone()
            }
        );

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN2_AMM, GC_FARM, BSX_TKN2_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                updated_at: 25,
                accumulated_rpvs: 120,
                accumulated_rpz: 12,
                total_shares: 800,
                total_valued_shares: 46_400,
                entries_count: 1,
                ..PREDEFINED_YIELD_FARMS.with(|v| v[1].clone())
            },
        );

        //user balances checks
        assert_eq!(
            Tokens::free_balance(BSX_TKN2_SHARE_ID, &ALICE),
            bsx_tkn2_alice_amm_shares_balance + withdrawn_shares
        );

        assert_eq!(Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH), 0);
        assert_eq!(
            Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH),
            bsx_tkn2_pallet_amm_shares_balance - withdrawn_shares
        );

        //yield farm balances checks
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tn1_yield_farm_account),
            bsx_tkn1_yield_farm_bsx_balance
        );
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_yield_farm_account),
            bsx_tkn2_yield_farm_bsx_balance - unclaimable_rewards
        );

        //global farm balance checks
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &global_farm_account),
            global_farm_bsx_balance + unclaimable_rewards
        );

        assert_eq!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[5]), None);

        // withdraw 2B
        let bsx_tkn2_bob_amm_shares_balance = Tokens::free_balance(BSX_TKN2_SHARE_ID, &BOB);
        let global_farm_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &global_farm_account);
        let bsx_tkn1_yield_farm_lp_shares_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tn1_yield_farm_account);

        let unclaimable_rewards = 5_911_539;
        let withdrawn_shares = 800;
        assert_eq!(
            LiquidityMining::withdraw_lp_shares(
                BOB,
                PREDEFINED_DEPOSIT_IDS[3],
                BSX_TKN2_YIELD_FARM_ID,
                unclaimable_rewards
            )
            .unwrap(),
            (GLOBAL_FARM_ID, BSX_TKN2_YIELD_FARM_ID, withdrawn_shares)
        );

        assert_eq!(
            LiquidityMining::global_farm(GC_FARM).unwrap(),
            GlobalFarmData {
                updated_at: 25,
                accumulated_rpz: 12,
                accumulated_rewards: 231_650,
                paid_accumulated_rewards: 1_164_400,
                total_shares_z: 0,
                ..PREDEFINED_GLOBAL_FARMS[2].clone()
            }
        );

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN2_AMM, GC_FARM, BSX_TKN2_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                updated_at: 25,
                accumulated_rpvs: 120,
                accumulated_rpz: 12,
                total_shares: 0,
                total_valued_shares: 0,
                entries_count: 0,
                ..PREDEFINED_YIELD_FARMS.with(|v| v[1].clone())
            },
        );

        //user balances checks
        assert_eq!(
            Tokens::free_balance(BSX_TKN2_SHARE_ID, &BOB),
            bsx_tkn2_bob_amm_shares_balance + withdrawn_shares
        );

        assert_eq!(Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH), 0);
        assert_eq!(Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH), 0);

        //yield farm balances checks
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tn1_yield_farm_account),
            bsx_tkn1_yield_farm_lp_shares_balance
        );

        //global farm balance checks
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &global_farm_account),
            global_farm_bsx_balance + 5_911_539 //5_911_539 unclaimable rewards after withdrawn
        );

        assert_eq!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[2]), None);
    });

    //charlie's farm inncetivize KSM and reward currency is ACA
    //This test check if correct currency is tranfered if rewards and incetvized
    //assts are different, otherwise farm behaviour is the same as in test above.
    predefined_test_ext().execute_with(|| {
        let aca_ksm_assets = AssetPair {
            asset_in: ACA,
            asset_out: KSM,
        };

        let aca_ksm_amm_account = AMM_POOLS.with(|v| v.borrow().get(&asset_pair_to_map_key(aca_ksm_assets)).unwrap().0);

        let ksm_balance_in_amm = 50;
        //this is done because amount of incetivized token in AMM is used in calculations.
        Tokens::set_balance(Origin::root(), aca_ksm_amm_account, KSM, ksm_balance_in_amm, 0).unwrap();
        Tokens::set_balance(Origin::root(), aca_ksm_amm_account, ACA, 20, 0).unwrap();

        set_block_number(1_800); //period 18

        let deposited_amount = 50;
        assert_ok!(LiquidityMining::deposit_lp_shares(
            ALICE,
            CHARLIE_FARM,
            ACA_KSM_YIELD_FARM_ID,
            ACA_KSM_AMM,
            deposited_amount,
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
                    yield_farm_id: ACA_KSM_YIELD_FARM_ID,
                    accumulated_rpvs: 0,
                    accumulated_claimed_rewards: 0,
                    entered_at: 18,
                    updated_at: 18,
                    valued_shares: 2_500
                }],
            },
        );

        set_block_number(2_596); //period 25

        let aca_ksm_alice_amm_shares_balance = Tokens::free_balance(ACA_KSM_SHARE_ID, &ALICE);

        assert_eq!(
            LiquidityMining::withdraw_lp_shares(ALICE, DEPOSIT_ID, ACA_KSM_YIELD_FARM_ID, 0).unwrap(),
            (CHARLIE_FARM, ACA_KSM_YIELD_FARM_ID, deposited_amount)
        );

        assert_eq!(
            Tokens::free_balance(ACA_KSM_SHARE_ID, &ALICE),
            aca_ksm_alice_amm_shares_balance + deposited_amount
        );
    });
}

#[test]
fn withdraw_shares_from_destroyed_farm_should_work() {
    //this is the case when yield farm was removed and global farm was destroyed. Only deposits stayed in
    //the storage. In this case only amm shares should be withdrawn

    predefined_test_ext_with_deposits().execute_with(|| {
        //cancel all yield farms in the farm
        assert_ok!(LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN1_AMM));
        assert_ok!(LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN2_AMM));

        //remove all yield farms from farm
        assert_ok!(LiquidityMining::destroy_yield_farm(
            GC,
            GC_FARM,
            BSX_TKN1_YIELD_FARM_ID,
            BSX_TKN1_AMM
        ));
        assert_ok!(LiquidityMining::destroy_yield_farm(
            GC,
            GC_FARM,
            BSX_TKN2_YIELD_FARM_ID,
            BSX_TKN2_AMM
        ));

        //destroy farm
        assert_ok!(LiquidityMining::destroy_global_farm(GC, GC_FARM));

        //check if farms was removed from storage
        assert!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, BSX_TKN1_YIELD_FARM_ID))
                .unwrap()
                .is_deleted()
        );
        assert!(
            LiquidityMining::yield_farm((BSX_TKN2_AMM, GC_FARM, BSX_TKN2_YIELD_FARM_ID))
                .unwrap()
                .is_deleted()
        );
        assert_eq!(
            LiquidityMining::global_farm(GC_FARM).unwrap().state,
            GlobalFarmState::Deleted
        );

        let test_data = vec![
            (
                ALICE,
                0,
                50,
                2_u64,
                BSX_TKN1_YIELD_FARM_ID,
                BSX_TKN1_SHARE_ID,
                BSX_TKN1_AMM,
            ),
            (
                BOB,
                1,
                80,
                1_u64,
                BSX_TKN1_YIELD_FARM_ID,
                BSX_TKN1_SHARE_ID,
                BSX_TKN1_AMM,
            ),
            (
                BOB,
                2,
                25,
                3_u64,
                BSX_TKN2_YIELD_FARM_ID,
                BSX_TKN2_SHARE_ID,
                BSX_TKN2_AMM,
            ),
            (
                BOB,
                3,
                800,
                2_u64,
                BSX_TKN2_YIELD_FARM_ID,
                BSX_TKN2_SHARE_ID,
                BSX_TKN2_AMM,
            ),
            (
                ALICE,
                4,
                87,
                1_u64,
                BSX_TKN2_YIELD_FARM_ID,
                BSX_TKN2_SHARE_ID,
                BSX_TKN2_AMM,
            ),
            (
                ALICE,
                5,
                48,
                0_u64,
                BSX_TKN2_YIELD_FARM_ID,
                BSX_TKN2_SHARE_ID,
                BSX_TKN2_AMM,
            ),
            (
                ALICE,
                6,
                486,
                0_u64,
                BSX_TKN1_YIELD_FARM_ID,
                BSX_TKN1_SHARE_ID,
                BSX_TKN1_AMM,
            ),
        ];

        for (caller, deposit_idx, withdrawn_shares, farm_entries_left, yield_farm_id, _lp_token, _amm_pool_id) in
            test_data
        {
            let bsx_tkn1_pallet_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH);
            let bsx_tkn2_pallet_amm_shares_balance = Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH);
            let bsx_tkn1_caller_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &caller);
            let bsx_tkn2_caller_shares_balance = Tokens::free_balance(BSX_TKN2_SHARE_ID, &caller);

            assert_eq!(
                LiquidityMining::withdraw_lp_shares(caller, PREDEFINED_DEPOSIT_IDS[deposit_idx], yield_farm_id, 0,)
                    .unwrap(),
                (GC_FARM, yield_farm_id, withdrawn_shares)
            );

            //TODO: ckeck farms state  e.g. entries count

            let mut bsx_tkn1_shares_withdrawn = 0;
            let mut bsx_tkn2_shares_withdrawn = 0;

            if yield_farm_id == BSX_TKN1_YIELD_FARM_ID {
                bsx_tkn1_shares_withdrawn = withdrawn_shares;
            } else {
                bsx_tkn2_shares_withdrawn = withdrawn_shares;
            }

            //check farm account shares balance
            assert_eq!(
                Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH),
                bsx_tkn1_pallet_amm_shares_balance - bsx_tkn1_shares_withdrawn
            );
            assert_eq!(
                Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH),
                bsx_tkn2_pallet_amm_shares_balance - bsx_tkn2_shares_withdrawn
            );

            //check user balances
            assert_eq!(
                Tokens::free_balance(BSX_TKN1_SHARE_ID, &caller),
                bsx_tkn1_caller_amm_shares_balance + bsx_tkn1_shares_withdrawn
            );
            assert_eq!(
                Tokens::free_balance(BSX_TKN2_SHARE_ID, &caller),
                bsx_tkn2_caller_shares_balance + bsx_tkn2_shares_withdrawn
            );

            //check if deposit was removed
            assert_eq!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[deposit_idx]), None);
        }
    });
}

#[test]
fn withdraw_shares_from_canceled_yield_farm_should_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        set_block_number(10_000);

        // cancel yield farm before withdraw test
        assert_ok!(LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN1_AMM));

        let global_farm_account = LiquidityMining::farm_account_id(GC_FARM).unwrap();
        let yield_farm_account = LiquidityMining::farm_account_id(BSX_TKN1_YIELD_FARM_ID).unwrap();

        //1-th withdraw
        let yield_farm_bsx_balance = Tokens::free_balance(BSX, &yield_farm_account);
        let global_farm_bsx_balance = Tokens::free_balance(BSX, &global_farm_account);
        let bsx_tkn1_pallet_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH);
        let bsx_tkn1_alice_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE);

        let global_farm = LiquidityMining::global_farm(GC_FARM).unwrap();
        let yield_farm = LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, BSX_TKN1_YIELD_FARM_ID)).unwrap();

        let unclaimable_rewards = 168_270;
        let withdrawn_amount = 50;
        assert_eq!(
            LiquidityMining::withdraw_lp_shares(
                ALICE,
                PREDEFINED_DEPOSIT_IDS[0],
                BSX_TKN1_YIELD_FARM_ID,
                unclaimable_rewards
            )
            .unwrap(),
            (GC_FARM, BSX_TKN1_YIELD_FARM_ID, withdrawn_amount)
        );

        assert_eq!(LiquidityMining::global_farm(GC_FARM).unwrap(), global_farm);

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, BSX_TKN1_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                total_shares: yield_farm.total_shares - withdrawn_amount,
                total_valued_shares: yield_farm.total_valued_shares - 2500,
                entries_count: 2,
                ..yield_farm
            }
        );

        assert_eq!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[0]), None);

        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH),
            bsx_tkn1_pallet_amm_shares_balance - withdrawn_amount
        );

        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE),
            bsx_tkn1_alice_amm_shares_balance + withdrawn_amount
        );

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
        let bsx_tkn1_pallet_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH);
        let bsx_tkn1_alice_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE);

        let global_farm = LiquidityMining::global_farm(GC_FARM).unwrap();
        let yield_farm = LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, BSX_TKN1_YIELD_FARM_ID)).unwrap();

        let unclaimable_rewards = 2_055_086;
        let shares_amount = 486;
        let valued_shares_amount = 38_880;

        assert_eq!(
            LiquidityMining::withdraw_lp_shares(
                ALICE,
                PREDEFINED_DEPOSIT_IDS[6],
                BSX_TKN1_YIELD_FARM_ID,
                unclaimable_rewards
            )
            .unwrap(),
            (GC_FARM, BSX_TKN1_YIELD_FARM_ID, shares_amount)
        );

        assert_eq!(LiquidityMining::global_farm(GC_FARM).unwrap(), global_farm);

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, BSX_TKN1_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                total_shares: yield_farm.total_shares - shares_amount,
                total_valued_shares: yield_farm.total_valued_shares - valued_shares_amount,
                entries_count: 1,
                ..yield_farm
            }
        );

        assert_eq!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[6]), None);

        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH),
            bsx_tkn1_pallet_amm_shares_balance - shares_amount
        );

        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE),
            bsx_tkn1_alice_amm_shares_balance + shares_amount
        );

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
        let bsx_tkn1_pallet_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH);
        let bsx_tkn1_bob_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &BOB);

        let global_farm = LiquidityMining::global_farm(GC_FARM).unwrap();
        let yield_farm = LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, BSX_TKN1_YIELD_FARM_ID)).unwrap();

        let unclaimable_rewards = 228_572;
        let shares_amount = 80;

        assert_eq!(
            LiquidityMining::withdraw_lp_shares(
                BOB,
                PREDEFINED_DEPOSIT_IDS[1],
                BSX_TKN1_YIELD_FARM_ID,
                unclaimable_rewards
            )
            .unwrap(),
            (GC_FARM, BSX_TKN1_YIELD_FARM_ID, shares_amount)
        );

        assert_eq!(LiquidityMining::global_farm(GC_FARM).unwrap(), global_farm);

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, BSX_TKN1_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                total_shares: 0,
                total_valued_shares: 0,
                entries_count: 0,
                ..yield_farm
            }
        );

        assert_eq!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[1]), None);

        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH),
            bsx_tkn1_pallet_amm_shares_balance - shares_amount
        );

        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &BOB),
            bsx_tkn1_bob_amm_shares_balance + shares_amount
        );

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

        //cancel yield farm before removing
        assert_ok!(LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN1_AMM));

        //remove yield farm before test
        assert_ok!(LiquidityMining::destroy_yield_farm(
            GC,
            GC_FARM,
            BSX_TKN1_YIELD_FARM_ID,
            BSX_TKN1_AMM
        ));

        assert!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, BSX_TKN1_YIELD_FARM_ID))
                .unwrap()
                .is_deleted(),
        );

        let global_farm = LiquidityMining::global_farm(GC_FARM).unwrap();

        let global_farm_account = LiquidityMining::farm_account_id(GC_FARM).unwrap();
        let bsx_tkn1_pallet_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH);
        let global_farm_bsx_balance = Tokens::free_balance(BSX, &global_farm_account);
        let bsx_tkn1_alice_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE);
        let alice_bsx_balance = Tokens::free_balance(BSX, &ALICE);

        let yield_farm = LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, BSX_TKN1_YIELD_FARM_ID)).unwrap();
        let shares_amount = 50;
        //1-th withdraw
        assert_eq!(
            LiquidityMining::withdraw_lp_shares(ALICE, PREDEFINED_DEPOSIT_IDS[0], BSX_TKN1_YIELD_FARM_ID, 0).unwrap(),
            (GC_FARM, BSX_TKN1_YIELD_FARM_ID, shares_amount)
        );

        assert_eq!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[0]), None);

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, BSX_TKN1_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                total_shares: 566,
                total_valued_shares: 43_040,
                entries_count: 2,
                ..yield_farm
            }
        );

        assert_eq!(LiquidityMining::global_farm(GC_FARM).unwrap(), global_farm);

        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH),
            bsx_tkn1_pallet_amm_shares_balance - shares_amount
        );
        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE),
            bsx_tkn1_alice_amm_shares_balance + shares_amount
        );

        //removed yield farm don't pay rewards, only transfer amm shares
        assert_eq!(Tokens::free_balance(BSX, &ALICE), alice_bsx_balance);
        assert_eq!(Tokens::free_balance(BSX, &global_farm_account), global_farm_bsx_balance);

        //2-nd withdraw
        let bsx_tkn1_alice_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE);
        let alice_bsx_balance = Tokens::free_balance(BSX, &ALICE);
        let bsx_tkn1_pallet_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH);
        let shares_amount = 486;

        let yield_farm = LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, BSX_TKN1_YIELD_FARM_ID)).unwrap();
        assert_eq!(
            LiquidityMining::withdraw_lp_shares(ALICE, PREDEFINED_DEPOSIT_IDS[6], BSX_TKN1_YIELD_FARM_ID, 0,).unwrap(),
            (GC_FARM, BSX_TKN1_YIELD_FARM_ID, shares_amount)
        );

        assert_eq!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[6]), None);

        assert_eq!(LiquidityMining::global_farm(GC_FARM).unwrap(), global_farm);

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, BSX_TKN1_YIELD_FARM_ID)).unwrap(),
            YieldFarmData {
                total_shares: 80,
                total_valued_shares: 4_160,
                entries_count: 1,
                ..yield_farm
            }
        );

        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH),
            bsx_tkn1_pallet_amm_shares_balance - shares_amount
        );
        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE),
            bsx_tkn1_alice_amm_shares_balance + shares_amount
        );

        //removed yield farm don't pay rewards, only return LP shares
        assert_eq!(Tokens::free_balance(BSX, &ALICE), alice_bsx_balance);
        assert_eq!(Tokens::free_balance(BSX, &global_farm_account), global_farm_bsx_balance);

        //3-th withdraw
        let bsx_tkn1_bob_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &BOB);
        let bob_bsx_balance = Tokens::free_balance(BSX, &BOB);
        let shares_amount = 80;

        assert_eq!(
            LiquidityMining::withdraw_lp_shares(BOB, PREDEFINED_DEPOSIT_IDS[1], BSX_TKN1_YIELD_FARM_ID, 0).unwrap(),
            (GC_FARM, BSX_TKN1_YIELD_FARM_ID, shares_amount)
        );

        //last withdraw whould flush yield farm if it's deleted
        assert!(LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, BSX_TKN1_YIELD_FARM_ID)).is_none());

        assert_eq!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[1]), None);

        assert_eq!(
            LiquidityMining::global_farm(GC_FARM).unwrap(),
            GlobalFarmData {
                yield_farms_count: (1, 1), //this value changed because last deposit flushed deleted yield farm
                ..global_farm
            }
        );

        assert_eq!(Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH), 0);
        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &BOB),
            bsx_tkn1_bob_amm_shares_balance + shares_amount
        );

        //removed yield farm don't pay rewards, only return LP shares
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
            LiquidityMining::withdraw_lp_shares(ALICE, DEPOSIT_ID, NOT_FOUND_ENTRY_ID, 0),
            Error::<Test>::YieldFarmEntryNotFound
        );
    });
}

#[test]
fn withdraw_shares_deposit_not_found_should_not_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        assert_noop!(
            LiquidityMining::withdraw_lp_shares(ALICE, 72_334_321_125_861_359_621, BSX_TKN1_YIELD_FARM_ID, 0),
            Error::<Test>::DepositNotFound
        );
    });
}
