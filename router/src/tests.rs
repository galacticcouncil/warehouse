// This file is part of HydraDX.

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

use std::borrow::Borrow;
use super::*;
use crate::mock::{Currency, ExtBuilder, Origin, Router, Test, ALICE, HDX, EXECUTED_TRADES, AssetId, Balance};
use frame_support::traits::OnFinalize;
use frame_support::{assert_noop, assert_ok};
use hydradx_traits::router::PoolType;
use crate::types::Trade;


#[test]
fn execute_sell_should_when_route_has_single_trade() {
    ExtBuilder::default().build().execute_with(|| {
        //Arrange
        let trade = Trade{
            pool: PoolType::XYK,
            asset_in: 0,
            asset_out: 1
        };
        let trades = vec![trade];

        //Act
        assert_ok!(Router::execute_sell(Origin::signed(ALICE), 0, 1, 3, 3,trades));

        //Assert
        assert_trades(vec![(PoolType::XYK, 3)]);
    });
}

fn assert_trades(expected_trades :Vec<(PoolType<AssetId>, Balance)>) {
    for expected_trade in expected_trades {
        EXECUTED_TRADES.borrow().with(|v| {
            assert!(v.borrow().contains(&expected_trade));
        });
    }
}
