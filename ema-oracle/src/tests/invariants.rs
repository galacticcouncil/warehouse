// This file is part of pallet-ema-oracle.

// Copyright (C) 2022  Intergalactic, Limited (GIB).
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

use pretty_assertions::assert_eq;
use proptest::prelude::*;

// Strategies
fn valid_asset_ids() -> impl Strategy<Value = (AssetId, AssetId)> {
    (any::<AssetId>(), any::<AssetId>()).prop_filter("asset ids should not be equal", |(a, b)| a != b)
}

fn non_zero_amount() -> impl Strategy<Value = Balance> {
    any::<Balance>().prop_filter("balances should be greater 0", |b| b > &0)
}

// Tests
proptest! {
    #[test]
    fn price_normalization_should_be_independent_of_asset_order(
        (asset_a, asset_b) in valid_asset_ids(),
        (amount_a, amount_b) in (non_zero_amount(), non_zero_amount())
    ) {
        let a_then_b = determine_normalized_price(asset_a, asset_b, amount_a, amount_b);
        let b_then_a = determine_normalized_price(asset_b, asset_a, amount_b, amount_a);
        prop_assert!(a_then_b.is_some());
        prop_assert!(b_then_a.is_some());
        prop_assert_eq!(a_then_b.unwrap(), b_then_a.unwrap());
    }
}

proptest! {
    #[test]
    fn on_liquidity_changed_should_not_change_volume(
        (asset_a, asset_b) in valid_asset_ids(),
        (amount_a, amount_b) in (non_zero_amount(), non_zero_amount()),
        liquidity in non_zero_amount(),
        (second_amount_a, second_amount_b) in (non_zero_amount(), non_zero_amount()),
        second_liquidity in non_zero_amount(),
    ) {
        new_test_ext().execute_with(|| {
            let timestamp = 5;
            System::set_block_number(timestamp);
            OnActivityHandler::<Test>::on_trade(SOURCE, asset_a, asset_b, amount_a, amount_b, liquidity);
            let volume_before = get_accumulator_entry(SOURCE, (asset_a, asset_b)).unwrap().volume;
            OnActivityHandler::<Test>::on_liquidity_changed(SOURCE, asset_a, asset_b, second_amount_a, second_amount_b, second_liquidity);
            let volume_after = get_accumulator_entry(SOURCE, (asset_a, asset_b)).unwrap().volume;
            assert_eq!(volume_before, volume_after);
        });
    }
}
