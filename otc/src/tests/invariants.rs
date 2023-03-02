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

fn asset_amount() -> impl Strategy<Value = Balance> {
    ONE..95 * ONE
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1_000))]
    #[test]
    fn otc_price_invariant(
        amount_fill in asset_amount(),
    ) {
        ExtBuilder::default().build().execute_with(|| {
            let initial_amount_in = 100 * ONE;
            let initial_amount_out = 10_000 * ONE;

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
