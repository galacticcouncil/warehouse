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
fn validate_create_farm_data_should_work() {
    assert_ok!(LiquidityMining::validate_create_global_farm_data(
        1_000_000,
        100,
        1,
        Perquintill::from_percent(50),
        5_000,
        One::one(),
    ));

    assert_ok!(LiquidityMining::validate_create_global_farm_data(
        9_999_000_000_000,
        2_000_000,
        500,
        Perquintill::from_percent(100),
        crate::MIN_DEPOSIT,
        One::one(),
    ));

    assert_ok!(LiquidityMining::validate_create_global_farm_data(
        10_000_000,
        101,
        16_986_741,
        Perquintill::from_perthousand(1),
        1_000_000_000_000_000,
        One::one(),
    ));
}

#[test]
fn validate_create_farm_data_should_not_work() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            LiquidityMining::validate_create_global_farm_data(
                999_999,
                100,
                1,
                Perquintill::from_percent(50),
                10_000,
                One::one()
            ),
            Error::<Test, Instance1>::InvalidTotalRewards
        );

        assert_noop!(
            LiquidityMining::validate_create_global_farm_data(
                9,
                100,
                1,
                Perquintill::from_percent(50),
                1_500,
                One::one()
            ),
            Error::<Test, Instance1>::InvalidTotalRewards
        );

        assert_noop!(
            LiquidityMining::validate_create_global_farm_data(
                0,
                100,
                1,
                Perquintill::from_percent(50),
                1_000,
                One::one()
            ),
            Error::<Test, Instance1>::InvalidTotalRewards
        );

        assert_noop!(
            LiquidityMining::validate_create_global_farm_data(
                1_000_000,
                99,
                1,
                Perquintill::from_percent(50),
                2_000,
                One::one()
            ),
            Error::<Test, Instance1>::InvalidPlannedYieldingPeriods
        );

        assert_noop!(
            LiquidityMining::validate_create_global_farm_data(
                1_000_000,
                0,
                1,
                Perquintill::from_percent(50),
                3_000,
                One::one()
            ),
            Error::<Test, Instance1>::InvalidPlannedYieldingPeriods
        );

        assert_noop!(
            LiquidityMining::validate_create_global_farm_data(
                1_000_000,
                87,
                1,
                Perquintill::from_percent(50),
                4_000,
                One::one()
            ),
            Error::<Test, Instance1>::InvalidPlannedYieldingPeriods
        );

        assert_noop!(
            LiquidityMining::validate_create_global_farm_data(
                1_000_000,
                100,
                0,
                Perquintill::from_percent(50),
                4_000,
                One::one()
            ),
            Error::<Test, Instance1>::InvalidBlocksPerPeriod
        );

        assert_noop!(
            LiquidityMining::validate_create_global_farm_data(
                1_000_000,
                100,
                10,
                Perquintill::from_percent(0),
                10_000,
                One::one()
            ),
            Error::<Test, Instance1>::InvalidYieldPerPeriod
        );

        assert_noop!(
            LiquidityMining::validate_create_global_farm_data(
                10_000_000,
                101,
                16_986_741,
                Perquintill::from_perthousand(1),
                crate::MIN_DEPOSIT - 1,
                One::one()
            ),
            Error::<Test, Instance1>::InvalidMinDeposit
        );

        assert_noop!(
            LiquidityMining::validate_create_global_farm_data(
                10_000_000,
                101,
                16_986_741,
                Perquintill::from_perthousand(1),
                10_000,
                Zero::zero()
            ),
            Error::<Test, Instance1>::InvalidPriceAdjustment
        );
    });
}
#[test]
fn get_period_number_should_work() {
    let block_num: BlockNumber = 1_u64;
    let blocks_per_period = 1;
    pretty_assertions::assert_eq!(
        LiquidityMining::get_period_number(block_num, blocks_per_period).unwrap(),
        1
    );

    let block_num: BlockNumber = 1_000_u64;
    let blocks_per_period = 1;
    pretty_assertions::assert_eq!(
        LiquidityMining::get_period_number(block_num, blocks_per_period).unwrap(),
        1_000
    );

    let block_num: BlockNumber = 23_u64;
    let blocks_per_period = 15;
    pretty_assertions::assert_eq!(
        LiquidityMining::get_period_number(block_num, blocks_per_period).unwrap(),
        1
    );

    let block_num: BlockNumber = 843_712_398_u64;
    let blocks_per_period = 13_412_341;
    pretty_assertions::assert_eq!(
        LiquidityMining::get_period_number(block_num, blocks_per_period).unwrap(),
        62
    );

    let block_num: BlockNumber = 843_u64;
    let blocks_per_period = 2_000;
    pretty_assertions::assert_eq!(
        LiquidityMining::get_period_number(block_num, blocks_per_period).unwrap(),
        0
    );

    let block_num: BlockNumber = 10_u64;
    let blocks_per_period = 10;
    pretty_assertions::assert_eq!(
        LiquidityMining::get_period_number(block_num, blocks_per_period).unwrap(),
        1
    );
}

#[test]
fn get_period_number_should_not_work_when_block_per_period_is_zero() {
    new_test_ext().execute_with(|| {
        let block_num: BlockNumber = 10_u64;
        assert_noop!(
            LiquidityMining::get_period_number(block_num, 0),
            Error::InconsistentState(InconsistentStateError::InvalidPeriod)
        );
    });
}

#[test]
fn get_loyalty_multiplier_should_work() {
    let loyalty_curve_1 = LoyaltyCurve::default();
    let loyalty_curve_2 = LoyaltyCurve {
        initial_reward_percentage: FixedU128::from(1),
        scale_coef: 50,
    };
    let loyalty_curve_3 = LoyaltyCurve {
        initial_reward_percentage: FixedU128::from_inner(123_580_000_000_000_000), // 0.12358
        scale_coef: 23,
    };
    let loyalty_curve_4 = LoyaltyCurve {
        initial_reward_percentage: FixedU128::from_inner(0), // 0.12358
        scale_coef: 15,
    };

    let testing_values = vec![
        (
            0,
            FixedU128::from_float(0.5_f64),
            FixedU128::from_float(1_f64),
            FixedU128::from_float(0.12358_f64),
            FixedU128::from_float(0_f64),
        ),
        (
            1,
            FixedU128::from_float(0.504950495_f64),
            FixedU128::from_float(1_f64),
            FixedU128::from_float(0.1600975_f64),
            FixedU128::from_float(0.0625_f64),
        ),
        (
            4,
            FixedU128::from_float(0.5192307692_f64),
            FixedU128::from_float(1_f64),
            FixedU128::from_float(0.25342_f64),
            FixedU128::from_float(0.2105263158_f64),
        ),
        (
            130,
            FixedU128::from_float(0.7826086957_f64),
            FixedU128::from_float(1_f64),
            FixedU128::from_float(0.8682505882_f64),
            FixedU128::from_float(0.8965517241_f64),
        ),
        (
            150,
            FixedU128::from_float(0.8_f64),
            FixedU128::from_float(1_f64),
            FixedU128::from_float(0.8834817341_f64),
            FixedU128::from_float(0.9090909091_f64),
        ),
        (
            180,
            FixedU128::from_float(0.8214285714_f64),
            FixedU128::from_float(1_f64),
            FixedU128::from_float(0.9007011823_f64),
            FixedU128::from_float(0.9230769231_f64),
        ),
        (
            240,
            FixedU128::from_float(0.8529411765_f64),
            FixedU128::from_float(1_f64),
            FixedU128::from_float(0.9233549049_f64),
            FixedU128::from_float(0.9411764706_f64),
        ),
        (
            270,
            FixedU128::from_float(0.8648648649_f64),
            FixedU128::from_float(1_f64),
            FixedU128::from_float(0.9312025256_f64),
            FixedU128::from_float(0.9473684211_f64),
        ),
        (
            280,
            FixedU128::from_float(0.8684210526_f64),
            FixedU128::from_float(1_f64),
            FixedU128::from_float(0.9334730693_f64),
            FixedU128::from_float(0.9491525424_f64),
        ),
        (
            320,
            FixedU128::from_float(0.880952381_f64),
            FixedU128::from_float(1_f64),
            FixedU128::from_float(0.941231312_f64),
            FixedU128::from_float(0.9552238806_f64),
        ),
        (
            380,
            FixedU128::from_float(0.8958333333_f64),
            FixedU128::from_float(1_f64),
            FixedU128::from_float(0.9499809926_f64),
            FixedU128::from_float(0.9620253165_f64),
        ),
        (
            390,
            FixedU128::from_float(0.8979591837_f64),
            FixedU128::from_float(1_f64),
            FixedU128::from_float(0.9511921065_f64),
            FixedU128::from_float(0.962962963_f64),
        ),
        (
            4000,
            FixedU128::from_float(0.987804878_f64),
            FixedU128::from_float(1_f64),
            FixedU128::from_float(0.994989396_f64),
            FixedU128::from_float(0.99626401_f64),
        ),
        (
            4400,
            FixedU128::from_float(0.9888888889_f64),
            FixedU128::from_float(1_f64),
            FixedU128::from_float(0.9954425367_f64),
            FixedU128::from_float(0.9966024915_f64),
        ),
        (
            4700,
            FixedU128::from_float(0.9895833333_f64),
            FixedU128::from_float(1_f64),
            FixedU128::from_float(0.995732022_f64),
            FixedU128::from_float(0.9968186638_f64),
        ),
    ];

    //Special case: loyalty curve is None
    pretty_assertions::assert_eq!(
        LiquidityMining::get_loyalty_multiplier(10, None).unwrap(),
        FixedU128::one()
    );

    let precission_delta = FixedU128::from_inner(100_000_000); //0.000_000_000_1
    for (periods, expected_multiplier_1, expected_multiplier_2, expected_multiplier_3, expected_multiplier_4) in
        testing_values.iter()
    {
        //1-th curve test
        assert!(is_approx_eq_fixedu128(
            LiquidityMining::get_loyalty_multiplier(*periods, Some(loyalty_curve_1.clone())).unwrap(),
            *expected_multiplier_1,
            precission_delta
        ));

        //2-nd curve test
        assert!(is_approx_eq_fixedu128(
            LiquidityMining::get_loyalty_multiplier(*periods, Some(loyalty_curve_2.clone())).unwrap(),
            *expected_multiplier_2,
            precission_delta
        ));

        //3-rd curve test
        assert!(is_approx_eq_fixedu128(
            LiquidityMining::get_loyalty_multiplier(*periods, Some(loyalty_curve_3.clone())).unwrap(),
            *expected_multiplier_3,
            precission_delta
        ));

        //-4th curve test
        assert!(is_approx_eq_fixedu128(
            LiquidityMining::get_loyalty_multiplier(*periods, Some(loyalty_curve_4.clone())).unwrap(),
            *expected_multiplier_4,
            precission_delta
        ));
    }
}

#[test]
fn update_global_farm_should_work() {
    let testing_values = vec![
        (
            26_u64,
            2_501_944_769_u128,
            FixedU128::from_inner(259_000_000_000_000_000_000_u128),
            HDX,
            BSX_FARM,
            0_u128,
            206_u64,
            65_192_006_u128,
            55_563_662_u128,
            FixedU128::from_inner(259_000_000_000_000_000_000_u128),
            55_563_662_u128,
        ),
        (
            188_u64,
            33_769_603_u128,
            FixedU128::from_inner(1_148_000_000_000_000_000_000_u128),
            BSX,
            ACA_FARM,
            30_080_406_306_u128,
            259_u64,
            1_548_635_u128,
            56_710_169_u128,
            FixedU128::from_inner(1_151_255_978_016_679_674_913_u128),
            166_663_254_u128,
        ),
        (
            195_u64,
            26_098_384_286_056_u128,
            FixedU128::from_inner(523_000_000_000_000_000_000_u128),
            ACA,
            KSM_FARM,
            32_055_u128,
            326_u64,
            1_712_797_u128,
            61_455_483_u128,
            FixedU128::from_inner(523_000_000_001_189_920_405_u128),
            61_486_538_u128,
        ),
        (
            181_u64,
            9_894_090_144_u128,
            FixedU128::from_inner(317_000_000_000_000_000_000_u128),
            KSM,
            ACA_FARM,
            36_806_694_280_u128,
            1856_u64,
            19_009_156_u128,
            52_711_084_u128,
            FixedU128::from_inner(320_218_116_657_175_263_350_u128),
            31_893_047_384_u128,
        ),
        (
            196_u64,
            26_886_423_482_043_u128,
            FixedU128::from_inner(596_000_000_000_000_000_000_u128),
            ACA,
            ACA_FARM,
            30_560_755_872_u128,
            954_u64,
            78_355_u128,
            34_013_971_u128,
            FixedU128::from_inner(596_000_002_209_036_469_267_u128),
            93_407_061_u128,
        ),
        (
            68_u64,
            1_138_057_342_u128,
            FixedU128::from_inner(4_000_000_000_000_000_000_u128),
            ACA,
            ACA_FARM,
            38_398_062_768_u128,
            161_u64,
            55_309_798_233_u128,
            71_071_995_u128,
            FixedU128::from_inner(37_740_006_193_817_956_143_u128),
            38_469_133_763_u128,
        ),
        (
            161_u64,
            24_495_534_649_923_u128,
            FixedU128::from_inner(213_000_000_000_000_000_000_u128),
            KSM,
            ACA_FARM,
            11_116_735_745_u128,
            448_u64,
            326_u128,
            85_963_452_u128,
            FixedU128::from_inner(213_000_000_003_819_553_291_u128),
            86_057_014_u128,
        ),
        (
            27_u64,
            22_108_444_u128,
            FixedU128::from_inner(970_000_000_000_000_000_000_u128),
            KSM,
            ACA_FARM,
            8_572_779_460_u128,
            132_u64,
            1_874_081_u128,
            43_974_403_u128,
            FixedU128::from_inner(978_900_603_995_468_880_577_u128),
            240_752_908_u128,
        ),
        (
            97_u64,
            1_593_208_u128,
            FixedU128::from_inner(6_000_000_000_000_000_000_u128),
            HDX,
            BSX_FARM,
            18_440_792_496_u128,
            146_u64,
            741_803_u128,
            14_437_690_u128,
            FixedU128::from_inner(28_814_564_702_160_672_052_u128),
            50_786_037_u128,
        ),
        (
            154_u64,
            27_279_119_649_838_u128,
            FixedU128::from_inner(713_000_000_000_000_000_000_u128),
            BSX,
            KSM_FARM,
            28_318_566_664_u128,
            202_u64,
            508_869_u128,
            7_533_987_u128,
            FixedU128::from_inner(713_000_000_895_399_569_837_u128),
            31_959_699_u128,
        ),
        (
            104_u64,
            20_462_312_838_954_u128,
            FixedU128::from_inner(833_000_000_000_000_000_000_u128),
            BSX,
            ACA_FARM,
            3_852_003_u128,
            131_u64,
            1_081_636_u128,
            75_149_021_u128,
            FixedU128::from_inner(833_000_000_188_199_791_016_u128),
            79_000_024_u128,
        ),
        (
            90_u64,
            37_650_830_596_054_u128,
            FixedU128::from_inner(586_000_000_000_000_000_000_u128),
            HDX,
            ACA_FARM,
            27_990_338_179_u128,
            110_u64,
            758_482_u128,
            36_765_518_u128,
            FixedU128::from_inner(586_000_000_402_903_196_552_u128),
            51_935_158_u128,
        ),
        (
            198_u64,
            318_777_215_u128,
            FixedU128::from_inner(251_000_000_000_000_000_000_u128),
            ACA,
            ACA_FARM,
            3_615_346_492_u128,
            582_u64,
            69_329_u128,
            12_876_432_u128,
            FixedU128::from_inner(251_083_513_923_666_093_889_u128),
            39_498_768_u128,
        ),
        (
            29_u64,
            33_478_250_u128,
            FixedU128::from_inner(77_000_000_000_000_000_000_u128),
            BSX,
            BSX_FARM,
            39_174_031_245_u128,
            100_u64,
            1_845_620_u128,
            26_611_087_u128,
            FixedU128::from_inner(80_914_153_816_283_706_585_u128),
            157_650_107_u128,
        ),
        (
            91_u64,
            393_922_835_172_u128,
            FixedU128::from_inner(2_491_000_000_000_000_000_000_u128),
            ACA,
            ACA_FARM,
            63_486_975_129_400_u128,
            260_u64,
            109_118_678_233_u128,
            85_100_506_u128,
            FixedU128::from_inner(2_537_813_880_726_983_020_710_u128),
            18_441_141_721_883_u128,
        ),
        (
            67_u64,
            1_126_422_u128,
            FixedU128::from_inner(295_000_000_000_000_000_000_u128),
            HDX,
            BSX_FARM,
            7_492_177_402_u128,
            229_u64,
            1_227_791_u128,
            35_844_776_u128,
            FixedU128::from_inner(471_578_708_512_440_275_491_u128),
            234_746_918_u128,
        ),
        (
            168_u64,
            28_351_324_279_041_u128,
            FixedU128::from_inner(450_000_000_000_000_000_000_u128),
            ACA,
            BSX_FARM,
            38_796_364_068_u128,
            361_u64,
            1_015_284_u128,
            35_695_723_u128,
            FixedU128::from_inner(450_000_006_911_487_099_206_u128),
            231_645_535_u128,
        ),
        (
            3_u64,
            17_631_376_575_792_u128,
            FixedU128::from_inner(82_000_000_000_000_000_000_u128),
            HDX,
            ACA_FARM,
            20_473_946_880_u128,
            52_u64,
            1_836_345_u128,
            93_293_564_u128,
            FixedU128::from_inner(82_000_005_103_453_188_308_u128),
            183_274_469_u128,
        ),
        (
            49_u64,
            94_059_u128,
            FixedU128::from_inner(81_000_000_000_000_000_000_u128),
            HDX,
            KSM_FARM,
            11_126_653_978_u128,
            132_u64,
            1_672_829_u128,
            75_841_904_u128,
            FixedU128::from_inner(1_557_145_897_787_558_872_622_u128),
            214_686_711_u128,
        ),
        (
            38_u64,
            14_085_u128,
            FixedU128::from_inner(266_000_000_000_000_000_000_u128),
            KSM,
            KSM_FARM,
            36_115_448_964_u128,
            400000_u64,
            886_865_u128,
            52_402_278_u128,
            FixedU128::from_inner(2_564_373_061_696_840_610_578_629_u128),
            36_167_850_242_u128,
        ),
        (
            158_u64,
            762_784_u128,
            FixedU128::from_inner(129_000_000_000_000_000_000_u128),
            BSX,
            KSM_FARM,
            21_814_882_774_u128,
            158_u64,
            789_730_u128,
            86_085_676_u128,
            FixedU128::from_inner(129_000_000_000_000_000_000_u128),
            86_085_676_u128,
        ),
    ];

    for (
        updated_at,
        total_shares_z,
        accumulated_rpz,
        reward_currency,
        id,
        rewards_left_to_distribute,
        current_period,
        reward_per_period,
        accumulated_rewards,
        expected_accumulated_rpz,
        expected_accumulated_rewards,
    ) in testing_values.iter()
    {
        let yield_per_period = Perquintill::from_percent(50);
        let planned_yielding_periods = 100;
        let blocks_per_period = 0;
        let owner = ALICE;
        let incentivized_token = BSX;
        let max_reward_per_period = 10_000_u128;

        let mut global_farm = GlobalFarmData::new(
            *id,
            *updated_at,
            *reward_currency,
            yield_per_period,
            planned_yielding_periods,
            blocks_per_period,
            owner,
            incentivized_token,
            max_reward_per_period,
            10,
            One::one(),
        );

        global_farm.total_shares_z = *total_shares_z;
        global_farm.accumulated_rewards = *accumulated_rewards;
        global_farm.accumulated_rpz = *accumulated_rpz;
        global_farm.paid_accumulated_rewards = 10;

        new_test_ext().execute_with(|| {
            //Add farm's account to whitelist
            let farm_account_id = LiquidityMining::farm_account_id(*id).unwrap();
            Whitelist::add_account(&farm_account_id).unwrap();

            Tokens::transfer(
                Origin::signed(TREASURY),
                farm_account_id,
                *reward_currency,
                *rewards_left_to_distribute,
            )
            .unwrap();

            pretty_assertions::assert_eq!(
                Tokens::free_balance(*reward_currency, &farm_account_id),
                *rewards_left_to_distribute
            );

            let r = with_transaction(|| {
                TransactionOutcome::Commit(LiquidityMining::update_global_farm(
                    &mut global_farm,
                    *current_period,
                    *reward_per_period,
                ))
            })
            .unwrap();

            if r.is_zero() && updated_at != current_period {
                frame_system::Pallet::<Test>::assert_has_event(mock::Event::LiquidityMining(
                    Event::AllRewardsDistributed { global_farm_id: *id },
                ));
            }

            let mut expected_global_farm = GlobalFarmData::new(
                *id,
                *current_period,
                *reward_currency,
                yield_per_period,
                planned_yielding_periods,
                blocks_per_period,
                owner,
                incentivized_token,
                max_reward_per_period,
                10,
                One::one(),
            );

            expected_global_farm.total_shares_z = *total_shares_z;
            expected_global_farm.paid_accumulated_rewards = 10;
            expected_global_farm.accumulated_rpz = *expected_accumulated_rpz;
            expected_global_farm.accumulated_rewards = *expected_accumulated_rewards;

            pretty_assertions::assert_eq!(global_farm, expected_global_farm);

            if updated_at != current_period {
                frame_system::Pallet::<Test>::assert_has_event(mock::Event::LiquidityMining(
                    Event::GlobalFarmAccRPZUpdated {
                        global_farm_id: *id,
                        accumulated_rpz: *expected_accumulated_rpz,
                        total_shares_z: *total_shares_z,
                    },
                ));
            }
        });
    }
}

#[test]
fn claim_from_global_farm_should_work() {
    let testing_values = vec![
        (
            26_u64,
            2501944769_u128,
            259_u128,
            299_u128,
            HDX,
            5556613662_u128,
            0_u128,
            55563662_u128,
            2222546480_u128,
            299_u128,
            3334067182_u128,
            2222546480_u128,
        ),
        (
            188_u64,
            33769603_u128,
            1148_u128,
            1151_u128,
            BSX,
            166663254_u128,
            30080406306_u128,
            5671016_u128,
            17013048_u128,
            1151_u128,
            149650206_u128,
            30097419354_u128,
        ),
        (
            195_u64,
            26098384286056_u128,
            523_u128,
            823_u128,
            ACA,
            61456483_u128,
            32055_u128,
            61428_u128,
            18428400_u128,
            823_u128,
            43028083_u128,
            18460455_u128,
        ),
        (
            181_u64,
            9894090144_u128,
            317_u128,
            320_u128,
            KSM,
            31893047384_u128,
            36806694280_u128,
            527114_u128,
            1581342_u128,
            320_u128,
            31891466042_u128,
            36808275622_u128,
        ),
        (
            196_u64,
            26886423482043_u128,
            596_u128,
            5684_u128,
            ACA,
            93407061_u128,
            30560755872_u128,
            3011_u128,
            15319968_u128,
            5684_u128,
            78087093_u128,
            30576075840_u128,
        ),
        (
            68_u64,
            1138057342_u128,
            4_u128,
            37_u128,
            ACA,
            38469134763_u128,
            38398062768_u128,
            71071995_u128,
            2345375835_u128,
            37_u128,
            36123758928_u128,
            40743438603_u128,
        ),
        (
            161_u64,
            24495534649923_u128,
            213_u128,
            678_u128,
            KSM,
            86057014_u128,
            11116735745_u128,
            85452_u128,
            39735180_u128,
            678_u128,
            46321834_u128,
            11156470925_u128,
        ),
        (
            27_u64,
            22108444_u128,
            970_u128,
            978_u128,
            KSM,
            240752908_u128,
            8572779460_u128,
            474403_u128,
            3795224_u128,
            978_u128,
            236957684_u128,
            8576574684_u128,
        ),
        (
            97_u64,
            1593208_u128,
            6_u128,
            28_u128,
            HDX,
            50786037_u128,
            18440792496_u128,
            147690_u128,
            3249180_u128,
            28_u128,
            47536857_u128,
            18444041676_u128,
        ),
        (
            154_u64,
            27279119649838_u128,
            713_u128,
            876_u128,
            BSX,
            319959699_u128,
            28318566664_u128,
            75987_u128,
            12385881_u128,
            876_u128,
            307573818_u128,
            28330952545_u128,
        ),
        (
            104_u64,
            20462312838954_u128,
            833_u128,
            8373_u128,
            BSX,
            790051024_u128,
            3852003_u128,
            7521_u128,
            56708340_u128,
            8373_u128,
            733342684_u128,
            60560343_u128,
        ),
        (
            90_u64,
            37650830596054_u128,
            586_u128,
            5886_u128,
            HDX,
            519356158_u128,
            27990338179_u128,
            318_u128,
            1685400_u128,
            5886_u128,
            517670758_u128,
            27992023579_u128,
        ),
        (
            198_u64,
            318777215_u128,
            251_u128,
            2591_u128,
            ACA,
            3949876895_u128,
            3615346492_u128,
            28732_u128,
            67232880_u128,
            2591_u128,
            3882644015_u128,
            3682579372_u128,
        ),
        (
            29_u64,
            33478250_u128,
            77_u128,
            80_u128,
            BSX,
            157650107_u128,
            39174031245_u128,
            26611087_u128,
            79833261_u128,
            80_u128,
            77816846_u128,
            39253864506_u128,
        ),
        (
            91_u64,
            393922835172_u128,
            2491_u128,
            2537_u128,
            ACA,
            18441141721883_u128,
            63486975129400_u128,
            85100506_u128,
            3914623276_u128,
            2537_u128,
            18437227098607_u128,
            63490889752676_u128,
        ),
        (
            67_u64,
            1126422_u128,
            295_u128,
            471_u128,
            HDX,
            234746918_u128,
            7492177402_u128,
            358776_u128,
            63144576_u128,
            471_u128,
            171602342_u128,
            7555321978_u128,
        ),
        (
            168_u64,
            28351324279041_u128,
            450_u128,
            952_u128,
            ACA,
            231645535_u128,
            38796364068_u128,
            356723_u128,
            179074946_u128,
            952_u128,
            52570589_u128,
            38975439014_u128,
        ),
        (
            3_u64,
            17631376575792_u128,
            82_u128,
            357_u128,
            HDX,
            1832794469_u128,
            20473946880_u128,
            932564_u128,
            256455100_u128,
            357_u128,
            1576339369_u128,
            20730401980_u128,
        ),
        (
            49_u64,
            94059_u128,
            81_u128,
            1557_u128,
            HDX,
            21495686711_u128,
            11126653978_u128,
            758404_u128,
            1119404304_u128,
            1557_u128,
            20376282407_u128,
            12246058282_u128,
        ),
        (
            38_u64,
            14085_u128,
            266_u128,
            2564373_u128,
            KSM,
            36167851242_u128,
            36115448964_u128,
            5278_u128,
            13533356746_u128,
            2564373_u128,
            22634494496_u128,
            49648805710_u128,
        ),
        (
            158_u64,
            762784_u128,
            129_u128,
            129_u128,
            BSX,
            86085676_u128,
            21814882774_u128,
            86085676_u128,
            0_u128,
            129_u128,
            86085676_u128,
            21814882774_u128,
        ),
    ];

    for (
        updated_at,
        total_shares_z,
        yield_farm_accumulated_rpz,
        global_farm_accumuated_rpz,
        reward_currency,
        accumulated_rewards,
        paid_accumulated_rewards,
        yield_farm_stake_in_global_farm,
        expected_rewards_from_global_farm,
        expected_yield_farm_accumulated_rpz,
        expected_global_farm_accumulated_rewards,
        expected_global_farm_pair_accumulated_rewards,
    ) in testing_values.iter()
    {
        let global_farm_id = 1;
        let yield_farm_id = 2;
        let yield_per_period = Perquintill::from_percent(50);
        let planned_yielding_periods = 100;
        let blocks_per_period = 1;
        let owner = ALICE;
        let incentivized_token = BSX;
        let max_reward_per_period = Balance::from(10_000_u32);

        let mut global_farm = GlobalFarmData::new(
            global_farm_id,
            *updated_at,
            *reward_currency,
            yield_per_period,
            planned_yielding_periods,
            blocks_per_period,
            owner,
            incentivized_token,
            max_reward_per_period,
            10,
            One::one(),
        );

        global_farm.total_shares_z = *total_shares_z;
        global_farm.accumulated_rpz = FixedU128::from(*global_farm_accumuated_rpz);
        global_farm.accumulated_rewards = *accumulated_rewards;
        global_farm.paid_accumulated_rewards = *paid_accumulated_rewards;

        let mut yield_farm = YieldFarmData::new(yield_farm_id, *updated_at, None, FixedU128::from(10_u128));
        yield_farm.accumulated_rpz = FixedU128::from(*yield_farm_accumulated_rpz);

        pretty_assertions::assert_eq!(
            LiquidityMining::claim_from_global_farm(
                &mut global_farm,
                &mut yield_farm,
                *yield_farm_stake_in_global_farm
            )
            .unwrap(),
            *expected_rewards_from_global_farm
        );

        let mut expected_global_farm = GlobalFarmData::new(
            global_farm_id,
            *updated_at,
            *reward_currency,
            yield_per_period,
            planned_yielding_periods,
            blocks_per_period,
            owner,
            incentivized_token,
            max_reward_per_period,
            10,
            One::one(),
        );

        expected_global_farm.total_shares_z = *total_shares_z;
        expected_global_farm.accumulated_rpz = FixedU128::from(*global_farm_accumuated_rpz);
        expected_global_farm.accumulated_rewards = *expected_global_farm_accumulated_rewards;
        expected_global_farm.paid_accumulated_rewards = *expected_global_farm_pair_accumulated_rewards;

        pretty_assertions::assert_eq!(global_farm, expected_global_farm);

        let mut expected_yield_farm = YieldFarmData::new(yield_farm_id, *updated_at, None, FixedU128::from(10_u128));
        expected_yield_farm.accumulated_rpz = FixedU128::from(*expected_yield_farm_accumulated_rpz);

        pretty_assertions::assert_eq!(yield_farm, expected_yield_farm);
    }
}

#[test]
fn update_yield_farm_should_work() {
    let testing_values = vec![
        (
            BSX_FARM,
            BSX_DOT_YIELD_FARM_ID,
            26_u64,
            206_u64,
            299_u128,
            0_u128,
            2_222_546_480_u128,
            BSX,
            299_000_000_000_000_000_000_u128,
            26_u64,
            0_u128,
        ),
        (
            BSX_FARM,
            BSX_ACA_YIELD_FARM_ID,
            188_u64,
            259_u64,
            1_151_u128,
            33_769_603_u128,
            170_130_593_048_u128,
            BSX,
            6_188_980_252_477_353_672_176_u128,
            259_u64,
            170_130_593_048_u128,
        ),
        (
            BSX_FARM,
            BSX_KSM_YIELD_FARM_ID,
            195_u64,
            326_u64,
            823_u128,
            2_604_286_056_u128,
            8_414_312_431_200_u128,
            BSX,
            4_053_947_849_148_258_082_137_u128,
            326_u64,
            8_414_312_431_200_u128,
        ),
        (
            BSX_FARM,
            BSX_KSM_YIELD_FARM_ID,
            181_u64,
            1856_u64,
            320_u128,
            8_940_144_u128,
            190_581_342_u128,
            BSX,
            341_317_480_121_125_565_762_u128,
            1856_u64,
            190_581_342_u128,
        ),
        (
            BSX_FARM,
            BSX_ACA_YIELD_FARM_ID,
            196_u64,
            954_u64,
            5_684_u128,
            28_2043_u128,
            15_319_968_u128,
            BSX,
            5_738_317_845_151_271_260_056_u128,
            954_u64,
            15_319_968_u128,
        ),
        (
            BSX_FARM,
            BSX_DOT_YIELD_FARM_ID,
            68_u64,
            161_u64,
            37_u128,
            1_138_057_342_u128,
            2_345_375_835_u128,
            BSX,
            39_060_859_104_760_294_231_u128,
            161_u64,
            2_345_375_835_u128,
        ),
        (
            BSX_FARM,
            BSX_ACA_YIELD_FARM_ID,
            161_u64,
            448_u64,
            678_u128,
            49_923_u128,
            39_735_180_u128,
            BSX,
            1_473_929_331_170_001_802_776_u128,
            448_u64,
            39_735_180_u128,
        ),
        (
            BSX_FARM,
            BSX_DOT_YIELD_FARM_ID,
            27_u64,
            132_u64,
            978_u128,
            2_444_u128,
            3_795_224_u128,
            BSX,
            2_530_873_977_086_743_044_189_u128,
            132_u64,
            3_795_224_u128,
        ),
        (
            BSX_FARM,
            BSX_KSM_YIELD_FARM_ID,
            97_u64,
            146_u64,
            28_u128,
            1_593_208_u128,
            3_249_180_u128,
            BSX,
            30_039_394_730_631_530_848_u128,
            146_u64,
            3_249_180_u128,
        ),
        (
            BSX_FARM,
            BSX_ACA_YIELD_FARM_ID,
            154_u64,
            202_u64,
            876_u128,
            9_838_u128,
            12_385_881_u128,
            BSX,
            2_134_983_634_885_139_255_946_u128,
            202_u64,
            12_385_881_u128,
        ),
        (
            BSX_FARM,
            BSX_DOT_YIELD_FARM_ID,
            104_u64,
            131_u64,
            8_373_u128,
            2_046_838_954_u128,
            56_708_340_909_u128,
            BSX,
            8_400_705_326_204_672_182_528_u128,
            131_u64,
            56_708_340_909_u128,
        ),
        (
            BSX_FARM,
            BSX_KSM_YIELD_FARM_ID,
            90_u64,
            110_u64,
            5_886_u128,
            596_054_u128,
            1_685_400_u128,
            BSX,
            5_888_827_596_157_395_135_340_u128,
            110_u64,
            1_685_400_u128,
        ),
        (
            BSX_FARM,
            BSX_DOT_YIELD_FARM_ID,
            198_u64,
            582_u64,
            2591_u128,
            377_215_u128,
            67_232_880_u128,
            BSX,
            2_769_234_905_822_939_172_620_u128,
            582_u64,
            67_232_880_u128,
        ),
        (
            BSX_FARM,
            BSX_KSM_YIELD_FARM_ID,
            29_u64,
            100_u64,
            80_u128,
            8_250_u128,
            79_833_261_u128,
            BSX,
            9_756_758_909_090_909_090_909_u128,
            100_u64,
            79_833_261_u128,
        ),
        (
            BSX_FARM,
            BSX_ACA_YIELD_FARM_ID,
            91_u64,
            260_u64,
            2_537_u128,
            35_172_u128,
            3_914_623_276_u128,
            BSX,
            113_836_422_153_986_125_326_964_u128,
            260_u64,
            3_914_623_276_u128,
        ),
        (
            BSX_FARM,
            BSX_ACA_YIELD_FARM_ID,
            67_u64,
            229_u64,
            471_u128,
            1_126_422_u128,
            63_144_576_u128,
            BSX,
            527_057_655_123_923_360_871_u128,
            229_u64,
            63_144_576_u128,
        ),
        (
            BSX_FARM,
            BSX_DOT_YIELD_FARM_ID,
            168_u64,
            361_u64,
            952_u128,
            28_279_041_u128,
            179_074_946_u128,
            BSX,
            958_332_426_407_246_271_187_u128,
            361_u64,
            179_074_946_u128,
        ),
        (
            BSX_FARM,
            BSX_ACA_YIELD_FARM_ID,
            3_u64,
            52_u64,
            357_u128,
            2_u128,
            256_455_100_u128,
            BSX,
            128_227_907_000_000_000_000_000_000_u128,
            52_u64,
            256_455_100_u128,
        ),
        (
            BSX_FARM,
            BSX_KSM_YIELD_FARM_ID,
            49_u64,
            132_u64,
            1_557_u128,
            94_059_u128,
            1_119_404_304_u128,
            BSX,
            13_458_086_594_584_250_310_975_u128,
            132_u64,
            1_119_404_304_u128,
        ),
        (
            BSX_FARM,
            BSX_ACA_YIELD_FARM_ID,
            38_u64,
            38_u64,
            2_564_373_u128,
            14_085_u128,
            13_533_356_746_u128,
            BSX,
            2_564_373_000_000_000_000_000_000_u128,
            38_u64,
            0_u128,
        ),
        (
            BSX_FARM,
            BSX_ACA_YIELD_FARM_ID,
            158_u64,
            159_u64,
            129_u128,
            762784_u128,
            179074933_u128,
            BSX,
            363_764_930_832_319_503_293_u128,
            159_u64,
            179_074_933_u128,
        ),
    ];

    for (
        global_farm_id,
        yield_farm_id,
        yield_farm_updated_at,
        current_period,
        yield_farm_accumulated_rpvs,
        yield_farm_total_valued_shares,
        yield_farm_rewards,
        reward_currency,
        expected_yield_farm_accumulated_rpvs,
        expected_updated_at,
        expected_yield_farm_reward_currency_balance,
    ) in testing_values.iter()
    {
        let owner = ALICE;
        let yield_per_period = Perquintill::from_percent(50);
        let blocks_per_period = BlockNumber::from(1_u32);
        let planned_yielding_periods = 100;
        let incentivized_token = BSX;
        let updated_at = 200_u64;
        let max_reward_per_period = Balance::from(10_000_u32);

        let mut global_farm = GlobalFarmData::<Test, Instance1>::new(
            *global_farm_id,
            updated_at,
            *reward_currency,
            yield_per_period,
            planned_yielding_periods,
            blocks_per_period,
            owner,
            incentivized_token,
            max_reward_per_period,
            10,
            One::one(),
        );

        global_farm.total_shares_z = 1_000_000_u128;
        global_farm.accumulated_rpz = FixedU128::from(200_u128);
        global_farm.accumulated_rewards = 1_000_000_u128;
        global_farm.paid_accumulated_rewards = 1_000_000_u128;

        let mut yield_farm = YieldFarmData {
            id: *yield_farm_id,
            updated_at: *yield_farm_updated_at,
            total_shares: 200_u128,
            total_valued_shares: *yield_farm_total_valued_shares,
            accumulated_rpvs: FixedU128::from(*yield_farm_accumulated_rpvs),
            accumulated_rpz: FixedU128::from(200_u128),
            loyalty_curve: None,
            multiplier: FixedU128::from(10_u128),
            state: FarmState::Active,
            entries_count: 0,
            left_to_distribute: 0,
            _phantom: PhantomData::default(),
        };

        let global_farm_account_id = LiquidityMining::farm_account_id(*global_farm_id).unwrap();
        let pot_account_id = LiquidityMining::pot_account_id().unwrap();

        new_test_ext().execute_with(|| {
            //Arrange
            let _ = Tokens::transfer(
                Origin::signed(TREASURY),
                global_farm_account_id,
                global_farm.reward_currency,
                9_000_000_000_000,
            );
            pretty_assertions::assert_eq!(
                Tokens::free_balance(global_farm.reward_currency, &global_farm_account_id),
                9_000_000_000_000_u128
            );

            //_0 - value before action
            let pot_balance_0 = 9_000_000_000_000;
            let _ = Tokens::transfer(
                Origin::signed(TREASURY),
                pot_account_id,
                global_farm.reward_currency,
                pot_balance_0,
            );

            //Act
            assert_ok!(LiquidityMining::update_yield_farm(
                &mut yield_farm,
                *yield_farm_rewards,
                *current_period,
                *global_farm_id,
            ));

            //Assert
            let mut rhs_global_farm = GlobalFarmData::new(
                *global_farm_id,
                updated_at,
                *reward_currency,
                yield_per_period,
                planned_yielding_periods,
                blocks_per_period,
                owner,
                incentivized_token,
                max_reward_per_period,
                10,
                One::one(),
            );

            rhs_global_farm.updated_at = 200_u64;
            rhs_global_farm.total_shares_z = 1_000_000_u128;
            rhs_global_farm.accumulated_rpz = FixedU128::from(200_u128);
            rhs_global_farm.accumulated_rewards = 1_000_000_u128;
            rhs_global_farm.paid_accumulated_rewards = 1_000_000_u128;

            pretty_assertions::assert_eq!(global_farm, rhs_global_farm);

            pretty_assertions::assert_eq!(
                yield_farm,
                YieldFarmData {
                    id: *yield_farm_id,
                    updated_at: *expected_updated_at,
                    total_shares: 200_u128,
                    total_valued_shares: *yield_farm_total_valued_shares,
                    accumulated_rpvs: FixedU128::from_inner(*expected_yield_farm_accumulated_rpvs),
                    accumulated_rpz: FixedU128::from(200_u128),
                    loyalty_curve: None,
                    multiplier: FixedU128::from(10_u128),
                    state: FarmState::Active,
                    entries_count: 0,
                    left_to_distribute: *expected_yield_farm_reward_currency_balance,
                    _phantom: PhantomData::default(),
                }
            );

            //yield-farm's rewards are not transferred from top so it's balance should not change
            pretty_assertions::assert_eq!(
                Tokens::free_balance(global_farm.reward_currency, &pot_account_id),
                pot_balance_0
            );

            if current_period != yield_farm_updated_at && !yield_farm_total_valued_shares.is_zero() {
                frame_system::Pallet::<Test>::assert_has_event(mock::Event::LiquidityMining(
                    Event::YieldFarmAccRPVSUpdated {
                        global_farm_id: global_farm.id,
                        yield_farm_id: *yield_farm_id,
                        accumulated_rpvs: FixedU128::from_inner(*expected_yield_farm_accumulated_rpvs),
                        total_valued_shares: yield_farm.total_valued_shares,
                    },
                ));
            }
        });
    }
}

#[test]
fn get_next_farm_id_should_work() {
    new_test_ext().execute_with(|| {
        pretty_assertions::assert_eq!(LiquidityMining::get_next_farm_id().unwrap(), 1);
        pretty_assertions::assert_eq!(LiquidityMining::last_farm_id(), 1);

        pretty_assertions::assert_eq!(LiquidityMining::get_next_farm_id().unwrap(), 2);
        pretty_assertions::assert_eq!(LiquidityMining::last_farm_id(), 2);

        pretty_assertions::assert_eq!(LiquidityMining::get_next_farm_id().unwrap(), 3);
        pretty_assertions::assert_eq!(LiquidityMining::last_farm_id(), 3);

        pretty_assertions::assert_eq!(LiquidityMining::get_next_farm_id().unwrap(), 4);
        pretty_assertions::assert_eq!(LiquidityMining::last_farm_id(), 4);
    });
}

#[test]
fn farm_account_id_should_work() {
    let ids: Vec<FarmId> = vec![1, 100, 543, u32::max_value()];

    for id in ids {
        assert_ok!(LiquidityMining::farm_account_id(id));
    }
}

#[test]
fn farm_account_id_should_fail_when_farm_id_is_zero() {
    let ids: Vec<FarmId> = vec![0];
    new_test_ext().execute_with(|| {
        for id in ids {
            assert_noop!(
                LiquidityMining::farm_account_id(id),
                Error::<Test, Instance1>::InconsistentState(InconsistentStateError::ZeroFarmId)
            );
        }
    });
}

#[test]
fn get_next_deposit_id_should_work() {
    new_test_ext().execute_with(|| {
        let test_data = vec![1, 2, 3, 4, 5];

        for expected_deposit_id in test_data {
            pretty_assertions::assert_eq!(LiquidityMining::get_next_deposit_id().unwrap(), expected_deposit_id);
        }
    });
}

#[test]
fn maybe_update_farms_should_work() {
    //NOTE: this test is not testing if farms are updated correctly only if they are updated when
    //conditions are met.

    const LEFT_TO_DISTRIBUTE: Balance = 1_000_000_000;
    let reward_currency: AssetId = get_predefined_global_farm_ins1(0).reward_currency;

    //_0 - before action
    let global_farm_0 = GlobalFarmData {
        updated_at: 20,
        accumulated_rpz: FixedU128::from(20),
        live_yield_farms_count: 1,
        total_yield_farms_count: 1,
        paid_accumulated_rewards: 1_000_000,
        total_shares_z: 1_000_000,
        accumulated_rewards: 20_000,
        ..get_predefined_global_farm_ins1(0)
    };

    let yield_farm_0 = YieldFarmData {
        updated_at: 20,
        total_shares: 200_000,
        total_valued_shares: 400_000,
        accumulated_rpvs: FixedU128::from(15),
        accumulated_rpz: FixedU128::from(20),
        ..get_predefined_yield_farm_ins1(1)
    };

    new_test_ext().execute_with(|| {
        let _ = with_transaction(|| {
            let farm_account_id = LiquidityMining::farm_account_id(get_predefined_global_farm_ins1(0).id).unwrap();
            Whitelist::add_account(&farm_account_id).unwrap();

            Tokens::transfer(
                Origin::signed(TREASURY),
                farm_account_id,
                reward_currency,
                LEFT_TO_DISTRIBUTE,
            )
            .unwrap();

            pretty_assertions::assert_eq!(
                Tokens::free_balance(reward_currency, &farm_account_id),
                LEFT_TO_DISTRIBUTE
            );

            let mut global_farm = GlobalFarmData {
                ..global_farm_0.clone()
            };

            let mut yield_farm = YieldFarmData {
                state: FarmState::Stopped,
                ..yield_farm_0.clone()
            };

            let current_period = 30;

            //I. - yield farming is stopped. Nothing should be updated if yield farm is stopped.
            assert_ok!(LiquidityMining::maybe_update_farms(
                &mut global_farm,
                &mut yield_farm,
                current_period
            ));

            pretty_assertions::assert_eq!(global_farm, global_farm_0);
            pretty_assertions::assert_eq!(
                yield_farm,
                YieldFarmData {
                    state: FarmState::Stopped,
                    ..yield_farm_0.clone()
                }
            );

            //II. - yield farm has 0 shares and was updated in this period.
            let current_period = 20;
            let mut yield_farm = YieldFarmData { ..yield_farm_0.clone() };
            assert_ok!(LiquidityMining::maybe_update_farms(
                &mut global_farm,
                &mut yield_farm,
                current_period
            ));

            pretty_assertions::assert_eq!(global_farm, global_farm_0);
            pretty_assertions::assert_eq!(yield_farm, yield_farm_0);

            //III. - global farm has 0 shares and was updated in this period - only yield farm should
            //be updated.
            let current_period = 30;
            let mut global_farm = GlobalFarmData {
                total_shares_z: 0,
                updated_at: 30,
                ..global_farm_0.clone()
            };

            assert_ok!(LiquidityMining::maybe_update_farms(
                &mut global_farm,
                &mut yield_farm,
                current_period
            ));

            pretty_assertions::assert_eq!(
                global_farm,
                GlobalFarmData {
                    total_shares_z: 0,
                    updated_at: 30,
                    ..global_farm_0.clone()
                }
            );
            assert_ne!(yield_farm, yield_farm_0);
            pretty_assertions::assert_eq!(yield_farm.updated_at, current_period);

            //IV. - booth farms met conditions for update
            let current_period = 30;
            assert_ok!(LiquidityMining::maybe_update_farms(
                &mut global_farm,
                &mut yield_farm,
                current_period
            ));

            assert_ne!(global_farm, global_farm_0);
            assert_ne!(yield_farm, yield_farm_0);

            pretty_assertions::assert_eq!(global_farm.updated_at, current_period);
            pretty_assertions::assert_eq!(yield_farm.updated_at, current_period);

            TransactionOutcome::Commit(DispatchResult::Ok(()))
        });
    });
}

#[test]
fn depositdata_add_farm_entry_to_should_work() {
    new_test_ext().execute_with(|| {
        let mut deposit = DepositData::<Test, Instance1> {
            shares: 10,
            amm_pool_id: BSX_TKN1_AMM,
            yield_farm_entries: vec![].try_into().unwrap(),
        };

        let test_farm_entries = vec![
            YieldFarmEntry::<Test, Instance1>::new(1, 50, 20, FixedU128::from(12), 2),
            YieldFarmEntry::<Test, Instance1>::new(2, 18, 20, FixedU128::from(14), 18),
            YieldFarmEntry::<Test, Instance1>::new(3, 60, 20, FixedU128::from(1), 1),
            YieldFarmEntry::<Test, Instance1>::new(4, 1, 20, FixedU128::from(10), 13),
            YieldFarmEntry::<Test, Instance1>::new(7, 2, 20, FixedU128::from(10), 13),
            YieldFarmEntry::<Test, Instance1>::new(5, 100, 20, FixedU128::from(10), 13),
            YieldFarmEntry::<Test, Instance1>::new(6, 4, 20, FixedU128::from(10), 13),
        ];

        assert_ok!(deposit.add_yield_farm_entry(test_farm_entries[0].clone()));

        assert_ok!(deposit.add_yield_farm_entry(test_farm_entries[2].clone()));

        assert_ok!(deposit.add_yield_farm_entry(test_farm_entries[3].clone()));

        //`yield_farm_id` must be unique in `yield_farm_entries`
        assert_noop!(
            deposit.add_yield_farm_entry(test_farm_entries[2].clone()),
            Error::<Test, Instance1>::DoubleLock
        );
        assert_noop!(
            deposit.add_yield_farm_entry(YieldFarmEntry::<Test, Instance1>::new(1, 50, 10, FixedU128::from(1), 1)),
            Error::<Test, Instance1>::DoubleLock
        );

        assert_ok!(deposit.add_yield_farm_entry(test_farm_entries[4].clone()));

        assert_ok!(deposit.add_yield_farm_entry(test_farm_entries[6].clone()));

        pretty_assertions::assert_eq!(
            deposit,
            DepositData::<Test, Instance1> {
                shares: 10,
                amm_pool_id: BSX_TKN1_AMM,
                yield_farm_entries: vec![
                    test_farm_entries[0].clone(),
                    test_farm_entries[2].clone(),
                    test_farm_entries[3].clone(),
                    test_farm_entries[4].clone(),
                    test_farm_entries[6].clone(),
                ]
                .try_into()
                .unwrap(),
            }
        );

        //5 is max farm entries.
        assert_noop!(
            deposit.add_yield_farm_entry(test_farm_entries[5].clone()),
            Error::<Test, Instance1>::MaxEntriesPerDeposit
        );
    });
}

#[test]
fn deposit_remove_yield_farm_entry_should_work() {
    new_test_ext().execute_with(|| {
        let mut deposit = DepositData::<Test, Instance1> {
            shares: 10,
            amm_pool_id: BSX_TKN1_AMM,
            yield_farm_entries: vec![
                YieldFarmEntry::<Test, Instance1>::new(4, 1, 20, FixedU128::from(10), 13),
                YieldFarmEntry::<Test, Instance1>::new(7, 2, 20, FixedU128::from(1), 13),
                YieldFarmEntry::<Test, Instance1>::new(6, 4, 20, FixedU128::from(10), 13),
                YieldFarmEntry::<Test, Instance1>::new(2, 18, 20, FixedU128::from(14), 18),
                YieldFarmEntry::<Test, Instance1>::new(3, 60, 20, FixedU128::from(1), 1),
            ]
            .try_into()
            .unwrap(),
        };

        const NON_EXISTING_YIELD_FARM_ID: YieldFarmId = 999_999_999;
        assert_noop!(
            deposit.remove_yield_farm_entry(NON_EXISTING_YIELD_FARM_ID),
            Error::<Test, Instance1>::YieldFarmEntryNotFound
        );

        assert_ok!(deposit.remove_yield_farm_entry(2));
        assert_ok!(deposit.remove_yield_farm_entry(18));
        assert_ok!(deposit.remove_yield_farm_entry(1));
        assert_ok!(deposit.remove_yield_farm_entry(4));
        assert_ok!(deposit.remove_yield_farm_entry(60));

        //This state should never happen, deposit should be flushed from storage when have no more
        //entries.
        pretty_assertions::assert_eq!(
            deposit.yield_farm_entries,
            TryInto::<BoundedVec<YieldFarmEntry<Test, Instance1>, ConstU32<5>>>::try_into(vec![]).unwrap()
        );

        assert_noop!(
            deposit.remove_yield_farm_entry(60),
            Error::<Test, Instance1>::YieldFarmEntryNotFound
        );
    });
}

#[test]
fn deposit_get_yield_farm_entry_should_work() {
    let mut deposit = DepositData::<Test, Instance1> {
        shares: 10,
        amm_pool_id: BSX_TKN1_AMM,
        yield_farm_entries: vec![
            YieldFarmEntry::<Test, Instance1>::new(4, 1, 20, FixedU128::from(10), 13),
            YieldFarmEntry::<Test, Instance1>::new(7, 2, 20, FixedU128::from(1), 13),
            YieldFarmEntry::<Test, Instance1>::new(6, 4, 20, FixedU128::from(10), 13),
            YieldFarmEntry::<Test, Instance1>::new(2, 18, 20, FixedU128::from(14), 18),
            YieldFarmEntry::<Test, Instance1>::new(3, 60, 20, FixedU128::from(1), 1),
        ]
        .try_into()
        .unwrap(),
    };

    pretty_assertions::assert_eq!(
        deposit.get_yield_farm_entry(18).unwrap(),
        &mut YieldFarmEntry::<Test, Instance1>::new(2, 18, 20, FixedU128::from(14), 18)
    );

    const NON_EXISTING_YIELD_FARM_ID: YieldFarmId = 98_908;
    assert!(deposit.get_yield_farm_entry(NON_EXISTING_YIELD_FARM_ID).is_none())
}

#[test]
fn deposit_search_yield_farm_entry_should_work() {
    let deposit = DepositData::<Test, Instance1> {
        shares: 10,
        amm_pool_id: BSX_TKN1_AMM,
        yield_farm_entries: vec![
            YieldFarmEntry::<Test, Instance1>::new(4, 1, 20, FixedU128::from(10), 13),
            YieldFarmEntry::<Test, Instance1>::new(7, 2, 20, FixedU128::from(1), 13),
            YieldFarmEntry::<Test, Instance1>::new(6, 4, 20, FixedU128::from(10), 13),
            YieldFarmEntry::<Test, Instance1>::new(2, 18, 20, FixedU128::from(14), 18),
            YieldFarmEntry::<Test, Instance1>::new(3, 60, 20, FixedU128::from(1), 1),
        ]
        .try_into()
        .unwrap(),
    };

    assert!(deposit.search_yield_farm_entry(1).is_some());
    assert!(deposit.search_yield_farm_entry(60).is_some());
    assert!(deposit.search_yield_farm_entry(4).is_some());

    const NON_EXISTING_YIELD_FARM_ID: YieldFarmId = 98_908;

    assert!(deposit.search_yield_farm_entry(NON_EXISTING_YIELD_FARM_ID).is_none());
}

#[test]
fn deposit_can_be_flushed_should_work() {
    //non empty deposit can't be flushed
    let deposit = DepositData::<Test, Instance1> {
        shares: 10,
        amm_pool_id: BSX_TKN1_AMM,
        yield_farm_entries: vec![
            YieldFarmEntry::<Test, Instance1>::new(4, 1, 20, FixedU128::from(10), 13),
            YieldFarmEntry::<Test, Instance1>::new(7, 2, 20, FixedU128::from(1), 13),
            YieldFarmEntry::<Test, Instance1>::new(6, 4, 20, FixedU128::from(10), 13),
            YieldFarmEntry::<Test, Instance1>::new(2, 18, 20, FixedU128::from(14), 18),
            YieldFarmEntry::<Test, Instance1>::new(3, 60, 20, FixedU128::from(1), 1),
        ]
        .try_into()
        .unwrap(),
    };

    assert!(!deposit.can_be_removed());

    let deposit = DepositData::<Test, Instance1> {
        shares: 10,
        amm_pool_id: BSX_TKN1_AMM,
        yield_farm_entries: vec![YieldFarmEntry::<Test, Instance1>::new(
            4,
            1,
            20,
            FixedU128::from(10),
            13,
        )]
        .try_into()
        .unwrap(),
    };

    assert!(!deposit.can_be_removed());

    //deposit with no entries can be flushed
    let deposit = DepositData::<Test, Instance1> {
        shares: 10,
        amm_pool_id: BSX_TKN1_AMM,
        yield_farm_entries: vec![].try_into().unwrap(),
    };

    assert!(deposit.can_be_removed());
}

#[test]
fn yield_farm_data_should_work() {
    new_test_ext().execute_with(|| {
        let mut yield_farm =
            YieldFarmData::<Test, Instance1>::new(1, 10, Some(LoyaltyCurve::default()), FixedU128::from(10_000));

        //new farm should be created active
        assert!(yield_farm.state.is_active());
        assert!(!yield_farm.state.is_stopped());
        assert!(!yield_farm.state.is_terminated());

        yield_farm.state = FarmState::Stopped;
        assert!(!yield_farm.state.is_active());
        assert!(yield_farm.state.is_stopped());
        assert!(!yield_farm.state.is_terminated());

        yield_farm.state = FarmState::Terminated;
        assert!(!yield_farm.state.is_active());
        assert!(!yield_farm.state.is_stopped());
        assert!(yield_farm.state.is_terminated());

        assert_ok!(yield_farm.increase_entries_count());
        pretty_assertions::assert_eq!(yield_farm.entries_count, 1);
        assert_ok!(yield_farm.increase_entries_count());
        assert_ok!(yield_farm.increase_entries_count());
        assert_ok!(yield_farm.increase_entries_count());
        pretty_assertions::assert_eq!(yield_farm.entries_count, 4);

        assert_ok!(yield_farm.decrease_entries_count());
        pretty_assertions::assert_eq!(yield_farm.entries_count, 3);
        assert_ok!(yield_farm.decrease_entries_count());
        assert_ok!(yield_farm.decrease_entries_count());
        assert_ok!(yield_farm.decrease_entries_count());
        pretty_assertions::assert_eq!(yield_farm.entries_count, 0);
        assert_noop!(
            yield_farm.decrease_entries_count(),
            Error::<Test, Instance1>::InconsistentState(InconsistentStateError::InvalidYieldFarmEntriesCount)
        );

        //no entries in the farm
        yield_farm.entries_count = 0;
        assert!(!yield_farm.has_entries());
        assert_ok!(yield_farm.increase_entries_count());
        assert!(yield_farm.has_entries());

        yield_farm.state = FarmState::Active;
        yield_farm.entries_count = 0;
        //active farm can't be flushed
        assert!(!yield_farm.can_be_removed());

        //stopped farm can't be flushed
        yield_farm.state = FarmState::Stopped;
        assert!(!yield_farm.can_be_removed());

        //deleted farm with entries can't be flushed
        yield_farm.state = FarmState::Terminated;
        yield_farm.entries_count = 1;
        assert!(!yield_farm.can_be_removed());

        //deleted farm with no entries can be flushed
        yield_farm.entries_count = 0;
        assert!(yield_farm.can_be_removed());
    });
}

#[test]
fn global_farm_should_work() {
    let mut global_farm = GlobalFarmData::<Test, Instance1>::new(
        1,
        10,
        BSX,
        Perquintill::from_float(0.2),
        1_000,
        100,
        GC,
        BSX,
        1_000_000,
        1_000,
        One::one(),
    );

    //new farm should be created active
    assert!(global_farm.state.is_active());
    global_farm.state = FarmState::Terminated;
    assert!(!global_farm.state.is_active());

    global_farm.state = FarmState::Active;

    assert_ok!(global_farm.increase_yield_farm_counts());
    assert_ok!(global_farm.increase_yield_farm_counts());
    pretty_assertions::assert_eq!(global_farm.live_yield_farms_count, 2);
    pretty_assertions::assert_eq!(global_farm.total_yield_farms_count, 2);
    assert_ok!(global_farm.increase_yield_farm_counts());
    assert_ok!(global_farm.increase_yield_farm_counts());
    pretty_assertions::assert_eq!(global_farm.live_yield_farms_count, 4);
    pretty_assertions::assert_eq!(global_farm.total_yield_farms_count, 4);
    assert_ok!(global_farm.decrease_live_yield_farm_count());
    assert_ok!(global_farm.decrease_live_yield_farm_count());
    //removing farm changes only live farms, total count is not changed
    pretty_assertions::assert_eq!(global_farm.live_yield_farms_count, 2);
    pretty_assertions::assert_eq!(global_farm.total_yield_farms_count, 4);
    assert_ok!(global_farm.increase_yield_farm_counts());
    pretty_assertions::assert_eq!(global_farm.live_yield_farms_count, 3);
    pretty_assertions::assert_eq!(global_farm.total_yield_farms_count, 5);
    assert_ok!(global_farm.decrease_total_yield_farm_count());
    assert_ok!(global_farm.decrease_total_yield_farm_count());
    //removing farm changes only total count(farm has to removed and deleted before it can be
    //flushed)
    pretty_assertions::assert_eq!(global_farm.live_yield_farms_count, 3);
    pretty_assertions::assert_eq!(global_farm.total_yield_farms_count, 3);

    assert!(global_farm.has_live_farms());
    global_farm.live_yield_farms_count = 0;
    global_farm.total_yield_farms_count = 3;
    assert!(!global_farm.has_live_farms());

    //active farm can't be flushed
    assert!(!global_farm.can_be_removed());
    global_farm.state = FarmState::Terminated;
    //deleted farm with yield farm can't be flushed
    assert!(!global_farm.can_be_removed());
    //deleted farm with no yield farms can be flushed
    global_farm.live_yield_farms_count = 0;
    global_farm.total_yield_farms_count = 0;
    assert!(global_farm.can_be_removed());
}

#[test]
fn is_yield_farm_clamable_should_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        let _ = with_transaction(|| {
            //active farm
            assert!(LiquidityMining::is_yield_farm_claimable(
                GC_FARM,
                GC_BSX_TKN1_YIELD_FARM_ID,
                BSX_TKN1_AMM
            ));

            //invalid amm_pool_id
            assert!(!LiquidityMining::is_yield_farm_claimable(
                GC_FARM,
                GC_BSX_TKN1_YIELD_FARM_ID,
                BSX_TKN2_AMM
            ));

            //farm withouth deposits
            assert!(!LiquidityMining::is_yield_farm_claimable(
                EVE_FARM,
                EVE_BSX_TKN1_YIELD_FARM_ID,
                BSX_TKN1_AMM
            ));

            //termianted yield farm
            assert_ok!(LiquidityMining::stop_yield_farm(GC, GC_FARM, BSX_TKN1_AMM));
            assert_ok!(LiquidityMining::terminate_yield_farm(
                GC,
                GC_FARM,
                GC_BSX_TKN1_YIELD_FARM_ID,
                BSX_TKN1_AMM
            ));

            assert!(!LiquidityMining::is_yield_farm_claimable(
                GC_FARM,
                GC_BSX_TKN1_YIELD_FARM_ID,
                BSX_TKN1_AMM
            ));

            TransactionOutcome::Commit(DispatchResult::Ok(()))
        });
    });
}

#[test]
fn get_global_farm_id_should_work() {
    predefined_test_ext_with_deposits().execute_with(|| {
        let _ = with_transaction(|| {
            //happy path
            pretty_assertions::assert_eq!(
                LiquidityMining::get_global_farm_id(PREDEFINED_DEPOSIT_IDS[0], GC_BSX_TKN1_YIELD_FARM_ID),
                Some(GC_FARM)
            );

            //happy path deposit with multiple farm entries
            //create second farm entry
            assert_ok!(LiquidityMining::redeposit_lp_shares(
                EVE_FARM,
                EVE_BSX_TKN1_YIELD_FARM_ID,
                PREDEFINED_DEPOSIT_IDS[0],
                |_, _, _| { Ok(10_u128) }
            ));

            pretty_assertions::assert_eq!(
                LiquidityMining::get_global_farm_id(PREDEFINED_DEPOSIT_IDS[0], EVE_BSX_TKN1_YIELD_FARM_ID),
                Some(EVE_FARM)
            );

            //deposit doesn't exists
            assert!(LiquidityMining::get_global_farm_id(999_9999, GC_BSX_TKN1_YIELD_FARM_ID).is_none());

            //farm's entry doesn't exists in the deposit
            assert!(
                LiquidityMining::get_global_farm_id(PREDEFINED_DEPOSIT_IDS[0], DAVE_BSX_TKN1_YIELD_FARM_ID).is_none()
            );

            TransactionOutcome::Commit(DispatchResult::Ok(()))
        });
    });
}

#[test]
fn farm_state_should_work() {
    let active = FarmState::Active;
    let deleted = FarmState::Terminated;
    let stopped = FarmState::Stopped;

    pretty_assertions::assert_eq!(active.is_active(), true);
    pretty_assertions::assert_eq!(active.is_stopped(), false);
    pretty_assertions::assert_eq!(active.is_terminated(), false);

    pretty_assertions::assert_eq!(stopped.is_active(), false);
    pretty_assertions::assert_eq!(stopped.is_stopped(), true);
    pretty_assertions::assert_eq!(stopped.is_terminated(), false);

    pretty_assertions::assert_eq!(deleted.is_active(), false);
    pretty_assertions::assert_eq!(deleted.is_stopped(), false);
    pretty_assertions::assert_eq!(deleted.is_terminated(), true);
}

#[test]
fn min_yield_farm_multiplier_should_be_ge_1_when_multiplied_by_min_deposit() {
    //WARN: don't remove this test. This rule is important.
    // min_yield_farm_multiplier * min_deposit >=1 otherwise non-zero deposit can result in a zero
    // stake in global-farm and farm can be falsely identified as empty.
    //https://github.com/galacticcouncil/warehouse/issues/127

    pretty_assertions::assert_eq!(
        crate::MIN_YIELD_FARM_MULTIPLIER
            .checked_mul_int(crate::MIN_DEPOSIT)
            .unwrap()
            .ge(&1_u128),
        true
    );
}

#[test]
fn update_global_farm_should_emit_all_rewards_distributed_when_reward_is_zero() {
    new_test_ext().execute_with(|| {
        let global_farm_id = 10;

        let mut global_farm = GlobalFarmData::new(
            global_farm_id,
            10,
            BSX,
            Perquintill::from_percent(1),
            10_000,
            1,
            ALICE,
            BSX,
            1_000_000 * ONE,
            1_000,
            One::one(),
        );
        global_farm.total_shares_z = 1_000 * ONE;

        let farm_account_id = LiquidityMining::farm_account_id(global_farm_id).unwrap();
        Whitelist::add_account(&farm_account_id).unwrap();

        pretty_assertions::assert_eq!(Tokens::free_balance(BSX, &farm_account_id), Balance::zero());

        pretty_assertions::assert_eq!(
            with_transaction(|| {
                TransactionOutcome::Commit(LiquidityMining::update_global_farm(
                    &mut global_farm,
                    1_000_000_000,
                    1_000_000 * ONE,
                ))
            })
            .unwrap(),
            Balance::zero()
        );

        frame_system::Pallet::<Test>::assert_has_event(mock::Event::LiquidityMining(Event::AllRewardsDistributed {
            global_farm_id,
        }));
    });
}
