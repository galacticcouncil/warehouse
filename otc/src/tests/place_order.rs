// This file is part of galacticcouncil/warehouse.
// Copyright (C) 2020-2022  Intergalactic, Limited (GIB). SPDX-License-Identifier: Apache-2.0

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

use crate::{Error, Event};
use frame_support::{assert_noop, assert_ok};
use orml_traits::NamedMultiReservableCurrency;
use pretty_assertions::assert_eq;

#[test]
fn place_order_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        // Act
        assert_ok!(OTC::place_order(
            Origin::signed(ALICE),
            DAI,
            HDX,
            20 * ONE,
            100 * ONE,
            true
        ));

        // Assert
        let order = OTC::orders(0).unwrap();
        assert_eq!(order.owner, ALICE);
        assert_eq!(order.asset_buy, DAI);
        assert_eq!(order.asset_sell, HDX);
        assert_eq!(order.amount_buy, 20 * ONE);
        assert_eq!(order.partially_fillable, true);

        expect_events(vec![Event::OrderPlaced {
            order_id: 0,
            asset_buy: DAI,
            asset_sell: HDX,
            amount_buy: order.amount_buy,
            amount_sell: 100 * ONE,
            partially_fillable: true,
        }
        .into()]);

        let reserve_id = named_reserve_identifier(0);
        assert_eq!(Tokens::reserved_balance_named(&reserve_id, HDX, &ALICE), 100 * ONE);

        let next_order_id = OTC::next_order_id();
        assert_eq!(next_order_id, 1);
    });
}

#[test]
fn place_order_should_work_when_user_has_multiple_orders() {
    ExtBuilder::default().build().execute_with(|| {
        // Act
        assert_ok!(OTC::place_order(
            Origin::signed(ALICE),
            DAI,
            HDX,
            20 * ONE,
            100 * ONE,
            true
        ));

        assert_ok!(OTC::place_order(
            Origin::signed(ALICE),
            DAI,
            HDX,
            10 * ONE,
            50 * ONE,
            true
        ));

        // Assert
        let reserve_id_0 = named_reserve_identifier(0);
        assert_eq!(Tokens::reserved_balance_named(&reserve_id_0, HDX, &ALICE), 100 * ONE);

        let reserve_id_1 = named_reserve_identifier(1);
        assert_eq!(Tokens::reserved_balance_named(&reserve_id_1, HDX, &ALICE), 50 * ONE);
    });
}

#[test]
fn place_order_should_throw_error_when_amount_is_higher_than_balance() {
    ExtBuilder::default().build().execute_with(|| {
        // Act
        assert_noop!(
            OTC::place_order(Origin::signed(ALICE), DAI, HDX, 20 * ONE, 100_000 * ONE, true),
            Error::<Test>::InsufficientBalance
        );
    });
}

#[test]
fn place_order_should_throw_error_when_asset_sell_is_not_registered() {
    ExtBuilder::default().build().execute_with(|| {
        // Act
        assert_noop!(
            OTC::place_order(Origin::signed(ALICE), DAI, DOGE, 20 * ONE, 100 * ONE, true),
            Error::<Test>::AssetNotRegistered
        );
    });
}

#[test]
fn place_order_should_throw_error_when_asset_buy_is_not_registered() {
    ExtBuilder::default().build().execute_with(|| {
        // Act
        assert_noop!(
            OTC::place_order(Origin::signed(ALICE), DOGE, HDX, 20 * ONE, 100 * ONE, true),
            Error::<Test>::AssetNotRegistered
        );
    });
}

#[test]
fn place_order_should_throw_error_when_amount_buy_is_too_low() {
    ExtBuilder::default().build().execute_with(|| {
        // Act
        assert_noop!(
            OTC::place_order(Origin::signed(ALICE), DAI, HDX, 4 * ONE, 100 * ONE, true),
            Error::<Test>::OrderAmountTooSmall
        );
    });
}

#[test]
fn place_order_should_throw_error_when_amount_sell_is_too_low() {
    ExtBuilder::default().build().execute_with(|| {
        // Act
        assert_noop!(
            OTC::place_order(Origin::signed(ALICE), DAI, HDX, 20 * ONE, 4 * ONE, true),
            Error::<Test>::OrderAmountTooSmall
        );
    });
}
