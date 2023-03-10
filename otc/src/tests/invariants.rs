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
use pretty_assertions::assert_eq;
use proptest::prelude::*;

fn asset_amount(min: Balance, max: Balance) -> impl Strategy<Value = Balance> {
    min..max
}

prop_compose! {
    fn get_asset_amounts()
        (
            amount_in in asset_amount(6 * ONE, 100 * ONE),
        )
        (
            amount_in in Just(amount_in),
            amount_out in asset_amount(amount_in, 1000 * ONE),
            amount_fill in asset_amount(ONE, amount_in - 5 * ONE),
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
                Origin::signed(ALICE),
                DAI,
                HDX,
                initial_amount_in,
                initial_amount_out,
                true
            ).unwrap();

            let initial_price = initial_amount_out / initial_amount_in;

            OTC::partial_fill_order(Origin::signed(BOB), 0, amount_fill).unwrap();

            let order = OTC::orders(0).unwrap();
            let new_price = order.amount_out / order.amount_in;

            assert_eq!(initial_price, new_price);
        });
    }
}
