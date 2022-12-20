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
use crate::tests::test_ext::new_test_ext;
use pretty_assertions::assert_eq;
use proptest::prelude::*;
use sp_arithmetic::traits::{CheckedAdd, CheckedMul};

const ONE: Balance = 1_000_000_000_000;
const TOLERANCE: Balance = 1_000;
const REWARD_CURRENCY: AssetId = BSX;

//6s blocks
const BLOCK_PER_YEAR: u64 = 5_256_000;

fn total_shares_z() -> impl Strategy<Value = Balance> {
    0..1_000_000_000 * ONE
}

fn left_to_distribute() -> impl Strategy<Value = Balance> {
    190 * ONE..100_000 * ONE
}

fn reward_per_period() -> impl Strategy<Value = Balance> {
    190 * ONE..1_000_000 * ONE //190BSX -> distribute 3B in 3 years(6s blocks) with 1 block per period
}

fn global_farm_accumulated_rewards() -> impl Strategy<Value = (Balance, Balance)> {
    (0..10_000_000_000 * ONE, 0..10_000_000_000 * ONE)
}

fn accumulated_rpz(total_shares_z: Balance, pending_rewards: Balance) -> impl Strategy<Value = Balance> {
    0..pending_rewards.checked_div(total_shares_z).unwrap().max(1)
}

prop_compose! {
    fn get_global_farm()
        (
            total_shares_z in total_shares_z(),
            (pending_rewards, accumulated_paid_rewards) in global_farm_accumulated_rewards(),
            reward_per_period in reward_per_period(),
        )(
            accumulated_rpz in accumulated_rpz(total_shares_z, pending_rewards),
            pending_rewards in Just(pending_rewards),
            accumulated_paid_rewards in Just(accumulated_paid_rewards),
            reward_per_period in Just(reward_per_period),
            total_shares_z in Just(total_shares_z),
            updated_at in 1_000_000..(BLOCK_PER_YEAR + 1_000_000),
        )
    -> GlobalFarmData<Test, Instance1> {
        GlobalFarmData::<Test, Instance1> {
            id: 1,
            owner: ALICE,
            updated_at,
            total_shares_z,
            accumulated_rpz: FixedU128::from(accumulated_rpz),
            reward_currency: REWARD_CURRENCY,
            pending_rewards,
            accumulated_paid_rewards,
            yield_per_period: Perquintill::from_float(0.002),
            planned_yielding_periods: 1_000,
            blocks_per_period: 1_000,
            incentivized_asset: REWARD_CURRENCY,
            max_reward_per_period: reward_per_period,
            min_deposit: 1,
            live_yield_farms_count: Zero::zero(),
            total_yield_farms_count: Zero::zero(),
            price_adjustment: FixedU128::one(),
            state: FarmState::Active,
        }
    }
}

prop_compose! {
    fn get_farms()
        (
            global_farm in get_global_farm(),
        )(
            yield_farm_accumulated_rpz in 0..global_farm.accumulated_rpz.checked_div_int(1_u128).unwrap().max(1),
            tmp_reward in 100_000 * ONE..5_256_000_000 * ONE, //max: 10K for 1 year, every block
            yield_farm_updated_at in global_farm.updated_at - 1_000..global_farm.updated_at,
            global_farm in Just(global_farm),
        )
    -> (GlobalFarmData<Test, Instance1>, YieldFarmData<Test,Instance1>) {
        //multiplier == 1 => valued_shares== Z
        let rpvs = tmp_reward.checked_div(global_farm.total_shares_z).unwrap();

        let yield_farm = YieldFarmData::<Test, Instance1> {
            id: 2,
            updated_at: yield_farm_updated_at,
            total_shares: Default::default(),
            total_valued_shares: global_farm.total_shares_z,
            accumulated_rpvs: FixedU128::from(rpvs),
            accumulated_rpz: FixedU128::from(yield_farm_accumulated_rpz),
            loyalty_curve: Default::default(),
            multiplier: One::one(),
            state: FarmState::Active,
            entries_count: Default::default(),
            left_to_distribute: Default::default(),
            total_stopped: Default::default(),
            _phantom: Default::default(),
        };

        (global_farm, yield_farm)
    }
}

prop_compose! {
    fn get_global_farm_and_current_period()
        (
            global_farm in get_global_farm(),
        )(
            current_period in global_farm.updated_at..(global_farm.updated_at + BLOCK_PER_YEAR),
            global_farm in Just(global_farm),
        )
    -> (GlobalFarmData<Test, Instance1>, BlockNumber) {
        (global_farm, current_period)
    }
}

prop_compose! {
    fn get_farms_and_current_period_and_yield_farm_rewards()
        (
            (global_farm, yield_farm) in get_farms(),
        )(
            current_period in global_farm.updated_at..(global_farm.updated_at + BLOCK_PER_YEAR),
            yield_farm in Just(yield_farm),
            global_farm in Just(global_farm),
        )
    -> (GlobalFarmData<Test, Instance1>, YieldFarmData<Test, Instance1>, BlockNumber, Balance) {
        //+1 rounding
        let yield_farm_rewards = yield_farm.accumulated_rpvs.checked_mul_int(yield_farm.total_valued_shares).unwrap() + 1;

        (global_farm, yield_farm, current_period, yield_farm_rewards)
    }
}

prop_compose! {
    fn get_farms_and_current_period_and_yield_farm_rewards_and_lef_to_distribute()
        (
            (global_farm, yield_farm, current_period, yield_farm_rewards) in get_farms_and_current_period_and_yield_farm_rewards(),
        )(
            left_to_distribute in yield_farm_rewards + ONE..yield_farm_rewards + 1_000_000 * ONE,
            global_farm in Just(global_farm),
            yield_farm in Just(yield_farm),
            current_period in Just(current_period),
            yield_farm_rewards in Just(yield_farm_rewards),
        )
    -> (GlobalFarmData<Test, Instance1>, YieldFarmData<Test, Instance1>, BlockNumber, Balance, Balance) {

        (global_farm, yield_farm, current_period, yield_farm_rewards, left_to_distribute)
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1_000))]
    #[test]
    fn sync_global_farm(
        (mut farm, current_period) in get_global_farm_and_current_period(),
        left_to_distribute in left_to_distribute(),
    ) {
        new_test_ext().execute_with(|| {
            let _ = with_transaction(|| {
                let farm_account = LiquidityMining::farm_account_id(farm.id).unwrap();
                Tokens::set_balance(Origin::root(), farm_account, REWARD_CURRENCY, left_to_distribute, 0).unwrap();

                //NOTE: _0 - before action, _1 - after action
                let pending_rewards_0 = farm.pending_rewards;
                let accumulated_rpz_0 = farm.accumulated_rpz;
                let reward = LiquidityMining::sync_global_farm(&mut farm, current_period).unwrap();

                let s_0 = accumulated_rpz_0
                    .checked_mul(&FixedU128::from((farm.total_shares_z, ONE))).unwrap()
                    .checked_add(&FixedU128::from((reward, ONE))).unwrap();
                let s_1 = farm.accumulated_rpz.checked_mul(&FixedU128::from((farm.total_shares_z, ONE))).unwrap();

                assert_eq_approx!(
                    s_0,
                    s_1,
                    FixedU128::from((TOLERANCE, ONE)),
                    "acc_rpz[1] x shares = acc_rpz[0] x shares + reward"
                );

                assert!(
                    farm.pending_rewards == pending_rewards_0.checked_add(reward).unwrap(),
                    "acc_rewards[1] = acc_rewards[0] + reward"
                );

                TransactionOutcome::Commit(DispatchResult::Ok(()))
            });
        });
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1_000))]
    #[test]
    fn calculate_rewards_from_pot(
        (mut global_farm, mut yield_farm) in get_farms()
    ) {
        new_test_ext().execute_with(|| {
            //NOTE: _0 - before action, _1 - after action
            let sum_accumulated_rewards_0 = global_farm.pending_rewards
                .checked_add(global_farm.accumulated_paid_rewards).unwrap();

            let stake_in_global_farm = yield_farm.total_valued_shares;  //multiplier == 1 => valued_share == z
            let _ = LiquidityMining::calculate_rewards_from_pot(&mut global_farm, &mut yield_farm, stake_in_global_farm).unwrap();

            let sum_accumulated_rewards_1 = global_farm.pending_rewards
                .checked_add(global_farm.accumulated_paid_rewards).unwrap();

            assert_eq!(sum_accumulated_rewards_0, sum_accumulated_rewards_1);
        });
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1_000))]
    #[test]
    fn sync_yield_farm(
        (mut global_farm, mut yield_farm, current_period, _, left_to_distribute) in get_farms_and_current_period_and_yield_farm_rewards_and_lef_to_distribute(),
    ) {
        new_test_ext().execute_with(|| {
            let _ = with_transaction(|| {
                const GLOBAL_FARM_ID: GlobalFarmId = 1;

                let pot = LiquidityMining::pot_account_id().unwrap();
                let global_farm_account = LiquidityMining::farm_account_id(GLOBAL_FARM_ID).unwrap();
                //rewads for yield farm are paid from global-farm's account to pot
                Tokens::set_balance(Origin::root(), global_farm_account, REWARD_CURRENCY, left_to_distribute, 0).unwrap();

                //NOTE: _0 - before action, _1 - after action
                let pot_balance_0 = Tokens::total_balance(REWARD_CURRENCY, &pot);
                let global_farm_balance_0 = Tokens::total_balance(REWARD_CURRENCY, &global_farm_account);
                let pending_rewards_0 = global_farm.pending_rewards;
                let accumulated_rpvs_0 = yield_farm.accumulated_rpvs;

                LiquidityMining::sync_yield_farm(
                    &mut yield_farm, &mut global_farm, current_period).unwrap();

                let global_farm_balance_1 = Tokens::total_balance(REWARD_CURRENCY, &global_farm_account);

                //invariant 1
                //NOTE: yield-farm's rewards are left in the pot until user claims.
                let pot_balance_1 = Tokens::total_balance(REWARD_CURRENCY, &pot);
                let s_0 = global_farm_balance_0 + pot_balance_0;
                let s_1 = global_farm_balance_1 + pot_balance_1;

                assert_eq!(
                    s_0,
                    s_1,
                    "invariant: `global_farm_balance + pot_balance` is always constant"
               );

                //invariant 2
                let s_0 = FixedU128::from((pending_rewards_0, ONE)) + accumulated_rpvs_0 * FixedU128::from((yield_farm.total_valued_shares, ONE));
                let s_1 = FixedU128::from((global_farm.pending_rewards, ONE)) + yield_farm.accumulated_rpvs * FixedU128::from((yield_farm.total_valued_shares, ONE));

                assert_eq!(
                    s_0,
                    s_1,
                    "invariant: `global_farm.pending_rewards + yield_farm.accumulated_rpvs * yield_farm.total_valued_shares` is always constant"
               );

                TransactionOutcome::Commit(DispatchResult::Ok(()))
            });
        });
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1_000))]
    #[test]
    fn sync_global_farm_left_to_distribute_invariant(
        (mut global_farm, _, current_period, _, left_to_distribute) in get_farms_and_current_period_and_yield_farm_rewards_and_lef_to_distribute(),
    ) {
        new_test_ext().execute_with(|| {
            let _ = with_transaction(|| {
                const GLOBAL_FARM_ID: GlobalFarmId = 1;
                let global_farm_account = LiquidityMining::farm_account_id(GLOBAL_FARM_ID).unwrap();
                let pot = LiquidityMining::pot_account_id().unwrap();
                Tokens::set_balance(Origin::root(), global_farm_account, REWARD_CURRENCY, left_to_distribute, 0).unwrap();

                let left_to_distribute_0 = Tokens::free_balance(REWARD_CURRENCY, &global_farm_account);
                let pot_balance_0 = Tokens::free_balance(REWARD_CURRENCY, &pot);

                let reward =
                    LiquidityMining::sync_global_farm(&mut global_farm, current_period).unwrap();

                let s_0 = (left_to_distribute_0 - reward).max(0);
                let s_1 = Tokens::free_balance(REWARD_CURRENCY, &global_farm_account);

                assert_eq!(
                    s_0,
                    s_1,
                    "left_to_distribute[1] = max(0, left_to_distribute[0] - reward)"
                );

                let s_0 = left_to_distribute_0 + pot_balance_0;
                let s_1 = Tokens::free_balance(REWARD_CURRENCY, &global_farm_account) + Tokens::free_balance(REWARD_CURRENCY, &pot);

                assert_eq!(
                    s_0,
                    s_1,
                    "global_farm_account[0] + pot[0] = global_farm_account[1] + pot[1]"
                );

                TransactionOutcome::Commit(DispatchResult::Ok(()))
            });
        });
    }
}
