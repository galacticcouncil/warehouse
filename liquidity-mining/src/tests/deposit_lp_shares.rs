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
fn deposit_lp_shares_should_work() {
    //NOTE: farm incentivize BSX token
    predefined_test_ext().execute_with(|| {
        let global_farm_id = GC_FARM;
        let bsx_tkn1_assets = AssetPair {
            asset_in: BSX,
            asset_out: TKN1,
        };

        let bsx_tkn2_assets = AssetPair {
            asset_in: BSX,
            asset_out: TKN2,
        };

        let global_farm_account = LiquidityMining::farm_account_id(global_farm_id).unwrap();
        let bsx_tnk1_yield_farm_account = LiquidityMining::farm_account_id(GC_BSX_TKN1_YIELD_FARM_ID).unwrap();
        let bsr_tkn2_yield_farm_account = LiquidityMining::farm_account_id(GC_BSX_TKN2_YIELD_FARM_ID).unwrap();
        let bsx_tkn1_amm_account =
            AMM_POOLS.with(|v| v.borrow().get(&asset_pair_to_map_key(bsx_tkn1_assets)).unwrap().0);
        let bsx_tkn2_amm_account =
            AMM_POOLS.with(|v| v.borrow().get(&asset_pair_to_map_key(bsx_tkn2_assets)).unwrap().0);
        //DEPOSIT 1:
        set_block_number(1_800); //18-th period

        let bsx_tkn1_alice_shares = Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE);

        //this is done because amount of incetivized token in AMM is used in calculations.
        Tokens::set_balance(Origin::root(), bsx_tkn1_amm_account, BSX, 50, 0).unwrap();

        let deposited_amount = 50;
        let yield_farm_id = GC_BSX_TKN1_YIELD_FARM_ID;
        assert_eq!(
            LiquidityMining::deposit_lp_shares(ALICE, global_farm_id, yield_farm_id, BSX_TKN1_AMM, deposited_amount)
                .unwrap(),
            PREDEFINED_DEPOSIT_IDS[0]
        );

        assert_eq!(
            LiquidityMining::global_farm(GC_FARM).unwrap(),
            GlobalFarmData {
                total_shares_z: 12_500,
                ..PREDEFINED_GLOBAL_FARMS[2].clone()
            }
        );

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, global_farm_id, yield_farm_id)).unwrap(),
            YieldFarmData {
                total_shares: deposited_amount,
                total_valued_shares: 2_500,
                entries_count: 1,
                ..PREDEFINED_YIELD_FARMS.with(|v| v[0].clone())
            },
        );

        assert_eq!(
            LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[0]).unwrap(),
            DepositData {
                shares: deposited_amount,
                amm_pool_id: BSX_TKN1_AMM,
                yield_farm_entries: vec![YieldFarmEntry::new(
                    global_farm_id,
                    GC_BSX_TKN1_YIELD_FARM_ID,
                    2_500,
                    0,
                    18
                )],
            },
        );

        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE),
            bsx_tkn1_alice_shares - deposited_amount
        );

        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH),
            deposited_amount
        );

        // DEPOSIT 2 (deposit in the same period):
        let bsx_tkn1_bob_shares = Tokens::free_balance(BSX_TKN1_SHARE_ID, &BOB);

        //This is done because amount of incetivized token in AMM is used in calculations.
        Tokens::set_balance(Origin::root(), bsx_tkn1_amm_account, BSX, 52, 0).unwrap();

        let deposited_amount = 80;
        let yield_farm_id = GC_BSX_TKN1_YIELD_FARM_ID;
        assert_eq!(
            LiquidityMining::deposit_lp_shares(BOB, global_farm_id, yield_farm_id, BSX_TKN1_AMM, deposited_amount)
                .unwrap(),
            PREDEFINED_DEPOSIT_IDS[1]
        );

        assert_eq!(
            LiquidityMining::global_farm(global_farm_id).unwrap(),
            GlobalFarmData {
                accumulated_rpz: 9,
                updated_at: 18,
                paid_accumulated_rewards: 112_500,
                total_shares_z: 33_300,
                ..PREDEFINED_GLOBAL_FARMS[2].clone()
            }
        );

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, global_farm_id, yield_farm_id)).unwrap(),
            YieldFarmData {
                updated_at: 18,
                accumulated_rpvs: 45,
                accumulated_rpz: 9,
                total_shares: 130,
                total_valued_shares: 6_660,
                entries_count: 2,
                ..PREDEFINED_YIELD_FARMS.with(|v| v[0].clone())
            },
        );

        assert_eq!(
            LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[1]).unwrap(),
            DepositData {
                shares: deposited_amount,
                amm_pool_id: BSX_TKN1_AMM,
                yield_farm_entries: vec![YieldFarmEntry::new(
                    global_farm_id,
                    GC_BSX_TKN1_YIELD_FARM_ID,
                    4_160,
                    45,
                    18
                )],
            },
        );

        //check if shares was transfered from deposit owner
        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &BOB),
            bsx_tkn1_bob_shares - deposited_amount
        );
        assert_eq!(Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH), 130); //130 - sum of all deposited shares until now

        assert_eq!(
            Tokens::free_balance(BSX, &global_farm_account),
            (30_000_000_000 - 112_500) //total_rewards - sum(claimed rewards by all liq. pools until now)
        );

        //check if claim from global pool was transfered to liq. pool account
        assert_eq!(Tokens::free_balance(BSX, &bsx_tnk1_yield_farm_account), 112_500);

        // DEPOSIT 3 (same period, second liq pool yield farm):
        let bsx_tkn2_bob_shares = Tokens::free_balance(BSX_TKN2_SHARE_ID, &BOB);

        //this is done because amount of incetivized token in AMM is used in calculations.
        Tokens::set_balance(Origin::root(), bsx_tkn2_amm_account, BSX, 8, 0).unwrap();

        let deposited_amount = 25;
        let yield_farm_id = GC_BSX_TKN2_YIELD_FARM_ID;
        assert_eq!(
            LiquidityMining::deposit_lp_shares(BOB, global_farm_id, yield_farm_id, BSX_TKN2_AMM, deposited_amount)
                .unwrap(),
            PREDEFINED_DEPOSIT_IDS[2]
        );

        assert_eq!(
            LiquidityMining::global_farm(global_farm_id).unwrap(),
            GlobalFarmData {
                updated_at: 18,
                max_reward_per_period: 60_000_000,
                accumulated_rpz: 9,
                paid_accumulated_rewards: 112_500,
                total_shares_z: 35_300,
                ..PREDEFINED_GLOBAL_FARMS[2].clone()
            }
        );

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN2_AMM, GC_FARM, yield_farm_id)).unwrap(),
            YieldFarmData {
                updated_at: 0,
                total_shares: 25,
                total_valued_shares: 200,
                entries_count: 1,
                ..PREDEFINED_YIELD_FARMS.with(|v| v[1].clone())
            },
        );

        assert_eq!(
            LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[2]).unwrap(),
            DepositData {
                shares: deposited_amount,
                amm_pool_id: BSX_TKN2_AMM,
                yield_farm_entries: vec![YieldFarmEntry::new(
                    global_farm_id,
                    GC_BSX_TKN2_YIELD_FARM_ID,
                    200,
                    0,
                    18
                )],
            },
        );

        //check if shares was transfered from deposit owner
        assert_eq!(
            Tokens::free_balance(BSX_TKN2_SHARE_ID, &BOB),
            bsx_tkn2_bob_shares - deposited_amount
        );
        assert_eq!(Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH), 25); //25 - sum of all deposited shares until now

        //pool wasn't updated in this period so no claim from global pool
        assert_eq!(
            Tokens::free_balance(BSX, &global_farm_account),
            (30_000_000_000 - 112_500) //total_rewards - claimed rewards by liq. pool
        );

        // no claim happed for this pool so this is same as after previous deposit
        assert_eq!(Tokens::free_balance(BSX, &bsx_tnk1_yield_farm_account), 112_500);
        //check if claim from global pool was transfered to liq. pool account
        //(there was no clai for this pool)
        assert_eq!(Tokens::free_balance(BSX, &bsr_tkn2_yield_farm_account), 0);

        // DEPOSIT 4 (new period):
        set_block_number(2051); //period 20
        let bsx_tkn2_bob_shares = Tokens::free_balance(BSX_TKN2_SHARE_ID, &BOB);

        //this is done because amount of incetivized token in AMM is used in calculations.
        Tokens::set_balance(Origin::root(), bsx_tkn2_amm_account, BSX, 58, 0).unwrap();

        let deposited_amount = 800;
        let yield_farm_id = GC_BSX_TKN2_YIELD_FARM_ID;
        assert_eq!(
            LiquidityMining::deposit_lp_shares(BOB, global_farm_id, yield_farm_id, BSX_TKN2_AMM, deposited_amount)
                .unwrap(),
            PREDEFINED_DEPOSIT_IDS[3]
        );

        assert_eq!(
            LiquidityMining::global_farm(global_farm_id).unwrap(),
            GlobalFarmData {
                updated_at: 20,
                accumulated_rpz: 10,
                paid_accumulated_rewards: 132_500,
                total_shares_z: 499_300,
                accumulated_rewards: 15_300,
                ..PREDEFINED_GLOBAL_FARMS[2].clone()
            }
        );

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN2_AMM, global_farm_id, yield_farm_id)).unwrap(),
            YieldFarmData {
                updated_at: 20,
                accumulated_rpvs: 100,
                accumulated_rpz: 10,
                total_shares: 825,
                total_valued_shares: 46_600,
                entries_count: 2,
                ..PREDEFINED_YIELD_FARMS.with(|v| v[1].clone())
            },
        );

        assert_eq!(
            LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[3]).unwrap(),
            DepositData {
                shares: deposited_amount,
                amm_pool_id: BSX_TKN2_AMM,
                yield_farm_entries: vec![YieldFarmEntry::new(
                    global_farm_id,
                    GC_BSX_TKN2_YIELD_FARM_ID,
                    46_400,
                    100,
                    20
                )],
            },
        );

        //check if shares was transfered from deposit owner
        assert_eq!(
            Tokens::free_balance(BSX_TKN2_SHARE_ID, &BOB),
            bsx_tkn2_bob_shares - deposited_amount
        );
        assert_eq!(Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH), 825); //825 - sum of all deposited shares until now

        assert_eq!(
            Tokens::free_balance(BSX, &global_farm_account),
            (30_000_000_000 - 132_500) //total_rewards - sum(claimed rewards by all liq. pools until now)
        );

        //check if claim from global pool was transfered to liq. pool account
        assert_eq!(Tokens::free_balance(BSX, &bsx_tnk1_yield_farm_account), 112_500);
        //check if claim from global pool was transfered to liq. pool account
        assert_eq!(Tokens::free_balance(BSX, &bsr_tkn2_yield_farm_account), 20_000);

        // DEPOSIT 5 (same period, second liq pool yield farm):
        set_block_number(2_586); //period 20
        let bsx_tkn2_alice_shares = Tokens::free_balance(BSX_TKN2_SHARE_ID, &ALICE);

        //this is done because amount of incetivized token in AMM is used in calculations.
        Tokens::set_balance(Origin::root(), bsx_tkn2_amm_account, BSX, 3, 0).unwrap();

        let deposited_amount = 87;
        let yield_farm_id = GC_BSX_TKN2_YIELD_FARM_ID;
        assert_eq!(
            LiquidityMining::deposit_lp_shares(ALICE, global_farm_id, yield_farm_id, BSX_TKN2_AMM, deposited_amount)
                .unwrap(),
            PREDEFINED_DEPOSIT_IDS[4]
        );

        assert_eq!(
            LiquidityMining::global_farm(global_farm_id).unwrap(),
            GlobalFarmData {
                updated_at: 25,
                accumulated_rpz: 12,
                total_shares_z: 501_910,
                accumulated_rewards: 331_550,
                paid_accumulated_rewards: 1_064_500,
                ..PREDEFINED_GLOBAL_FARMS[2].clone()
            }
        );

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN2_AMM, global_farm_id, yield_farm_id)).unwrap(),
            YieldFarmData {
                updated_at: 25,
                accumulated_rpvs: 120,
                accumulated_rpz: 12,
                total_shares: 912,
                total_valued_shares: 46_861,
                entries_count: 3,
                ..PREDEFINED_YIELD_FARMS.with(|v| v[1].clone())
            },
        );

        assert_eq!(
            LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[4]).unwrap(),
            DepositData {
                shares: deposited_amount,
                amm_pool_id: BSX_TKN2_AMM,
                yield_farm_entries: vec![YieldFarmEntry::new(
                    global_farm_id,
                    GC_BSX_TKN2_YIELD_FARM_ID,
                    261,
                    120,
                    25
                )],
            },
        );

        //check if shares was transfered from deposit owner
        assert_eq!(
            Tokens::free_balance(BSX_TKN2_SHARE_ID, &ALICE),
            bsx_tkn2_alice_shares - 87
        );
        assert_eq!(Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH), 912); //912 - sum of all deposited shares until now

        assert_eq!(
            Tokens::free_balance(BSX, &global_farm_account),
            (30_000_000_000 - 1_064_500) //total_rewards - sum(claimed rewards by all liq. pools until now)
        );

        //check if claim from global pool was transfered to liq. pool account
        assert_eq!(Tokens::free_balance(BSX, &bsx_tnk1_yield_farm_account), 112_500); //total_rewards - sum(claimed rewards by all liq. pools until now)
        assert_eq!(Tokens::free_balance(BSX, &bsr_tkn2_yield_farm_account), 952_000); //total_rewards - sum(claimed rewards by all liq. pools until now)

        // DEPOSIT 6 (same period):
        set_block_number(2_596); //period 20
        let bsx_tkn2_alice_shares = Tokens::free_balance(BSX_TKN2_SHARE_ID, &ALICE);

        //this is done because amount of incetivized token in AMM is used in calculations.
        Tokens::set_balance(Origin::root(), bsx_tkn2_amm_account, BSX, 16, 0).unwrap();

        let deposited_amount = 48;
        let yield_farm_id = GC_BSX_TKN2_YIELD_FARM_ID;
        assert_eq!(
            LiquidityMining::deposit_lp_shares(ALICE, global_farm_id, yield_farm_id, BSX_TKN2_AMM, deposited_amount)
                .unwrap(),
            PREDEFINED_DEPOSIT_IDS[5]
        );

        assert_eq!(
            LiquidityMining::global_farm(global_farm_id).unwrap(),
            GlobalFarmData {
                updated_at: 25,
                accumulated_rpz: 12,
                total_shares_z: 509_590,
                accumulated_rewards: 331_550,
                paid_accumulated_rewards: 1_064_500,
                ..PREDEFINED_GLOBAL_FARMS[2].clone()
            }
        );

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN2_AMM, global_farm_id, yield_farm_id)).unwrap(),
            YieldFarmData {
                updated_at: 25,
                accumulated_rpvs: 120,
                accumulated_rpz: 12,
                total_shares: 960,
                total_valued_shares: 47_629,
                entries_count: 4,
                ..PREDEFINED_YIELD_FARMS.with(|v| v[1].clone())
            },
        );

        assert_eq!(
            LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[5]).unwrap(),
            DepositData {
                shares: deposited_amount,
                amm_pool_id: BSX_TKN2_AMM,
                yield_farm_entries: vec![YieldFarmEntry::new(
                    global_farm_id,
                    GC_BSX_TKN2_YIELD_FARM_ID,
                    768,
                    120,
                    25
                )],
            },
        );

        //check if shares was transfered from deposit owner
        assert_eq!(
            Tokens::free_balance(BSX_TKN2_SHARE_ID, &ALICE),
            bsx_tkn2_alice_shares - deposited_amount
        );
        assert_eq!(Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH), 960); //960 - sum of all deposited shares until now

        assert_eq!(
            Tokens::free_balance(BSX, &global_farm_account),
            (30_000_000_000 - 1_064_500) //total_rewards - sum(claimed rewards by all liq. pools until now)
        );

        assert_eq!(Tokens::free_balance(BSX, &bsx_tnk1_yield_farm_account), 112_500); //total_rewards - sum(claimed rewards by all liq. pools until now)
        assert_eq!(Tokens::free_balance(BSX, &bsr_tkn2_yield_farm_account), 952_000); //total_rewards - sum(claimed rewards by all liq. pools until now)

        // DEPOSIT 7 : (same period differen liq poll farm)
        set_block_number(2_596); //period 20
        let bsx_tkn1_alice_shares = Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE);

        //this is done because amount of incetivized token in AMM is used in calculations.
        Tokens::set_balance(Origin::root(), bsx_tkn1_amm_account, BSX, 80, 0).unwrap();

        let deposited_amount = 486;
        let yield_farm_id = GC_BSX_TKN1_YIELD_FARM_ID;
        assert_eq!(
            LiquidityMining::deposit_lp_shares(ALICE, global_farm_id, yield_farm_id, BSX_TKN1_AMM, deposited_amount)
                .unwrap(),
            PREDEFINED_DEPOSIT_IDS[6]
        );

        assert_eq!(
            LiquidityMining::global_farm(global_farm_id).unwrap(),
            GlobalFarmData {
                updated_at: 25,
                accumulated_rpz: 12,
                total_shares_z: 703_990,
                accumulated_rewards: 231_650,
                paid_accumulated_rewards: 1_164_400,
                ..PREDEFINED_GLOBAL_FARMS[2].clone()
            }
        );

        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, global_farm_id, yield_farm_id)).unwrap(),
            YieldFarmData {
                updated_at: 25,
                accumulated_rpvs: 60,
                accumulated_rpz: 12,
                total_shares: 616,
                total_valued_shares: 45_540,
                entries_count: 3,
                ..PREDEFINED_YIELD_FARMS.with(|v| v[0].clone())
            },
        );

        assert_eq!(
            LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[6]).unwrap(),
            DepositData {
                shares: deposited_amount,
                amm_pool_id: BSX_TKN1_AMM,
                yield_farm_entries: vec![YieldFarmEntry::new(
                    global_farm_id,
                    GC_BSX_TKN1_YIELD_FARM_ID,
                    38_880,
                    60,
                    25
                )],
            },
        );

        //check if shares was transfered from deposit owner
        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE),
            bsx_tkn1_alice_shares - deposited_amount
        );
        assert_eq!(Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH), 616); //616 - sum of all deposited shares until now

        assert_eq!(
            Tokens::free_balance(BSX, &global_farm_account),
            (30_000_000_000 - 1_164_400) //total_rewards - sum(claimed rewards by all liq. pools until now)
        );

        //check if claim from global pool was transfered to liq. pool account
        assert_eq!(Tokens::free_balance(BSX, &bsx_tnk1_yield_farm_account), 212_400); //total_rewards - sum(claimed rewards by all liq. pools until now)
        assert_eq!(Tokens::free_balance(BSX, &bsr_tkn2_yield_farm_account), 952_000);
        //total_rewards - sum(claimed rewards by all liq. pools until now)
    });

    //deposit to farm with different incentivized_asset and reward_currency
    //charlie's farm inncetivize KSM and reward currency is ACA
    //This test only check if valued shares are correctly calculated if reward and incentivized
    //assts are different, otherwise pool behaviour is same as in test above.
    predefined_test_ext().execute_with(|| {
        let aca_ksm_assets = AssetPair {
            asset_in: ACA,
            asset_out: KSM,
        };

        let aca_ksm_amm_account = AMM_POOLS.with(|v| v.borrow().get(&asset_pair_to_map_key(aca_ksm_assets)).unwrap().0);
        let ksm_balance_in_amm = 16;

        //this is done because amount of incetivized token in AMM is used in calculations.
        Tokens::set_balance(Origin::root(), aca_ksm_amm_account, KSM, ksm_balance_in_amm, 0).unwrap();
        Tokens::set_balance(Origin::root(), aca_ksm_amm_account, ACA, 20, 0).unwrap();

        set_block_number(2_596); //period 25

        let deposited_amount = 1_000_000;
        let deposit_id = 1; //1 - because new test ext
        let yield_farm_id = CHARLIE_ACA_KSM_YIELD_FARM_ID;
        assert_eq!(
            LiquidityMining::deposit_lp_shares(ALICE, CHARLIE_FARM, yield_farm_id, ACA_KSM_AMM, deposited_amount)
                .unwrap(),
            deposit_id
        );

        assert_eq!(
            LiquidityMining::deposit(deposit_id).unwrap(),
            DepositData {
                shares: deposited_amount,
                amm_pool_id: ACA_KSM_AMM,
                yield_farm_entries: vec![YieldFarmEntry::new(
                    CHARLIE_FARM,
                    CHARLIE_ACA_KSM_YIELD_FARM_ID,
                    deposited_amount * ksm_balance_in_amm,
                    0,
                    25
                )],
            },
        );
    });
}

#[test]
fn deposit_lp_shares_bellow_min_deposit_should_not_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        //NOTE: min. deposit is 10
        let yield_farm_id = GC_BSX_TKN1_YIELD_FARM_ID;

        assert_noop!(
            LiquidityMining::deposit_lp_shares(ALICE, GC_FARM, yield_farm_id, BSX_TKN1_AMM, 0),
            Error::<Test>::InvalidDepositAmount
        );

        assert_noop!(
            LiquidityMining::deposit_lp_shares(ALICE, GC_FARM, yield_farm_id, BSX_TKN1_AMM, 1),
            Error::<Test>::InvalidDepositAmount
        );

        assert_noop!(
            LiquidityMining::deposit_lp_shares(ALICE, GC_FARM, yield_farm_id, BSX_TKN1_AMM, 8),
            Error::<Test>::InvalidDepositAmount
        );

        //margin value should works
        assert_ok!(LiquidityMining::deposit_lp_shares(
            ALICE,
            GC_FARM,
            yield_farm_id,
            BSX_TKN1_AMM,
            10
        ));
    });
}

#[test]
fn deposit_lp_shares_non_existing_yield_farm_should_not_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        assert_noop!(
            LiquidityMining::deposit_lp_shares(ALICE, GC_FARM, BSX_DOT_YIELD_FARM_ID, BSX_DOT_AMM, 10_000),
            Error::<Test>::YieldFarmNotFound
        );
    });
}

#[test]
fn deposit_lp_shares_stop_yield_farm_should_not_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        assert_ok!(LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN1_AMM));

        assert_noop!(
            LiquidityMining::deposit_lp_shares(ALICE, GC_FARM, GC_BSX_TKN1_YIELD_FARM_ID, BSX_TKN1_AMM, 10_000),
            Error::<Test>::LiquidityMiningIsNotActive
        );
    });
}
