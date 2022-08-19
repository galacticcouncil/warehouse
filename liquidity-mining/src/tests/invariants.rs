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
use proptest::prelude::*;

const ONE: Balance = 1_000_000_000_000;
const TOLERANCE: Balance = 1_000;
const REWARD_CURRENCY: AssetId = BSX;

fn total_shares_z() -> impl Strategy<Value = Balance> {
    0..100_000_000_000_000_u128
}

fn left_to_distribute() -> impl Strategy<Value = Balance> {
    ONE..u128::MAX / 2
}

fn reward_per_period() -> impl Strategy<Value = Balance> {
    190 * ONE..1_000_000_000 * ONE //190BSX -> distribute 3B in 3 years(6s blocks) with 1 block per period
}

fn global_farm_accumulated_rewards() -> impl Strategy<Value = (Balance, Balance)> {
    (0..10_000_000_000 * ONE, 0..10_000_000_000 * ONE)
}

prop_compose! {
    fn global_farm_and_current_period()
        (
            current_period in 1_000_000..52_560_000_u64,
            total_shares_z in total_shares_z(),
            (accumulated_rewards, paid_accumulated_rewards) in global_farm_accumulated_rewards(),
            reward_per_period in reward_per_period(),
        )(
            accumulated_rpz in 0..accumulated_rewards.checked_div(total_shares_z).unwrap(),
            accumulated_rewards in Just(accumulated_rewards),
            paid_accumulated_rewards in Just(paid_accumulated_rewards),
            reward_per_period in Just(reward_per_period),
            total_shares_z in Just(total_shares_z),
            updated_at in 0..current_period,
            current_period in Just(current_period),
        )
    -> (GlobalFarmData<Test, Instance1>, BlockNumber) {
        (GlobalFarmData::<Test, Instance1> {
            id: 1,
            owner: ALICE,
            updated_at,
            total_shares_z,
            accumulated_rpz,
            reward_currency: REWARD_CURRENCY,
            accumulated_rewards,
            paid_accumulated_rewards,
            yield_per_period: Perquintill::from_float(0.002),
            planned_yielding_periods: 1_000,
            blocks_per_period: 1_000,
            incentivized_asset: REWARD_CURRENCY,
            max_reward_per_period: reward_per_period,
            min_deposit: 1,
            yield_farms_count: (0,0),
            price_adjustment: FixedU128::one(),
            state: FarmState::Active,
        }, current_period)
    }
}

prop_compose! {
    fn get_both_farms_and_current_period_and_yield_farm_rewards ()(
            (global_farm, current_period) in global_farm_and_current_period(),
        )(
            current_period in Just(current_period),
            yield_farm_accumulated_rpz in 0..global_farm.accumulated_rpz,
            yield_farm_updated_at in global_farm.updated_at..current_period,
            global_farm in Just(global_farm),
            yield_farm_reward_per_period in reward_per_period(),
        ) -> (GlobalFarmData<Test,Instance1>, YieldFarmData<Test, Instance1>, BlockNumber, Balance) {
            let yield_farm_rewards = yield_farm_reward_per_period * (current_period - yield_farm_updated_at) as u128;

            //multiplier == 1 => valued_shares== Z
            let rpvs = yield_farm_rewards.checked_div(global_farm.total_shares_z).unwrap();

            let yield_farm = YieldFarmData::<Test, Instance1> {
                id: 2,
                updated_at: yield_farm_updated_at,
                total_shares: Default::default(),
                total_valued_shares: global_farm.total_shares_z,
                accumulated_rpvs: rpvs,
                accumulated_rpz: yield_farm_accumulated_rpz,
                loyalty_curve: Default::default(),
                multiplier: One::one(),
                state: FarmState::Active,
                entries_count: Default::default(),
                _phantom: Default::default(),
            };

            (global_farm, yield_farm, current_period, yield_farm_rewards)
        }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1_000))]
    #[test]
    fn update_global_farm(
        (mut farm, current_period) in global_farm_and_current_period(),
        left_to_distribute in left_to_distribute(),
    ) {
        new_test_ext().execute_with(|| {
            let farm_account = LiquidityMining::farm_account_id(farm.id).unwrap();
            Tokens::set_balance(Origin::root(), farm_account, REWARD_CURRENCY, left_to_distribute, 0).unwrap();

            //NOTE: _0 - before action, _1 - after action
            let accumulated_rewards_0 = farm.accumulated_rewards;
            let accumulated_rpz_0 = farm.accumulated_rpz;
            let reward_per_period = farm.max_reward_per_period;
            let reward = LiquidityMining::update_global_farm(&mut farm, current_period, reward_per_period).unwrap();

            let s_0 = accumulated_rpz_0
                .checked_mul(farm.total_shares_z).unwrap()
                .checked_add(reward).unwrap();
            let s_1 = farm.accumulated_rpz.checked_mul(farm.total_shares_z).unwrap();
            let invariant = FixedU128::from((s_0, ONE)) / FixedU128::from((s_1, ONE));

            assert_eq_approx!(
                invariant,
                FixedU128::from(1u128),
                FixedU128::from((TOLERANCE, ONE)),
                "acc_rpz[1] x shares = acc_rpz[0] x shares + reward"
            );

            assert!(
                farm.accumulated_rewards == accumulated_rewards_0.checked_add(reward).unwrap(),
                "acc_rewards[1] = acc_rewards[0] + reward"
            );
        });
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]
    #[test]
    fn claim_from_global_farm(
        (mut global_farm, mut yield_farm, _, _) in get_both_farms_and_current_period_and_yield_farm_rewards()
    ) {
        new_test_ext().execute_with(|| {
            //NOTE: _0 - before action, _1 - after action
            let sum_accumulated_rewards_0 = global_farm.accumulated_rewards
                .checked_add(global_farm.paid_accumulated_rewards).unwrap();

            let stake_in_global_farm = yield_farm.total_valued_shares;  //multiplier == 1 => valued_share == z
            let _ = LiquidityMining::claim_from_global_farm(&mut global_farm, &mut yield_farm, stake_in_global_farm).unwrap();

            let sum_accumulated_rewards_1 = global_farm.accumulated_rewards
                .checked_add(global_farm.paid_accumulated_rewards).unwrap();

            pretty_assertions::assert_eq!(sum_accumulated_rewards_0, sum_accumulated_rewards_1);
        });
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]
    #[test]
    fn update_yield_farm(
        (_, mut yield_farm, current_period, yield_farm_rewards) in get_both_farms_and_current_period_and_yield_farm_rewards(),
        left_to_distribute in left_to_distribute(),
    ) {
        new_test_ext().execute_with(|| {
            const GLOBAL_FARM_ID: GlobalFarmId = 1;
            let global_farm_account = LiquidityMining::farm_account_id(GLOBAL_FARM_ID).unwrap();
            let yield_farm_account = LiquidityMining::farm_account_id(yield_farm.id).unwrap();
            Tokens::set_balance(Origin::root(), global_farm_account, REWARD_CURRENCY, left_to_distribute, 0).unwrap();

            //rewads for yield farm are paid from pot account
            let pot_account = LiquidityMining::pot_account_id();
            Tokens::set_balance(Origin::root(), pot_account, REWARD_CURRENCY, left_to_distribute, 0).unwrap();

            //NOTE: _0 - before action, _1 - after action
            let global_farm_balance_0 = Tokens::total_balance(REWARD_CURRENCY, &global_farm_account);
            let yield_farm_balance_0 = Tokens::total_balance(REWARD_CURRENCY, &yield_farm_account);

            let accumulated_rpvs_0 = yield_farm.accumulated_rpvs;

            let _ = LiquidityMining::update_yield_farm(
                &mut yield_farm, yield_farm_rewards, current_period, GLOBAL_FARM_ID, REWARD_CURRENCY).unwrap();

            //invariant 1
            let global_farm_balance_1 = Tokens::total_balance(REWARD_CURRENCY, &global_farm_account);
            let yield_farm_balance_1 = Tokens::total_balance(REWARD_CURRENCY, &yield_farm_account);
            let s_0 = global_farm_balance_0 + yield_farm_balance_0;
            let s_1 = global_farm_balance_1 + yield_farm_balance_1;
            let invariant = FixedU128::from((s_0, ONE)) / FixedU128::from((s_1, ONE));

            assert_eq_approx!(
                invariant,
                FixedU128::from(1u128),
                FixedU128::from((TOLERANCE, ONE)),
                "invariant: global_farm_balance + yield_farm_balance"
            );

            //invariant 2
            let s_0 = global_farm_balance_0 + accumulated_rpvs_0 * yield_farm.total_valued_shares;
            let s_1 = global_farm_balance_1 + yield_farm.accumulated_rpz * yield_farm.total_valued_shares;
            let invariant = FixedU128::from((s_0, ONE)) / FixedU128::from((s_1, ONE));

            assert_eq_approx!(
                invariant,
                FixedU128::from(1u128),
                FixedU128::from((TOLERANCE, ONE)),
                "invariant: global_farm_balance + acc_rpvs * total_valued_shares"
            );
        });
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]
    #[test]
    fn update_global_farm_left_to_distribute_invariant(
        (mut global_farm, _, current_period, _) in get_both_farms_and_current_period_and_yield_farm_rewards(),
        left_to_distribute in left_to_distribute(),
    ) {
        new_test_ext().execute_with(|| {
            const GLOBAL_FARM_ID: GlobalFarmId = 1;
            let global_farm_account = LiquidityMining::farm_account_id(GLOBAL_FARM_ID).unwrap();
            Tokens::set_balance(Origin::root(), global_farm_account, REWARD_CURRENCY, left_to_distribute, 0).unwrap();

            let left_to_distribute_0 = Tokens::free_balance(REWARD_CURRENCY, &global_farm_account);
            let reward_per_period = global_farm.max_reward_per_period;

            let reward =
                LiquidityMining::update_global_farm(&mut global_farm, current_period, reward_per_period).unwrap();

            let s_0 = (left_to_distribute_0 - reward).max(0);
            let s_1 = Tokens::free_balance(REWARD_CURRENCY, &global_farm_account);
            let invariant = FixedU128::from((s_0, ONE)) / FixedU128::from((s_1, ONE));

            assert_eq_approx!(
                invariant,
                FixedU128::from(1u128),
                FixedU128::from((TOLERANCE, ONE)),
                "left_to_distribute[1] = max(0, left_to_distribute[0] - reward)"
            );
        });
    }
}
