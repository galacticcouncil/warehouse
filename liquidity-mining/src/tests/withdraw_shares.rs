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

        //let pallet_account = LiquidityMining::account_id();
        let bsx_tkn1_liq_pool_account = LiquidityMining::pool_account_id(BSX_TKN1_LIQ_POOL_ID).unwrap();
        let bsx_tkn2_liq_pool_account = LiquidityMining::pool_account_id(BSX_TKN2_LIQ_POOL_ID).unwrap();
        let global_pool_account = LiquidityMining::pool_account_id(GC_FARM).unwrap();

        // This balance is used to transfer unclaimable_rewards from liq. pool to global pool.
        // Claimin is not part of withdraw_shares() so balance need to be set.
        Tokens::set_balance(Origin::root(), bsx_tkn1_liq_pool_account, BSX, 100_000_000_000, 0).unwrap();
        Tokens::set_balance(Origin::root(), bsx_tkn2_liq_pool_account, BSX, 100_000_000_000, 0).unwrap();

        // withdraw 1A
        let bsx_tkn1_alice_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE);
        let bsx_tkn1_pallet_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH);
        let bsx_tkn2_pallet_amm_shares_balance = Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH);
        let global_pool_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &global_pool_account);
        let bsx_tkn1_liq_pool_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn1_liq_pool_account);
        let bsx_tkn2_liq_pool_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_liq_pool_account);

        let unclaimable_rewards = 100_000;
        assert_ok!(LiquidityMining::withdraw_shares(
            ALICE,
            PREDEFINED_DEPOSIT_IDS[0],
            BSX_TKN1_AMM,
            unclaimable_rewards,
        ));

        assert_eq!(
            LiquidityMining::global_pool(GC_FARM).unwrap(),
            GlobalPool {
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
                liq_pools_count: 2,
                total_shares_z: 691_490,
                accumulated_rewards: 231_650,
                paid_accumulated_rewards: 1_164_400,
            }
        );

        assert_eq!(
            LiquidityMining::liquidity_pool(GC_FARM, BSX_TKN1_AMM).unwrap(),
            LiquidityPoolYieldFarm {
                id: BSX_TKN1_LIQ_POOL_ID,
                updated_at: 25,
                accumulated_rpvs: 60,
                accumulated_rpz: 12,
                total_shares: 566,
                total_valued_shares: 43_040,
                loyalty_curve: Some(LoyaltyCurve::default()),
                stake_in_global_pool: 215_200,
                multiplier: FixedU128::from(5_u128),
                canceled: false,
            },
        );

        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE),
            bsx_tkn1_alice_amm_shares_balance + 50
        );

        //stash shares account balances checks
        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH),
            bsx_tkn1_pallet_amm_shares_balance - 50
        );
        assert_eq!(
            Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH),
            bsx_tkn2_pallet_amm_shares_balance
        );

        //liq pool balance checks
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn1_liq_pool_account),
            bsx_tkn1_liq_pool_bsx_balance - unclaimable_rewards
        );
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_liq_pool_account),
            bsx_tkn2_liq_pool_bsx_balance
        );

        //global pool balance checks
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &global_pool_account),
            global_pool_bsx_balance + unclaimable_rewards
        );

        assert_eq!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[0]), None);

        assert_eq!(
            LiquidityMining::liq_pool_meta(BSX_TKN1_LIQ_POOL_ID).unwrap(),
            (2, GC_FARM)
        );

        set_block_number(12_800);

        // withdraw 3B
        let bsx_tkn2_alice_amm_shares_balance = Tokens::free_balance(BSX_TKN2_SHARE_ID, &ALICE);
        let bsx_tkn1_pallet_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH);
        let bsx_tkn2_pallet_amm_shares_balance = Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH);
        let global_pool_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &global_pool_account);
        let bsx_tkn1_liq_pool_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn1_liq_pool_account);
        let bsx_tkn2_liq_pool_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_liq_pool_account);

        let unclaimable_rewards = 32_786;
        assert_ok!(LiquidityMining::withdraw_shares(
            ALICE,
            PREDEFINED_DEPOSIT_IDS[4],
            BSX_TKN2_AMM,
            unclaimable_rewards
        ));

        assert_eq!(
            LiquidityMining::global_pool(GC_FARM).unwrap(),
            GlobalPool {
                updated_at: 25,
                accumulated_rpz: 12,
                accumulated_rewards: 231_650,
                paid_accumulated_rewards: 1_164_400,
                total_shares_z: 688_880,
                ..PREDEFINED_GLOBAL_POOLS[2]
            }
        );

        // this pool should not change
        assert_eq!(
            LiquidityMining::liquidity_pool(GC_FARM, BSX_TKN1_AMM).unwrap(),
            LiquidityPoolYieldFarm {
                id: BSX_TKN1_LIQ_POOL_ID,
                updated_at: 25,
                accumulated_rpvs: 60,
                accumulated_rpz: 12,
                total_shares: 566,
                total_valued_shares: 43_040,
                loyalty_curve: Some(LoyaltyCurve::default()),
                stake_in_global_pool: 215_200,
                multiplier: FixedU128::from(5_u128),
                canceled: false,
            },
        );

        assert_eq!(
            LiquidityMining::liquidity_pool(GC_FARM, BSX_TKN2_AMM).unwrap(),
            LiquidityPoolYieldFarm {
                id: BSX_TKN2_LIQ_POOL_ID,
                updated_at: 25,
                accumulated_rpvs: 120,
                accumulated_rpz: 12,
                total_shares: 873,
                total_valued_shares: 47_368,
                loyalty_curve: Some(LoyaltyCurve::default()),
                stake_in_global_pool: 473_680,
                multiplier: FixedU128::from(10_u128),
                canceled: false,
            },
        );

        //user balances checks
        assert_eq!(
            Tokens::free_balance(BSX_TKN2_SHARE_ID, &ALICE),
            bsx_tkn2_alice_amm_shares_balance + 87
        );

        //stash shares account balances checks
        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH),
            bsx_tkn1_pallet_amm_shares_balance
        );
        assert_eq!(
            Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH),
            bsx_tkn2_pallet_amm_shares_balance - 87
        );

        //liq pool balance checks
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn1_liq_pool_account),
            bsx_tkn1_liq_pool_bsx_balance
        );

        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_liq_pool_account),
            (bsx_tkn2_liq_pool_bsx_balance - unclaimable_rewards)
        );

        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &global_pool_account),
            global_pool_bsx_balance + unclaimable_rewards
        );

        assert_eq!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[4]), None);

        assert_eq!(
            LiquidityMining::liq_pool_meta(BSX_TKN2_LIQ_POOL_ID).unwrap(),
            (3, GC_FARM)
        );

        // withdraw 3A
        let bsx_tkn1_alice_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE);
        let bsx_tkn1_pallet_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH);
        let bsx_tkn2_pallet_amm_shares_balance = Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH);
        let global_pool_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &global_pool_account);
        let bsx_tkn1_liq_pool_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn1_liq_pool_account);
        let bsx_tkn2_liq_pool_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_liq_pool_account);

        let unclaimable_rewards = 2_441_971;
        assert_ok!(LiquidityMining::withdraw_shares(
            ALICE,
            PREDEFINED_DEPOSIT_IDS[6],
            BSX_TKN1_AMM,
            unclaimable_rewards,
        ));

        assert_eq!(
            LiquidityMining::global_pool(GC_FARM).unwrap(),
            GlobalPool {
                updated_at: 25,
                accumulated_rpz: 12,
                accumulated_rewards: 231_650,
                paid_accumulated_rewards: 1_164_400,
                total_shares_z: 494480,
                ..PREDEFINED_GLOBAL_POOLS[2]
            }
        );

        assert_eq!(
            LiquidityMining::liquidity_pool(GC_FARM, BSX_TKN1_AMM).unwrap(),
            LiquidityPoolYieldFarm {
                id: BSX_TKN1_LIQ_POOL_ID,
                updated_at: 25,
                accumulated_rpvs: 60,
                accumulated_rpz: 12,
                total_shares: 80,
                total_valued_shares: 4_160,
                loyalty_curve: Some(LoyaltyCurve::default()),
                stake_in_global_pool: 20_800,
                multiplier: FixedU128::from(5_u128),
                canceled: false,
            },
        );

        //user balances checks
        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE),
            bsx_tkn1_alice_amm_shares_balance + 486
        );

        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH),
            bsx_tkn1_pallet_amm_shares_balance - 486
        );
        assert_eq!(
            Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH),
            bsx_tkn2_pallet_amm_shares_balance
        );

        //liq pool balance checks
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn1_liq_pool_account),
            bsx_tkn1_liq_pool_bsx_balance - unclaimable_rewards
        );

        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_liq_pool_account),
            bsx_tkn2_liq_pool_bsx_balance
        );

        //global pool balance checks
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &global_pool_account),
            global_pool_bsx_balance + unclaimable_rewards
        );

        assert_eq!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[6]), None);

        assert_eq!(
            LiquidityMining::liq_pool_meta(BSX_TKN1_LIQ_POOL_ID).unwrap(),
            (1, GC_FARM)
        );

        // withdraw 2A
        let bsx_tkn1_bob_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &BOB);
        let bsx_tkn2_pallet_amm_shares_balance = Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH);
        let global_pool_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &global_pool_account);
        let bsx_tkn1_liq_pool_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn1_liq_pool_account);
        let bsx_tkn2_liq_pool_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_liq_pool_account);

        let unclaimable_rewards = 267_429;
        assert_ok!(LiquidityMining::withdraw_shares(
            BOB,
            PREDEFINED_DEPOSIT_IDS[1],
            BSX_TKN1_AMM,
            unclaimable_rewards,
        ));

        assert_eq!(
            LiquidityMining::global_pool(GC_FARM).unwrap(),
            GlobalPool {
                updated_at: 25,
                accumulated_rpz: 12,
                accumulated_rewards: 231_650,
                paid_accumulated_rewards: 1_164_400,
                total_shares_z: 473_680,
                ..PREDEFINED_GLOBAL_POOLS[2]
            }
        );

        assert_eq!(
            LiquidityMining::liquidity_pool(GC_FARM, BSX_TKN1_AMM).unwrap(),
            LiquidityPoolYieldFarm {
                id: BSX_TKN1_LIQ_POOL_ID,
                updated_at: 25,
                accumulated_rpvs: 60,
                accumulated_rpz: 12,
                total_shares: 0,
                total_valued_shares: 0,
                loyalty_curve: Some(LoyaltyCurve::default()),
                stake_in_global_pool: 0,
                multiplier: FixedU128::from(5_u128),
                canceled: false,
            },
        );

        //user balances checks
        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &BOB),
            bsx_tkn1_bob_amm_shares_balance + 80
        );

        assert_eq!(Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH), 0);
        assert_eq!(
            Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH),
            bsx_tkn2_pallet_amm_shares_balance
        );

        //liq pool balance checks
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn1_liq_pool_account),
            bsx_tkn1_liq_pool_bsx_balance - unclaimable_rewards
        );

        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_liq_pool_account),
            bsx_tkn2_liq_pool_bsx_balance
        );

        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &global_pool_account),
            global_pool_bsx_balance + unclaimable_rewards
        );

        assert_eq!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[1]), None);

        assert_eq!(
            LiquidityMining::liq_pool_meta(BSX_TKN1_LIQ_POOL_ID).unwrap(),
            (0, GC_FARM)
        );

        // withdraw 1B
        let bsx_tkn2_bob_amm_shares_balance = Tokens::free_balance(BSX_TKN2_SHARE_ID, &BOB);
        let bsx_tkn2_pallet_amm_shares_balance = Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH);
        let global_pool_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &global_pool_account);
        let bsx_tkn1_liq_pool_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn1_liq_pool_account);
        let bsx_tkn2_liq_pool_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_liq_pool_account);

        let unclaimable_rewards = 30_001;
        assert_ok!(LiquidityMining::withdraw_shares(
            BOB,
            PREDEFINED_DEPOSIT_IDS[2],
            BSX_TKN2_AMM,
            unclaimable_rewards
        ));

        assert_eq!(
            LiquidityMining::global_pool(GC_FARM).unwrap(),
            GlobalPool {
                updated_at: 25,
                accumulated_rpz: 12,
                accumulated_rewards: 231_650,
                paid_accumulated_rewards: 1_164_400,
                total_shares_z: 471_680,
                ..PREDEFINED_GLOBAL_POOLS[2]
            }
        );

        assert_eq!(
            LiquidityMining::liquidity_pool(GC_FARM, BSX_TKN1_AMM).unwrap(),
            LiquidityPoolYieldFarm {
                id: BSX_TKN1_LIQ_POOL_ID,
                updated_at: 25,
                accumulated_rpvs: 60,
                accumulated_rpz: 12,
                total_shares: 0,
                total_valued_shares: 0,
                loyalty_curve: Some(LoyaltyCurve::default()),
                stake_in_global_pool: 0,
                multiplier: FixedU128::from(5_u128),
                canceled: false,
            },
        );

        assert_eq!(
            LiquidityMining::liquidity_pool(GC_FARM, BSX_TKN2_AMM).unwrap(),
            LiquidityPoolYieldFarm {
                id: BSX_TKN2_LIQ_POOL_ID,
                updated_at: 25,
                accumulated_rpvs: 120,
                accumulated_rpz: 12,
                total_shares: 848,
                total_valued_shares: 47_168,
                loyalty_curve: Some(LoyaltyCurve::default()),
                stake_in_global_pool: 471_680,
                multiplier: FixedU128::from(10_u128),
                canceled: false,
            },
        );

        //user balances checks
        assert_eq!(
            Tokens::free_balance(BSX_TKN2_SHARE_ID, &BOB),
            bsx_tkn2_bob_amm_shares_balance + 25
        );

        assert_eq!(Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH), 0);
        assert_eq!(
            Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH),
            bsx_tkn2_pallet_amm_shares_balance - 25
        );

        //liq pool balance checks
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn1_liq_pool_account),
            bsx_tkn1_liq_pool_bsx_balance
        );

        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_liq_pool_account),
            bsx_tkn2_liq_pool_bsx_balance - unclaimable_rewards
        );

        //global pool balance checks
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &global_pool_account),
            global_pool_bsx_balance + unclaimable_rewards
        );

        assert_eq!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[2]), None);

        assert_eq!(
            LiquidityMining::liq_pool_meta(BSX_TKN2_LIQ_POOL_ID).unwrap(),
            (2, GC_FARM)
        );

        // withdraw 4B
        let bsx_tkn2_alice_amm_shares_balance = Tokens::free_balance(BSX_TKN2_SHARE_ID, &ALICE);
        let bsx_tkn2_pallet_amm_shares_balance = Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH);
        let global_pool_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &global_pool_account);
        let bsx_tkn1_liq_pool_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn1_liq_pool_account);
        let bsx_tkn2_liq_pool_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_liq_pool_account);

        let unclaimable_rewards = 96_473;
        assert_ok!(LiquidityMining::withdraw_shares(
            ALICE,
            PREDEFINED_DEPOSIT_IDS[5],
            BSX_TKN2_AMM,
            unclaimable_rewards,
        ));

        assert_eq!(
            LiquidityMining::global_pool(GC_FARM).unwrap(),
            GlobalPool {
                updated_at: 25,
                accumulated_rpz: 12,
                accumulated_rewards: 231_650,
                paid_accumulated_rewards: 1_164_400,
                total_shares_z: 464_000,
                ..PREDEFINED_GLOBAL_POOLS[2]
            }
        );

        assert_eq!(
            LiquidityMining::liquidity_pool(GC_FARM, BSX_TKN2_AMM).unwrap(),
            LiquidityPoolYieldFarm {
                id: BSX_TKN2_LIQ_POOL_ID,
                updated_at: 25,
                accumulated_rpvs: 120,
                accumulated_rpz: 12,
                total_shares: 800,
                total_valued_shares: 46_400,
                loyalty_curve: Some(LoyaltyCurve::default()),
                stake_in_global_pool: 464_000,
                multiplier: FixedU128::from(10_u128),
                canceled: false,
            },
        );

        //user balances checks
        assert_eq!(
            Tokens::free_balance(BSX_TKN2_SHARE_ID, &ALICE),
            bsx_tkn2_alice_amm_shares_balance + 48
        );

        assert_eq!(Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH), 0);
        assert_eq!(
            Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH),
            bsx_tkn2_pallet_amm_shares_balance - 48
        );

        //liq pool balances checks
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn1_liq_pool_account),
            bsx_tkn1_liq_pool_bsx_balance
        );
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn2_liq_pool_account),
            bsx_tkn2_liq_pool_bsx_balance - unclaimable_rewards
        );

        //global pool balance checks
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &global_pool_account),
            global_pool_bsx_balance + unclaimable_rewards
        );

        assert_eq!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[5]), None);

        assert_eq!(
            LiquidityMining::liq_pool_meta(BSX_TKN2_LIQ_POOL_ID).unwrap(),
            (1, GC_FARM)
        );

        // withdraw 2B
        let bsx_tkn2_bob_amm_shares_balance = Tokens::free_balance(BSX_TKN2_SHARE_ID, &BOB);
        let global_pool_bsx_balance = Tokens::free_balance(REWARD_CURRENCY, &global_pool_account);
        let bsx_tkn1_liq_pool_amm_shares_balance = Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn1_liq_pool_account);

        let unclaimable_rewards = 5_911_539;
        assert_ok!(LiquidityMining::withdraw_shares(
            BOB,
            PREDEFINED_DEPOSIT_IDS[3],
            BSX_TKN2_AMM,
            unclaimable_rewards
        ));

        assert_eq!(
            LiquidityMining::global_pool(GC_FARM).unwrap(),
            GlobalPool {
                updated_at: 25,
                accumulated_rpz: 12,
                accumulated_rewards: 231_650,
                paid_accumulated_rewards: 1_164_400,
                total_shares_z: 0,
                ..PREDEFINED_GLOBAL_POOLS[2]
            }
        );

        assert_eq!(
            LiquidityMining::liquidity_pool(GC_FARM, BSX_TKN2_AMM).unwrap(),
            LiquidityPoolYieldFarm {
                id: BSX_TKN2_LIQ_POOL_ID,
                updated_at: 25,
                accumulated_rpvs: 120,
                accumulated_rpz: 12,
                total_shares: 0,
                total_valued_shares: 0,
                loyalty_curve: Some(LoyaltyCurve::default()),
                stake_in_global_pool: 0,
                multiplier: FixedU128::from(10_u128),
                canceled: false,
            },
        );

        //user balances checks
        assert_eq!(
            Tokens::free_balance(BSX_TKN2_SHARE_ID, &BOB),
            bsx_tkn2_bob_amm_shares_balance + 800
        );

        assert_eq!(Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH), 0);
        assert_eq!(Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH), 0);

        //liq pool balances checks
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &bsx_tkn1_liq_pool_account),
            bsx_tkn1_liq_pool_amm_shares_balance
        );

        //global pool balance checks
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY, &global_pool_account),
            global_pool_bsx_balance + 5_911_539 //5_911_539 unclaimable rewards after withdrawn
        );

        assert_eq!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[2]), None);

        assert_eq!(
            LiquidityMining::liq_pool_meta(BSX_TKN2_LIQ_POOL_ID).unwrap(),
            (0, GC_FARM)
        );
    });

    //charlie's farm inncetivize KSM and reward currency is ACA
    //This test check if correct currency is tranfered if rewards and incetvized
    //assts are different, otherwise pool behaviour is the same as in test above.
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
        assert_ok!(LiquidityMining::deposit_shares(
            ALICE,
            CHARLIE_FARM,
            deposited_amount,
            ACA_KSM_AMM
        ));

        const DEPOSIT_ID: DepositId = 4_294_967_303;
        assert_eq!(
            LiquidityMining::deposit(DEPOSIT_ID).unwrap(),
            Deposit {
                shares: deposited_amount,
                valued_shares: 2500,
                accumulated_rpvs: 0,
                accumulated_claimed_rewards: 0,
                entered_at: 18,
                updated_at: 18,
            }
        );

        set_block_number(2_596); //period 25

        let aca_ksm_alice_amm_shares_balance = Tokens::free_balance(ACA_KSM_SHARE_ID, &ALICE);

        assert_ok!(LiquidityMining::withdraw_shares(ALICE, DEPOSIT_ID, ACA_KSM_AMM, 0));

        assert_eq!(
            Tokens::free_balance(ACA_KSM_SHARE_ID, &ALICE),
            aca_ksm_alice_amm_shares_balance + deposited_amount
        );
    });
}

#[test]
fn withdraw_shares_from_destroyed_farm_should_work() {
    //this is the case when liq. pools was removed and global pool was destroyed. Only deposits stayed in
    //the storage. In this case only amm shares should be withdrawn

    let bsx_tkn1_assets = AssetPair {
        asset_in: BSX,
        asset_out: TKN1,
    };

    let bsx_tkn2_assets = AssetPair {
        asset_in: BSX,
        asset_out: TKN2,
    };

    predefined_test_ext_with_deposits().execute_with(|| {
        let bsx_tkn1_amm_account = AMM_POOLS.with(|v| {
            v.borrow()
                .get(&asset_pair_to_map_key(bsx_tkn1_assets.clone()))
                .unwrap()
                .0
        });
        let bsx_tkn2_amm_account = AMM_POOLS.with(|v| {
            v.borrow()
                .get(&asset_pair_to_map_key(bsx_tkn2_assets.clone()))
                .unwrap()
                .0
        });

        //check if farm and pools exist
        assert!(LiquidityMining::liquidity_pool(GC_FARM, bsx_tkn1_amm_account).is_some());
        assert!(LiquidityMining::liquidity_pool(GC_FARM, bsx_tkn2_amm_account).is_some());
        assert!(LiquidityMining::global_pool(GC_FARM).is_some());

        //cancel all liq. pools in the farm
        assert_ok!(LiquidityMining::cancel_liquidity_pool(GC, GC_FARM, BSX_TKN1_AMM));
        assert_ok!(LiquidityMining::cancel_liquidity_pool(GC, GC_FARM, BSX_TKN2_AMM));

        //remove all liq. pools from farm
        assert_ok!(LiquidityMining::remove_liquidity_pool(GC, GC_FARM, BSX_TKN1_AMM));
        assert_ok!(LiquidityMining::remove_liquidity_pool(GC, GC_FARM, BSX_TKN2_AMM));

        //withdraw all undistributed rewards form global pool before destroying
        assert_ok!(LiquidityMining::withdraw_undistributed_rewards(GC, GC_FARM));

        //destroy farm
        assert_ok!(LiquidityMining::destroy_farm(GC, GC_FARM));

        //check if farm and pools was removed from storage
        assert!(LiquidityMining::liquidity_pool(GC_FARM, bsx_tkn1_amm_account).is_none());
        assert!(LiquidityMining::liquidity_pool(GC_FARM, bsx_tkn2_amm_account).is_none());
        assert!(LiquidityMining::global_pool(GC_FARM).is_none());

        let test_data = vec![
            (
                ALICE,
                0,
                50,
                2_u64,
                BSX_TKN1_LIQ_POOL_ID,
                BSX_TKN1_SHARE_ID,
                BSX_TKN1_AMM,
            ),
            (BOB, 1, 80, 1_u64, BSX_TKN1_LIQ_POOL_ID, BSX_TKN1_SHARE_ID, BSX_TKN1_AMM),
            (BOB, 2, 25, 3_u64, BSX_TKN2_LIQ_POOL_ID, BSX_TKN2_SHARE_ID, BSX_TKN2_AMM),
            (
                BOB,
                3,
                800,
                2_u64,
                BSX_TKN2_LIQ_POOL_ID,
                BSX_TKN2_SHARE_ID,
                BSX_TKN2_AMM,
            ),
            (
                ALICE,
                4,
                87,
                1_u64,
                BSX_TKN2_LIQ_POOL_ID,
                BSX_TKN2_SHARE_ID,
                BSX_TKN2_AMM,
            ),
            (
                ALICE,
                5,
                48,
                0_u64,
                BSX_TKN2_LIQ_POOL_ID,
                BSX_TKN2_SHARE_ID,
                BSX_TKN2_AMM,
            ),
            (
                ALICE,
                6,
                486,
                0_u64,
                BSX_TKN1_LIQ_POOL_ID,
                BSX_TKN1_SHARE_ID,
                BSX_TKN1_AMM,
            ),
        ];

        for (caller, deposit_id_index, withdrawn_shares, deposits_left, liq_pool_farm_id, _lp_token, amm_pool_id) in
            test_data
        {
            let bsx_tkn1_pallet_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH);
            let bsx_tkn2_pallet_amm_shares_balance = Tokens::free_balance(BSX_TKN2_SHARE_ID, &LP_SHARES_STASH);
            let bsx_tkn1_caller_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &caller);
            let bsx_tkn2_caller_shares_balance = Tokens::free_balance(BSX_TKN2_SHARE_ID, &caller);

            //withdraw
            assert_ok!(LiquidityMining::withdraw_shares(
                caller,
                PREDEFINED_DEPOSIT_IDS[deposit_id_index],
                amm_pool_id,
                0,
            ));

            let mut bsx_tkn1_shares_withdrawn = 0;
            let mut bsx_tkn2_shares_withdrawn = 0;

            if liq_pool_farm_id == BSX_TKN1_LIQ_POOL_ID {
                bsx_tkn1_shares_withdrawn = withdrawn_shares;
            } else {
                bsx_tkn2_shares_withdrawn = withdrawn_shares;
            }

            //check pool account shares balance
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
            assert_eq!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[deposit_id_index]), None);

            //check if liq. pool meta was updated
            if deposits_left.is_zero() {
                // last deposit should remove liq. pool metadata
                assert!(LiquidityMining::liq_pool_meta(liq_pool_farm_id).is_none());
            } else {
                assert_eq!(
                    LiquidityMining::liq_pool_meta(liq_pool_farm_id).unwrap(),
                    (deposits_left, GC_FARM)
                );
            }
        }
    });
}

#[test]
fn withdraw_shares_from_canceled_pool_should_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        set_block_number(10_000);

        // cancel liq. pool before withdraw test
        assert_ok!(LiquidityMining::cancel_liquidity_pool(GC, GC_FARM, BSX_TKN1_AMM));

        //let pallet_account = LiquidityMining::account_id();
        let global_pool_account = LiquidityMining::pool_account_id(GC_FARM).unwrap();
        let liq_pool_account = LiquidityMining::pool_account_id(BSX_TKN1_LIQ_POOL_ID).unwrap();

        //1-th withdraw
        let liq_pool_bsx_balance = Tokens::free_balance(BSX, &liq_pool_account);
        let global_pool_bsx_balance = Tokens::free_balance(BSX, &global_pool_account);
        let bsx_tkn1_pallet_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH);
        let bsx_tkn1_alice_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE);

        let global_pool = LiquidityMining::global_pool(GC_FARM).unwrap();
        let liq_pool = LiquidityMining::liquidity_pool(GC_FARM, BSX_TKN1_AMM).unwrap();

        let unclaimable_rewards = 168_270;
        assert_ok!(LiquidityMining::withdraw_shares(
            ALICE,
            PREDEFINED_DEPOSIT_IDS[0],
            BSX_TKN1_AMM,
            unclaimable_rewards
        ));

        assert_eq!(LiquidityMining::global_pool(GC_FARM).unwrap(), global_pool);

        assert_eq!(
            LiquidityMining::liquidity_pool(GC_FARM, BSX_TKN1_AMM).unwrap(),
            LiquidityPoolYieldFarm {
                total_shares: liq_pool.total_shares - 50,
                total_valued_shares: liq_pool.total_valued_shares - 2500,
                ..liq_pool
            }
        );

        assert_eq!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[0]), None);

        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH),
            bsx_tkn1_pallet_amm_shares_balance - 50
        );

        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE),
            bsx_tkn1_alice_amm_shares_balance + 50
        );

        assert_eq!(
            Tokens::free_balance(BSX, &global_pool_account),
            global_pool_bsx_balance + unclaimable_rewards
        );

        assert_eq!(
            Tokens::free_balance(BSX, &liq_pool_account),
            liq_pool_bsx_balance - unclaimable_rewards
        );

        assert_eq!(
            LiquidityMining::liq_pool_meta(BSX_TKN1_LIQ_POOL_ID).unwrap(),
            (2, GC_FARM)
        );

        //2-nd withdraw
        let liq_pool_bsx_balance = Tokens::free_balance(BSX, &liq_pool_account);
        let global_pool_bsx_balance = Tokens::free_balance(BSX, &global_pool_account);
        let bsx_tkn1_pallet_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH);
        let bsx_tkn1_alice_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE);

        let global_pool = LiquidityMining::global_pool(GC_FARM).unwrap();
        let liq_pool = LiquidityMining::liquidity_pool(GC_FARM, BSX_TKN1_AMM).unwrap();

        let unclaimable_rewards = 2_055_086;
        let shares_amount = 486;
        let valued_shares_amount = 38_880;

        assert_ok!(LiquidityMining::withdraw_shares(
            ALICE,
            PREDEFINED_DEPOSIT_IDS[6],
            BSX_TKN1_AMM,
            unclaimable_rewards
        ));

        assert_eq!(LiquidityMining::global_pool(GC_FARM).unwrap(), global_pool);

        assert_eq!(
            LiquidityMining::liquidity_pool(GC_FARM, BSX_TKN1_AMM).unwrap(),
            LiquidityPoolYieldFarm {
                total_shares: liq_pool.total_shares - shares_amount,
                total_valued_shares: liq_pool.total_valued_shares - valued_shares_amount,
                ..liq_pool
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
            Tokens::free_balance(BSX, &global_pool_account),
            global_pool_bsx_balance + unclaimable_rewards
        );

        assert_eq!(
            Tokens::free_balance(BSX, &liq_pool_account),
            liq_pool_bsx_balance - unclaimable_rewards
        );

        assert_eq!(
            LiquidityMining::liq_pool_meta(BSX_TKN1_LIQ_POOL_ID).unwrap(),
            (1, GC_FARM)
        );

        //3-th withdraw
        let liq_pool_bsx_balance = Tokens::free_balance(BSX, &liq_pool_account);
        let global_pool_bsx_balance = Tokens::free_balance(BSX, &global_pool_account);
        let bsx_tkn1_pallet_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH);
        let bsx_tkn1_bob_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &BOB);

        let global_pool = LiquidityMining::global_pool(GC_FARM).unwrap();
        let liq_pool = LiquidityMining::liquidity_pool(GC_FARM, BSX_TKN1_AMM).unwrap();

        let unclaimable_rewards = 228_572;
        let shares_amount = 80;

        assert_ok!(LiquidityMining::withdraw_shares(
            BOB,
            PREDEFINED_DEPOSIT_IDS[1],
            BSX_TKN1_AMM,
            unclaimable_rewards
        ));

        assert_eq!(LiquidityMining::global_pool(GC_FARM).unwrap(), global_pool);

        assert_eq!(
            LiquidityMining::liquidity_pool(GC_FARM, BSX_TKN1_AMM).unwrap(),
            LiquidityPoolYieldFarm {
                total_shares: 0,
                total_valued_shares: 0,
                ..liq_pool
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
            Tokens::free_balance(BSX, &global_pool_account),
            global_pool_bsx_balance + unclaimable_rewards
        );

        assert_eq!(
            Tokens::free_balance(BSX, &liq_pool_account),
            liq_pool_bsx_balance - unclaimable_rewards
        );

        //Last withdraw should NOT remove pool_metadata because liq. pool can be
        //resumed in the future
        assert_eq!(
            LiquidityMining::liq_pool_meta(BSX_TKN1_LIQ_POOL_ID).unwrap(),
            (0, GC_FARM)
        );
    });
}

#[test]
fn withdraw_shares_from_removed_pool_should_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        set_block_number(10_000);

        //cancel liq. pool before removing
        assert_ok!(LiquidityMining::cancel_liquidity_pool(GC, GC_FARM, BSX_TKN1_AMM));

        //remove liq. pool before test
        assert_ok!(LiquidityMining::remove_liquidity_pool(GC, GC_FARM, BSX_TKN1_AMM));

        assert_eq!(LiquidityMining::liquidity_pool(GC_FARM, BSX_TKN1_AMM), None);

        let global_pool = LiquidityMining::global_pool(GC_FARM).unwrap();

        let liq_pool_id_removed: PoolId = BSX_TKN1_LIQ_POOL_ID;
        let globa_pool_account = LiquidityMining::pool_account_id(GC_FARM).unwrap();
        let bsx_tkn1_pallet_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH);
        let global_pool_bsx_balance = Tokens::free_balance(BSX, &globa_pool_account);
        let bsx_tkn1_alice_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE);
        let alice_bsx_balance = Tokens::free_balance(BSX, &ALICE);

        //1-th withdraw
        assert_ok!(LiquidityMining::withdraw_shares(
            ALICE,
            PREDEFINED_DEPOSIT_IDS[0],
            BSX_TKN1_AMM,
            0
        ));

        let shares_amount = 50;

        assert_eq!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[0]), None);

        assert_eq!(
            LiquidityMining::liq_pool_meta(liq_pool_id_removed).unwrap(),
            (2, GC_FARM)
        );

        assert_eq!(LiquidityMining::global_pool(GC_FARM).unwrap(), global_pool);

        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH),
            bsx_tkn1_pallet_amm_shares_balance - shares_amount
        );
        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE),
            bsx_tkn1_alice_amm_shares_balance + shares_amount
        );

        //removed liq. pool don't pay rewards, only transfer amm shares
        assert_eq!(Tokens::free_balance(BSX, &ALICE), alice_bsx_balance);
        assert_eq!(Tokens::free_balance(BSX, &globa_pool_account), global_pool_bsx_balance);

        //2-nd withdraw
        let bsx_tkn1_alice_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE);
        let alice_bsx_balance = Tokens::free_balance(BSX, &ALICE);
        let bsx_tkn1_pallet_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH);
        let shares_amount = 486;

        assert_ok!(LiquidityMining::withdraw_shares(
            ALICE,
            PREDEFINED_DEPOSIT_IDS[6],
            BSX_TKN1_AMM,
            0,
        ));

        assert_eq!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[6]), None);

        assert_eq!(LiquidityMining::global_pool(GC_FARM).unwrap(), global_pool);

        assert_eq!(
            LiquidityMining::liq_pool_meta(liq_pool_id_removed).unwrap(),
            (1, GC_FARM)
        );

        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH),
            bsx_tkn1_pallet_amm_shares_balance - shares_amount
        );
        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &ALICE),
            bsx_tkn1_alice_amm_shares_balance + shares_amount
        );

        //removed liq. pool don't pay rewards, only transfer amm shares
        assert_eq!(Tokens::free_balance(BSX, &ALICE), alice_bsx_balance);
        assert_eq!(Tokens::free_balance(BSX, &globa_pool_account), global_pool_bsx_balance);

        //3-th withdraw
        let bsx_tkn1_bob_amm_shares_balance = Tokens::free_balance(BSX_TKN1_SHARE_ID, &BOB);
        let bob_bsx_balance = Tokens::free_balance(BSX, &BOB);
        let shares_amount = 80;

        assert_ok!(LiquidityMining::withdraw_shares(
            BOB,
            PREDEFINED_DEPOSIT_IDS[1],
            BSX_TKN1_AMM,
            0
        ));

        assert_eq!(LiquidityMining::deposit(PREDEFINED_DEPOSIT_IDS[1]), None);

        assert_eq!(LiquidityMining::global_pool(GC_FARM).unwrap(), global_pool);

        assert_eq!(Tokens::free_balance(BSX_TKN1_SHARE_ID, &LP_SHARES_STASH), 0);
        assert_eq!(
            Tokens::free_balance(BSX_TKN1_SHARE_ID, &BOB),
            bsx_tkn1_bob_amm_shares_balance + shares_amount
        );

        //removed liq. pool don't pay rewards, only transfer amm shares
        assert_eq!(Tokens::free_balance(BSX, &BOB), bob_bsx_balance);
        assert_eq!(Tokens::free_balance(BSX, &globa_pool_account), global_pool_bsx_balance);

        //last withdrawn from removed pool should remove liq. pool metadata
        assert_eq!(LiquidityMining::liq_pool_meta(liq_pool_id_removed), None);
    });
}

#[test]
fn withdraw_shares_pool_metadata_not_found_should_not_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        // liq. pool or liq. pool metadata don't exists for this nft id
        // 723_500_752_978_313_215 -> liq. pool id: u32::max(), nft sequence: 168_453_145
        const DEPOSIT_ID: DepositId = 723_500_752_978_313_215_u128;
        const NOT_FOUND_METADATA: mock::AccountId = 999_999_999_999;
        assert_noop!(
            LiquidityMining::withdraw_shares(ALICE, DEPOSIT_ID, NOT_FOUND_METADATA, 0),
            Error::<Test>::LiquidityPoolNotFound
        );
    });
}

#[test]
fn withdraw_shares_invalid_deposit_id_should_not_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        let invalid_deposit_id = 684;

        assert_noop!(
            LiquidityMining::withdraw_shares(ALICE, invalid_deposit_id, BSX_TKN1_AMM, 0),
            Error::<Test>::InvalidDepositId
        );
    });
}

#[test]
fn withdraw_shares_deposit_not_found_should_not_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        //72_334_321_125_861_359_621 -> liq. pool id: 5, nft sequence: 16_841_646_546
        //deposit and nft with this id don't exist
        assert_noop!(
            LiquidityMining::withdraw_shares(ALICE, 72_334_321_125_861_359_621, BSX_TKN1_AMM, 0),
            Error::<Test>::DepositNotFound
        );
    });
}
