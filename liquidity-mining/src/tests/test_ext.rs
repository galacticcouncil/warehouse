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

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| set_block_number(1));
    ext
}

pub fn predefined_test_ext() -> sp_io::TestExternalities {
    let mut ext = new_test_ext();
    ext.execute_with(|| {
        assert_ok!(LiquidityMining::create_global_farm(
            100_000_000_000,
            PREDEFINED_GLOBAL_FARMS[0].planned_yielding_periods,
            PREDEFINED_GLOBAL_FARMS[0].blocks_per_period,
            PREDEFINED_GLOBAL_FARMS[0].incentivized_asset,
            PREDEFINED_GLOBAL_FARMS[0].reward_currency,
            ALICE,
            PREDEFINED_GLOBAL_FARMS[0].yield_per_period,
        ));

        assert_ok!(LiquidityMining::create_global_farm(
            1_000_000_000,
            PREDEFINED_GLOBAL_FARMS[1].planned_yielding_periods,
            PREDEFINED_GLOBAL_FARMS[1].blocks_per_period,
            PREDEFINED_GLOBAL_FARMS[1].incentivized_asset,
            PREDEFINED_GLOBAL_FARMS[1].reward_currency,
            BOB,
            PREDEFINED_GLOBAL_FARMS[1].yield_per_period,
        ));

        assert_ok!(LiquidityMining::create_global_farm(
            30_000_000_000,
            PREDEFINED_GLOBAL_FARMS[2].planned_yielding_periods,
            PREDEFINED_GLOBAL_FARMS[2].blocks_per_period,
            PREDEFINED_GLOBAL_FARMS[2].incentivized_asset,
            PREDEFINED_GLOBAL_FARMS[2].reward_currency,
            GC,
            PREDEFINED_GLOBAL_FARMS[2].yield_per_period,
        ));

        assert_ok!(LiquidityMining::create_global_farm(
            30_000_000_000,
            PREDEFINED_GLOBAL_FARMS[3].planned_yielding_periods,
            PREDEFINED_GLOBAL_FARMS[3].blocks_per_period,
            PREDEFINED_GLOBAL_FARMS[3].incentivized_asset,
            PREDEFINED_GLOBAL_FARMS[3].reward_currency,
            CHARLIE,
            PREDEFINED_GLOBAL_FARMS[3].yield_per_period,
        ));

        assert_ok!(LiquidityMining::create_global_farm(
            30_000_000_000,
            PREDEFINED_GLOBAL_FARMS[4].planned_yielding_periods,
            PREDEFINED_GLOBAL_FARMS[4].blocks_per_period,
            PREDEFINED_GLOBAL_FARMS[4].incentivized_asset,
            PREDEFINED_GLOBAL_FARMS[4].reward_currency,
            DAVE,
            PREDEFINED_GLOBAL_FARMS[4].yield_per_period,
        ));

        assert_ok!(LiquidityMining::create_global_farm(
            30_000_000_000,
            PREDEFINED_GLOBAL_FARMS[5].planned_yielding_periods,
            PREDEFINED_GLOBAL_FARMS[5].blocks_per_period,
            PREDEFINED_GLOBAL_FARMS[5].incentivized_asset,
            PREDEFINED_GLOBAL_FARMS[5].reward_currency,
            EVE,
            PREDEFINED_GLOBAL_FARMS[5].yield_per_period,
        ));

        let amm_mock_data = vec![
            (
                AssetPair {
                    asset_in: BSX,
                    asset_out: ACA,
                },
                (BSX_ACA_AMM, BSX_ACA_SHARE_ID),
            ),
            (
                AssetPair {
                    asset_in: KSM,
                    asset_out: BSX,
                },
                (BSX_KSM_AMM, BSX_KSM_SHARE_ID),
            ),
            (
                AssetPair {
                    asset_in: BSX,
                    asset_out: DOT,
                },
                (BSX_DOT_AMM, BSX_DOT_SHARE_ID),
            ),
            (
                AssetPair {
                    asset_in: BSX,
                    asset_out: ETH,
                },
                (BSX_ETH_AMM, BSX_ETH_SHARE_ID),
            ),
            (
                AssetPair {
                    asset_in: BSX,
                    asset_out: HDX,
                },
                (BSX_HDX_AMM, BSX_HDX_SHARE_ID),
            ),
            (
                AssetPair {
                    asset_in: BSX,
                    asset_out: TKN1,
                },
                (BSX_TKN1_AMM, BSX_TKN1_SHARE_ID),
            ),
            (
                AssetPair {
                    asset_in: BSX,
                    asset_out: TKN2,
                },
                (BSX_TKN2_AMM, BSX_TKN2_SHARE_ID),
            ),
            (
                AssetPair {
                    asset_in: KSM,
                    asset_out: DOT,
                },
                (KSM_DOT_AMM, KSM_DOT_SHARE_ID),
            ),
            (
                AssetPair {
                    asset_in: ACA,
                    asset_out: KSM,
                },
                (ACA_KSM_AMM, ACA_KSM_SHARE_ID),
            ),
        ];

        AMM_POOLS.with(|h| {
            let mut hm = h.borrow_mut();
            for (k, v) in amm_mock_data {
                hm.insert(asset_pair_to_map_key(k), v);
            }
        });

        let yield_farm = PREDEFINED_YIELD_FARMS.with(|v| v[0].clone());
        init_yield_farm(GC, GC_FARM, BSX_TKN1_AMM, BSX, TKN1, yield_farm);

        let yield_farm = PREDEFINED_YIELD_FARMS.with(|v| v[1].clone());
        init_yield_farm(GC, GC_FARM, BSX_TKN2_AMM, BSX, TKN2, yield_farm);

        let yield_farm = PREDEFINED_YIELD_FARMS.with(|v| v[2].clone());
        init_yield_farm(CHARLIE, CHARLIE_FARM, ACA_KSM_AMM, ACA, KSM, yield_farm);

        let yield_farm = PREDEFINED_YIELD_FARMS.with(|v| v[3].clone());
        init_yield_farm(DAVE, DAVE_FARM, BSX_TKN1_AMM, BSX, TKN1, yield_farm);

        let yield_farm = PREDEFINED_YIELD_FARMS.with(|v| v[4].clone());
        init_yield_farm(EVE, EVE_FARM, BSX_TKN1_AMM, BSX, TKN1, yield_farm);

        let yield_farm = PREDEFINED_YIELD_FARMS.with(|v| v[5].clone());
        init_yield_farm(EVE, EVE_FARM, BSX_TKN2_AMM, BSX, TKN2, yield_farm);

        reset_on_rpvs_update();
        reset_on_rpz_update();
    });

    ext
}

fn init_yield_farm(
    owner: AccountId,
    farm_id: GlobalFarmId,
    amm_id: AccountId,
    asset_a: AssetId,
    asset_b: AssetId,
    yield_farm: YieldFarmData<Test>,
) {
    assert_ok!(LiquidityMining::create_yield_farm(
        owner,
        farm_id,
        yield_farm.multiplier,
        yield_farm.loyalty_curve.clone(),
        amm_id,
        asset_a,
        asset_b,
    ));

    assert_eq!(
        LiquidityMining::yield_farm((amm_id, farm_id, yield_farm.id)).unwrap(),
        yield_farm
    );
}

pub fn predefined_test_ext_with_deposits() -> sp_io::TestExternalities {
    let mut ext = predefined_test_ext();

    ext.execute_with(|| {
        let farm_id = GC_FARM; //global pool

        let bsx_tkn1_assets = AssetPair {
            asset_in: BSX,
            asset_out: TKN1,
        };

        let bsx_tkn2_assets = AssetPair {
            asset_in: BSX,
            asset_out: TKN2,
        };

        let global_pool_account = LiquidityMining::farm_account_id(GC_FARM).unwrap();
        let bsx_tkn1_liq_pool_account = LiquidityMining::farm_account_id(GC_BSX_TKN1_YIELD_FARM_ID).unwrap();
        let bsx_tkn2_liq_pool_account = LiquidityMining::farm_account_id(GC_BSX_TKN2_YIELD_FARM_ID).unwrap();
        let bsx_tkn1_amm_account =
            AMM_POOLS.with(|v| v.borrow().get(&asset_pair_to_map_key(bsx_tkn1_assets)).unwrap().0);
        let bsx_tkn2_amm_account =
            AMM_POOLS.with(|v| v.borrow().get(&asset_pair_to_map_key(bsx_tkn2_assets)).unwrap().0);

        //DEPOSIT 1:
        set_block_number(1_800); //18-th period

        //this is done because amount of incetivized token in AMM is used in calculations.
        Tokens::set_balance(Origin::root(), bsx_tkn1_amm_account, BSX, 50, 0).unwrap();

        let deposited_amount = 50;
        assert_ok!(LiquidityMining::deposit_lp_shares(
            ALICE,
            farm_id,
            GC_BSX_TKN1_YIELD_FARM_ID,
            BSX_TKN1_AMM,
            deposited_amount,
        ));

        assert!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[0]).is_some());

        // DEPOSIT 2 (deposit in same period):

        //this is done because amount of incetivized token in AMM is used in calculations.
        Tokens::set_balance(Origin::root(), bsx_tkn1_amm_account, BSX, 52, 0).unwrap();

        let deposited_amount = 80;
        assert_eq!(
            LiquidityMining::deposit_lp_shares(BOB, farm_id, GC_BSX_TKN1_YIELD_FARM_ID, BSX_TKN1_AMM, deposited_amount,)
                .unwrap(),
            PREDEFINED_DEPOSIT_IDS[1]
        );

        assert!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[1]).is_some());

        // DEPOSIT 3 (same period, second liq pool yield farm):

        //this is done because amount of incetivized token in AMM is used in calculations.
        Tokens::set_balance(Origin::root(), bsx_tkn2_amm_account, BSX, 8, 0).unwrap();

        let deposited_amount = 25;
        assert_ok!(LiquidityMining::deposit_lp_shares(
            BOB,
            farm_id,
            GC_BSX_TKN2_YIELD_FARM_ID,
            BSX_TKN2_AMM,
            deposited_amount,
        ));

        assert!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[2]).is_some());

        // DEPOSIT 4 (new period):
        set_block_number(2051); //period 20

        //this is done because amount of incetivized token in AMM is used in calculations.
        Tokens::set_balance(Origin::root(), bsx_tkn2_amm_account, BSX, 58, 0).unwrap();

        let deposited_amount = 800;
        assert_ok!(LiquidityMining::deposit_lp_shares(
            BOB,
            farm_id,
            GC_BSX_TKN2_YIELD_FARM_ID,
            BSX_TKN2_AMM,
            deposited_amount,
        ));

        assert!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[3]).is_some());

        // DEPOSIT 5 (same period, second liq pool yield farm):
        set_block_number(2_586); //period 25

        //this is done because amount of incetivized token in AMM is used in calculations.
        Tokens::set_balance(Origin::root(), bsx_tkn2_amm_account, BSX, 3, 0).unwrap();

        let deposited_amount = 87;
        assert_ok!(LiquidityMining::deposit_lp_shares(
            ALICE,
            farm_id,
            GC_BSX_TKN2_YIELD_FARM_ID,
            BSX_TKN2_AMM,
            deposited_amount,
        ));

        assert!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[4]).is_some());

        // DEPOSIT 6 (same period):
        set_block_number(2_596); //period 25

        //this is done because amount of incetivized token in AMM is used in calculations.
        Tokens::set_balance(Origin::root(), bsx_tkn2_amm_account, BSX, 16, 0).unwrap();

        let deposited_amount = 48;
        assert_ok!(LiquidityMining::deposit_lp_shares(
            ALICE,
            farm_id,
            GC_BSX_TKN2_YIELD_FARM_ID,
            BSX_TKN2_AMM,
            deposited_amount,
        ));

        assert!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[5]).is_some());

        // DEPOSIT 7 : (same period differen liq poll farm)
        set_block_number(2_596); //period 25

        //this is done because amount of incetivized token in AMM is used in calculations.
        Tokens::set_balance(Origin::root(), bsx_tkn1_amm_account, BSX, 80, 0).unwrap();

        let deposited_amount = 486;
        assert_ok!(LiquidityMining::deposit_lp_shares(
            ALICE,
            farm_id,
            GC_BSX_TKN1_YIELD_FARM_ID,
            BSX_TKN1_AMM,
            deposited_amount,
        ));

        assert!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[6]).is_some());

        assert_eq!(
            LiquidityMining::global_farm(GC_FARM).unwrap(),
            GlobalFarmData {
                id: GC_FARM,
                updated_at: 25,
                reward_currency: BSX,
                yield_per_period: Permill::from_percent(50),
                planned_yielding_periods: 500_u64,
                blocks_per_period: 100_u64,
                owner: GC,
                incentivized_asset: BSX,
                max_reward_per_period: 60_000_000,
                accumulated_rpz: 12,
                yield_farms_count: (2, 2),
                total_shares_z: 703_990,
                accumulated_rewards: 231_650,
                paid_accumulated_rewards: 1_164_400,
                state: GlobalFarmState::Active,
            }
        );

        let yield_farm_id = PREDEFINED_YIELD_FARMS.with(|v| v[0].id);
        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN1_AMM, GC_FARM, yield_farm_id)).unwrap(),
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

        let yield_farm_id = PREDEFINED_YIELD_FARMS.with(|v| v[1].id);
        assert_eq!(
            LiquidityMining::yield_farm((BSX_TKN2_AMM, GC_FARM, yield_farm_id)).unwrap(),
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

        //reward currency balance check. total_rewards - sum(claimes from global pool)
        assert_eq!(
            Tokens::free_balance(BSX, &global_pool_account),
            (30_000_000_000 - 1_164_400)
        );

        //check of claimed amount from global pool (sum of all claims)
        assert_eq!(Tokens::free_balance(BSX, &bsx_tkn1_liq_pool_account), 212_400);
        assert_eq!(Tokens::free_balance(BSX, &bsx_tkn2_liq_pool_account), 952_000);

        //balance check after transfer amm shares
        assert_eq!(Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE), 3_000_000 - 536);
        assert_eq!(Tokens::free_balance(BSX_TKN2_SHARE_ID, &ALICE), 3_000_000 - 135);

        //balance check after transfer amm shares
        assert_eq!(Tokens::free_balance(BSX_TKN1_SHARE_ID, &BOB), 2_000_000 - 80);
        assert_eq!(Tokens::free_balance(BSX_TKN2_SHARE_ID, &BOB), 2_000_000 - 825);

        reset_on_rpvs_update();
        reset_on_rpz_update();
    });

    ext
}
