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
use crate::mock::{Currency, ExtBuilder, Origin, Router, Test, ALICE, aUSD, BSX, KSM, EXECUTED_SELLS,SELL_CALCULATION_RESULT, INVALID_CALCULATION_AMOUNT, AssetId, Balance, assert_executed_sell_trades, assert_that_there_is_no_any_executed_buys};
use frame_support::traits::OnFinalize;
use frame_support::{assert_err, assert_noop, assert_ok};
use hydradx_traits::router::PoolType;
use crate::types::Trade;
use pretty_assertions::assert_eq;
use crate::Error;

#[test]
fn execute_sell_should_work_when_route_has_single_trade() {
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
        assert_ok!(Router::execute_sell(Origin::signed(ALICE), BSX, aUSD, amount, limit,trades));

        //Assert
        assert_executed_sell_trades(vec![(PoolType::XYK, amount)]);
        assert_that_there_is_no_any_executed_buys();
    });
}

#[test]
fn execute_sell_should_fail_when_route_has_single_trade_producing_calculation_error() {
    ExtBuilder::default().build().execute_with(|| {
        //Arrange
        let limit = 5;
        let trade = Trade{
            pool: PoolType::XYK,
            asset_in: BSX,
            asset_out: aUSD
        };
        let trades = vec![trade];

        //Act and Assert
        assert_noop!(
            Router::execute_sell(Origin::signed(ALICE), BSX, aUSD, INVALID_CALCULATION_AMOUNT, limit,trades),
            Error::<Test>::PriceCalculationFailed
        );
    });
}

#[test]
fn execute_sell_should_work_when_route_has_multiple_trades() {
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
        assert_ok!(Router::execute_sell(Origin::signed(ALICE), BSX, KSM, amount, limit,trades));

        //Assert
        assert_executed_sell_trades(vec![(PoolType::XYK, amount), (PoolType::XYK, SELL_CALCULATION_RESULT)]);
        assert_that_there_is_no_any_executed_buys();
    });
}

//TODO: Dani - add cases for errors
