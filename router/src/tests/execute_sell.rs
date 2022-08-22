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

use super::*;
use crate::mock::*;
use crate::types::Trade;
use crate::Error;
use frame_support::traits::OnFinalize;
use frame_support::{assert_err, assert_noop, assert_ok};
use hydradx_traits::router::PoolType;
use pretty_assertions::assert_eq;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::ops::Deref;
use sp_runtime::DispatchError::BadOrigin;

#[test]
fn execute_sell_should_work_when_route_has_single_trade() {
    ExtBuilder::default().build().execute_with(|| {
        //Arrange
        let amount = 10;
        let limit = 5;

        let trades = vec![BSX_AUSD_TRADE_IN_XYK];

        //Act
        assert_ok!(Router::execute_sell(
            Origin::signed(ALICE),
            BSX,
            aUSD,
            amount,
            limit,
            trades
        ));

        //Assert
        assert_executed_sell_trades(vec![(PoolType::XYK, amount, BSX, aUSD)]);
    });
}

#[test]
fn execute_sell_should_fail_when_route_has_single_trade_producing_calculation_error() {
    ExtBuilder::default().build().execute_with(|| {
        //Arrange
        let limit = 5;

        let trades = vec![BSX_AUSD_TRADE_IN_XYK];

        //Act and Assert
        assert_noop!(
            Router::execute_sell(
                Origin::signed(ALICE),
                BSX,
                aUSD,
                INVALID_CALCULATION_AMOUNT,
                limit,
                trades
            ),
            Error::<Test>::PriceCalculationFailed
        );
    });
}

#[test]
fn execute_sell_should_work_when_route_has_multiple_trades_with_same_pooltype() {
    ExtBuilder::default().build().execute_with(|| {
        //Arrange
        let amount = 10;
        let limit = 5;
        let trade1 = Trade {
            pool: PoolType::XYK,
            asset_in: BSX,
            asset_out: aUSD,
        };
        let trade2 = Trade {
            pool: PoolType::XYK,
            asset_in: aUSD,
            asset_out: MOVR,
        };
        let trade3 = Trade {
            pool: PoolType::XYK,
            asset_in: MOVR,
            asset_out: KSM,
        };
        let trades = vec![trade1, trade2, trade3];

        //Act
        assert_ok!(Router::execute_sell(
            Origin::signed(ALICE),
            BSX,
            KSM,
            amount,
            limit,
            trades
        ));

        //Assert
        assert_executed_sell_trades(vec![
            (PoolType::XYK, amount, BSX, aUSD),
            (PoolType::XYK, XYK_SELL_CALCULATION_RESULT, aUSD, MOVR),
            (PoolType::XYK, XYK_SELL_CALCULATION_RESULT, MOVR, KSM),
        ]);
    });
}

#[test]
fn execute_sell_should_work_when_route_has_multiple_trades_with_different_pool_type() {
    ExtBuilder::default().build().execute_with(|| {
        //Arrange
        let amount = 10;
        let limit = 1;
        let trade1 = Trade {
            pool: PoolType::XYK,
            asset_in: BSX,
            asset_out: MOVR,
        };
        let trade2 = Trade {
            pool: PoolType::Stableswap(aUSD),
            asset_in: MOVR,
            asset_out: aUSD,
        };
        let trade3 = Trade {
            pool: PoolType::Omnipool,
            asset_in: aUSD,
            asset_out: KSM,
        };
        let trades = vec![trade1, trade2, trade3];

        //Act
        assert_ok!(Router::execute_sell(
            Origin::signed(ALICE),
            BSX,
            KSM,
            amount,
            limit,
            trades
        ));

        //Assert
        assert_executed_sell_trades(vec![
            (PoolType::XYK, amount, BSX, MOVR),
            (PoolType::Stableswap(aUSD), XYK_SELL_CALCULATION_RESULT, MOVR, aUSD),
            (PoolType::Omnipool, STABLESWAP_SELL_CALCULATION_RESULT, aUSD, KSM),
        ]);
    });
}

#[test]
fn execute_sell_should_work_when_first_trade_is_not_supported_in_the_first_pool() {
    ExtBuilder::default().build().execute_with(|| {
        //Arrange
        let amount = 10;
        let limit = 5;
        let trade1 = Trade {
            pool: PoolType::Stableswap(aUSD),
            asset_in: BSX,
            asset_out: aUSD,
        };
        let trade2 = Trade {
            pool: PoolType::XYK,
            asset_in: aUSD,
            asset_out: KSM,
        };
        let trades = vec![trade1, trade2];

        //Act
        assert_ok!(Router::execute_sell(
            Origin::signed(ALICE),
            BSX,
            KSM,
            amount,
            limit,
            trades
        ));

        //Assert
        assert_executed_sell_trades(vec![
            (PoolType::Stableswap(aUSD), amount, BSX, aUSD),
            (PoolType::XYK, STABLESWAP_SELL_CALCULATION_RESULT, aUSD, KSM),
        ]);
    });
}

#[test]
fn execute_sell_should_fail_when_called_with_non_signed_origin() {
    ExtBuilder::default().build().execute_with(|| {
        //Arrange
        let amount = 10;
        let limit = 5;
        let trades = vec![BSX_AUSD_TRADE_IN_XYK];

        //Act and Assert
        assert_noop!(
            Router::execute_sell(
            Origin::none(),
            BSX,
            aUSD,
            amount,
            limit,
            trades
        ),
            BadOrigin
        );
    });
}
