// This file is part of galacticcouncil/warehouse.

// Copyright (C) 2020-2023  Intergalactic, Limited (GIB).
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

use crate::tests::mock::*;
use proptest::prelude::*;
use sp_runtime::FixedU128;
use std::cmp::min;
use test_utils::assert_eq_approx;

const MIN_ORDER_SIZE: Balance = 5 * ONE;
const DEVIATION_TOLERANCE: f64 = 0.000_000_000_1;

fn asset_amount(max: Balance) -> impl Strategy<Value = Balance> {
    (MIN_ORDER_SIZE + ONE)..max
}

fn amount_fill(amount_in: Balance, amount_out: Balance) -> impl Strategy<Value = Balance> {
    let max_remaining_amount_out = amount_in - MIN_ORDER_SIZE * amount_in / amount_out;
    let max_remaining_amount_in = amount_in - MIN_ORDER_SIZE;

    ONE..min(max_remaining_amount_out, max_remaining_amount_in)
}

prop_compose! {
    fn get_asset_amounts()
    (
        amount_in in asset_amount(100 * ONE),
        amount_out in asset_amount(100 * ONE),
    )
    (
        amount_in in Just(amount_in),
        amount_out in Just(amount_out),
        amount_fill in amount_fill(amount_in, amount_out),
    )
    -> (Balance, Balance, Balance) {
        (amount_in, amount_out, amount_fill)
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1_000))]
    #[test]
    fn otc_price_invariant(
        (initial_amount_in, initial_amount_out, amount_fill) in get_asset_amounts()
    ) {
        ExtBuilder::default().build().execute_with(|| {
            OTC::place_order(
                RuntimeOrigin::signed(ALICE),
                DAI,
                HDX,
                initial_amount_in,
                initial_amount_out,
                true
            ).unwrap();

            let initial_price = FixedU128::from(initial_amount_out) / FixedU128::from(initial_amount_in);

            OTC::partial_fill_order(RuntimeOrigin::signed(BOB), 0, amount_fill).unwrap();

            let order = OTC::orders(0).unwrap();
            let new_price = FixedU128::from(order.amount_out) / FixedU128::from(order.amount_in);

            assert_eq_approx!(
                initial_price,
                new_price,
                FixedU128::from_float(DEVIATION_TOLERANCE),
                "initial_amount_in / initial_amount_out = amount_in / amount_out"
            );
        });
    }
}
