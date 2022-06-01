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
fn create_yield_farm_should_work() {
    //Note: global_farm.updated_at isn't changed because global farm is empty (no yield farm stake in global farm)
    let test_data = vec![
        (
            AssetPair {
                asset_in: BSX,
                asset_out: ACA,
            },
            YieldFarmData {
                id: 8,
                updated_at: 17,
                total_shares: 0,
                total_valued_shares: 0,
                accumulated_rpvs: 0,
                accumulated_rpz: 0,
                multiplier: FixedU128::from(20_000_u128),
                loyalty_curve: Some(LoyaltyCurve::default()),
                canceled: false,
            },
            BSX_ACA_AMM,
            ALICE,
            ALICE_FARM,
            17_850,
            GlobalFarmData {
                yield_farms_count: 1,
                ..PREDEFINED_GLOBAL_FARMS[0].clone()
            },
        ),
        (
            AssetPair {
                asset_in: KSM,
                asset_out: BSX,
            },
            YieldFarmData {
                id: 9,
                updated_at: 17,
                total_shares: 0,
                total_valued_shares: 0,
                accumulated_rpvs: 0,
                accumulated_rpz: 0,
                multiplier: FixedU128::from(10_000_u128),
                loyalty_curve: None,
                canceled: false,
            },
            BSX_KSM_AMM,
            ALICE,
            ALICE_FARM,
            17_850,
            GlobalFarmData {
                yield_farms_count: 2,
                ..PREDEFINED_GLOBAL_FARMS[0].clone()
            },
        ),
        (
            AssetPair {
                asset_in: BSX,
                asset_out: ETH,
            },
            YieldFarmData {
                id: 10,
                updated_at: 20,
                total_shares: 0,
                total_valued_shares: 0,
                accumulated_rpvs: 0,
                accumulated_rpz: 0,
                multiplier: FixedU128::from(10_000_u128),
                loyalty_curve: Some(LoyaltyCurve {
                    initial_reward_percentage: FixedU128::from_inner(100_000_000_000_000_000),
                    scale_coef: 50,
                }),
                canceled: false,
            },
            BSX_ETH_AMM,
            ALICE,
            ALICE_FARM,
            20_000,
            GlobalFarmData {
                yield_farms_count: 3,
                ..PREDEFINED_GLOBAL_FARMS[0].clone()
            },
        ),
        (
            AssetPair {
                asset_in: BSX,
                asset_out: ETH,
            },
            YieldFarmData {
                id: 11,
                updated_at: 2,
                total_shares: 0,
                total_valued_shares: 0,
                accumulated_rpvs: 0,
                accumulated_rpz: 0,
                multiplier: FixedU128::from(50_000_128),
                loyalty_curve: Some(LoyaltyCurve {
                    initial_reward_percentage: FixedU128::from_inner(1),
                    scale_coef: 0,
                }),
                canceled: false,
            },
            BSX_ETH_AMM,
            BOB,
            BOB_FARM,
            20_000,
            GlobalFarmData {
                yield_farms_count: 1,
                ..PREDEFINED_GLOBAL_FARMS[1].clone()
            },
        ),
    ];

    predefined_test_ext().execute_with(|| {
        for (assets, yield_farm, amm_id, who, global_farm_id, now, global_farm) in test_data.clone() {
            set_block_number(now);

            assert_eq!(LiquidityMining::create_yield_farm(
                who,
                global_farm_id,
                yield_farm.multiplier,
                yield_farm.loyalty_curve.clone(),
                amm_id,
                assets.asset_in,
                assets.asset_out,
            ).unwrap(), yield_farm.id);

            assert_eq!(LiquidityMining::global_farm(global_farm_id).unwrap(), global_farm);
        }

        const EXPECTED_FARM_ENTRIES_COUNT: u64 = 0;
        for (_, yield_farm, amm_id, _, global_farm_id, _, _) in test_data {
            assert_eq!(LiquidityMining::yield_farm(amm_id, global_farm_id).unwrap(), yield_farm);
            assert_eq!(
                LiquidityMining::yield_farm_metadata(yield_farm.id).unwrap(),
                EXPECTED_FARM_ENTRIES_COUNT
            );
        }
    });
}

#[test]
fn add_yield_farm_missing_incentivized_asset_should_not_work() {
    predefined_test_ext().execute_with(|| {
        assert_noop!(
            LiquidityMining::create_yield_farm(
                ALICE,
                ALICE_FARM,
                FixedU128::from(10_000_u128),
                None,
                KSM_DOT_AMM,
                //neither KSM nor DOT is incetivized in farm
                KSM,
                DOT,
            ),
            Error::<Test>::MissingIncentivizedAsset
        );
    });
}

#[test]
fn add_yield_farm_not_owner_should_not_work() {
    predefined_test_ext().execute_with(|| {
        assert_noop!(
            LiquidityMining::create_yield_farm(
                BOB,
                ALICE_FARM,
                FixedU128::from(10_000_u128),
                None,
                BSX_HDX_AMM,
                BSX,
                HDX,
            ),
            Error::<Test>::Forbidden
        );

        assert_noop!(
            LiquidityMining::create_yield_farm(
                BOB,
                ALICE_FARM,
                FixedU128::from(10_000_u128),
                Some(LoyaltyCurve::default()),
                BSX_HDX_AMM,
                BSX,
                HDX,
            ),
            Error::<Test>::Forbidden
        );
    });
}

#[test]
fn add_yield_farm_invalid_loyalty_curve_should_not_work() {
    predefined_test_ext().execute_with(|| {
        let curves = vec![
            Some(LoyaltyCurve {
                initial_reward_percentage: FixedU128::one(),
                scale_coef: 0,
            }),
            Some(LoyaltyCurve {
                initial_reward_percentage: FixedU128::from_float(1.0),
                scale_coef: 1_000_000,
            }),
            Some(LoyaltyCurve {
                initial_reward_percentage: FixedU128::from_float(1.000_000_000_000_000_001),
                scale_coef: 25_996_000,
            }),
            Some(LoyaltyCurve {
                initial_reward_percentage: FixedU128::from(1_u128),
                scale_coef: 25_996_000,
            }),
            Some(LoyaltyCurve {
                initial_reward_percentage: FixedU128::from(5_u128),
                scale_coef: 25_996_000,
            }),
            Some(LoyaltyCurve {
                initial_reward_percentage: FixedU128::from(16_874_354_654_u128),
                scale_coef: 25_996_000,
            }),
        ];

        for c in curves {
            assert_noop!(
                LiquidityMining::create_yield_farm(
                    ALICE,
                    ALICE_FARM,
                    FixedU128::from(10_000_u128),
                    c,
                    BSX_HDX_AMM,
                    BSX,
                    HDX,
                ),
                Error::<Test>::InvalidInitialRewardPercentage
            );
        }
    });
}

#[test]
fn add_yield_farm_invalid_multiplier_should_not_work() {
    predefined_test_ext().execute_with(|| {
        assert_noop!(
            LiquidityMining::create_yield_farm(
                ALICE,
                ALICE_FARM,
                FixedU128::from(0_u128),
                Some(LoyaltyCurve::default()),
                BSX_HDX_AMM,
                BSX,
                HDX,
            ),
            Error::<Test>::InvalidMultiplier
        );
    });
}

#[test]
fn add_yield_farm_add_duplicate_amm_should_not_work() {
    predefined_test_ext().execute_with(|| {
        set_block_number(20_000);

        let aca_ksm_assets = AssetPair {
            asset_in: ACA,
            asset_out: KSM,
        };

        let aca_ksm_amm_account = AMM_POOLS.with(|v| v.borrow().get(&asset_pair_to_map_key(aca_ksm_assets)).unwrap().0);

        //check if yeild farm for aca ksm assets pair exist
        assert!(LiquidityMining::yield_farm(aca_ksm_amm_account, CHARLIE_FARM).is_some());

        //try to add same amm second time in the same block(period)
        assert_noop!(
            LiquidityMining::create_yield_farm(
                CHARLIE,
                CHARLIE_FARM,
                FixedU128::from(9_000_u128),
                Some(LoyaltyCurve::default()),
                ACA_KSM_AMM,
                ACA,
                KSM,
            ),
            Error::<Test>::YieldFarmAlreadyExists
        );

        //try to add same amm second time in later block(period)
        set_block_number(30_000);

        assert_noop!(
            LiquidityMining::create_yield_farm(
                CHARLIE,
                CHARLIE_FARM,
                FixedU128::from(9_000_u128),
                Some(LoyaltyCurve::default()),
                ACA_KSM_AMM,
                ACA,
                KSM,
            ),
            Error::<Test>::YieldFarmAlreadyExists
        );
    });
}
