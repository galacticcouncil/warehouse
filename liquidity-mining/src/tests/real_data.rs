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

use crate::tests::mock::LiquidityMining2;

#[test]
fn rewards_should_be_calculated_correctly_when_onchain_data_are_used() {
    new_test_ext().execute_with(|| {
        let _ = with_transaction(|| {
            const GLOBAL_FARM: GlobalFarmId = 1;
            const YIELD_FARM: YieldFarmId = 2;

            const DEPOSIT: DepositId = 1;

            let pot = LiquidityMining2::pot_account_id().unwrap();
            let g_farm_acc = LiquidityMining2::farm_account_id(1).unwrap();

            Tokens::set_balance(Origin::root(), pot, 0, 1_000_000_000_000_000_000_000_000, 0).unwrap();
            Tokens::set_balance(Origin::root(), g_farm_acc, 0, 1_000_000_000_000_000_000_000_000, 0).unwrap();

            //initialize farms
            set_block_number(16_904_576);

            let g_farm = GlobalFarmData::<Test, Instance2> {
                id: 1,
                owner: GC,
                updated_at: 17_371_477,
                total_shares_z: 585_577_864_606_183_635_618,
                accumulated_rpz: FixedU128::from_inner(107_063_658_619_594_554),
                reward_currency: 0,
                pending_rewards: 224_689_u128,
                accumulated_paid_rewards: 24_330_154_676_984_555_347_u128,
                yield_per_period: Perquintill::from_rational(152_207_001_522, 1_000_000_000_000_000_000_u128),
                planned_yielding_periods: 1_314_000_u64,
                blocks_per_period: 1,
                incentivized_asset: 0,
                max_reward_per_period: 380_517_503_805_175_u128,
                min_deposit: 1_000,
                live_yield_farms_count: 3,
                total_yield_farms_count: 3,
                price_adjustment: FixedU128::one(),
                state: FarmState::Active,
            };

            let y_farm = YieldFarmData::<Test, Instance2> {
                id: 2,
                updated_at: 17_371_477_u64,
                total_shares: 27_440_871_751_343_975_814_u128,
                total_valued_shares: 28_367_193_726_018_988_982_u128,
                accumulated_rpvs: FixedU128::from_inner(107_044_176_123_399_551_u128),
                accumulated_rpz: FixedU128::from_inner(107_063_658_619_594_554_u128),
                loyalty_curve: Some(LoyaltyCurve {
                    initial_reward_percentage: FixedU128::from_inner(500_000_000_000_000_000_u128),
                    scale_coef: 50_000,
                }),
                multiplier: FixedU128::from(2_u128),
                state: FarmState::Active,
                entries_count: 54,
                left_to_distribute: 1_180_522_127_113_710_220_u128,
                total_stopped: 0,
                _phantom: PhantomData,
            };

            let deposit_entry = YieldFarmEntry::<Test, Instance2> {
                global_farm_id: 1,
                yield_farm_id: 2,
                valued_shares: 2_333_493_900_158_141_382_u128,
                accumulated_rpvs: FixedU128::from_inner(16_925_875_190_250_743_u128),
                accumulated_claimed_rewards: 182_234_166_129_320_334_u128,
                entered_at: 16_772_854_u64,
                updated_at: 17_310_743_u64,
                stopped_at_creation: 0,
                _phantom: PhantomData,
            };

            let mut deposit = DepositData::<Test, Instance2> {
                shares: 2_297_319_211_600_000_000_u128,
                amm_pool_id: BSX_TKN1_AMM,
                yield_farm_entries: BoundedVec::default(),
            };
            deposit.add_yield_farm_entry(deposit_entry).unwrap();

            crate::pallet::GlobalFarm::<Test, Instance2>::insert(g_farm.id, g_farm.clone());
            crate::pallet::ActiveYieldFarm::<Test, Instance2>::insert(BSX_TKN1_AMM, g_farm.id, y_farm.id);
            crate::pallet::YieldFarm::<Test, Instance2>::insert((BSX_TKN1_AMM, g_farm.id, y_farm.id), y_farm);
            crate::pallet::Deposit::<Test, Instance2>::insert(1, deposit);

            set_block_number(17_412_361);
            let (_, _, claimed, _) = LiquidityMining2::claim_rewards(ALICE, 1, 2, false).unwrap();

            assert_eq!(claimed, 48_420_552_738_013_981_u128);

            TransactionOutcome::Commit(DispatchResult::Ok(()))
        });
    });
}
