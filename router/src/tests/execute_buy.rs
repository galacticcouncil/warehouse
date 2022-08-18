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
use std::cell::RefCell;
use std::ops::Deref;
use super::*;
use crate::mock::{Currency, ExtBuilder, Origin, Router, Test, ALICE, aUSD, BSX, KSM, EXECUTED_SELLS, AssetId, Balance};
use frame_support::traits::OnFinalize;
use frame_support::{assert_noop, assert_ok};
use hydradx_traits::router::PoolType;
use crate::types::Trade;
use pretty_assertions::assert_eq;

#[test]
fn execute_buy_should_when_route_has_single_trade() {
    ExtBuilder::default().build().execute_with(|| {
        //Arrange
        let amount = 10;
        let limit = 5;
        let trade = Trade{
            pool: PoolType::XYK,
            asset_in: BSX,
            asset_out: aUSD
        };
        let trades = vec![trade];

        //Act
        assert_ok!(Router::execute_buy(Origin::signed(ALICE), BSX, aUSD, amount, limit,trades));

        //Assert
        assert_executed_trades(vec![(PoolType::XYK, 5)]);
    });
}

#[test]
fn execute_buy_should_when_route_has_multiple_trades() {
    ExtBuilder::default().build().execute_with(|| {
        //Arrange
        let amount = 10;
        let limit = 5;
        let trade1 = Trade{
            pool: PoolType::XYK,
            asset_in: BSX,
            asset_out: aUSD
        };
        let trade2 = Trade{
            pool: PoolType::XYK,
            asset_in: aUSD,
            asset_out: KSM
        };
        let trades = vec![trade1, trade2];

        //Act
        assert_ok!(Router::execute_buy(Origin::signed(ALICE), BSX, KSM, amount, limit,trades));

        //Assert
        assert_executed_trades(vec![(PoolType::XYK, 5), (PoolType::XYK, 5)]);
    });
}

fn assert_executed_trades(expected_trades :Vec<(PoolType<AssetId>, Balance)>) {
    EXECUTED_SELLS.borrow().with(|v| {
        let trades = v.borrow().deref().clone();
        assert_eq!(expected_trades,trades);
    });
}
