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
use orml_traits::MultiCurrency;
use orml_traits::NamedMultiReservableCurrency;
use pretty_assertions::assert_eq;

#[test]
fn partial_fill_order_should_work_when_order_is_partially_fillable() {
    ExtBuilder::default().build().execute_with(|| {
        let reserve_id = named_reserve_identifier(1);

        // Arrange
        assert_ok!(OTC::place_order(
            Origin::signed(ALICE),
            DAI,
            HDX,
            20 * ONE,
            100 * ONE,
            true
        ));

        let alice_free_hdx_balance_before = Tokens::free_balance(HDX, &ALICE);
        let alice_reserved_hdx_balance_before = Tokens::reserved_balance_named(&reserve_id, HDX, &ALICE);
        let bob_hdx_balance_before = Tokens::free_balance(HDX, &BOB);

        let alice_dai_balance_before = Tokens::free_balance(DAI, &ALICE);
        let bob_dai_balance_before = Tokens::free_balance(DAI, &BOB);

        // Act
        let amount = 5 * ONE;
        assert_ok!(OTC::fill_order(Origin::signed(BOB), 1, DAI, amount));

        // Assert
        let expected_amount_receive = 25_000_000_000_000_u128;
        let expected_new_amount_buy = 15_000_000_000_000_u128;

        let alice_free_hdx_balance_after = Tokens::free_balance(HDX, &ALICE);
        let alice_reserved_hdx_balance_after = Tokens::reserved_balance_named(&reserve_id, HDX, &ALICE);
        let bob_hdx_balance_after = Tokens::free_balance(HDX, &BOB);

        let alice_dai_balance_after = Tokens::free_balance(DAI, &ALICE);
        let bob_dai_balance_after = Tokens::free_balance(DAI, &BOB);

        // Alice: HDX *free* balance remains the same, reserved balance decreases with amount_receive; DAI grows
        assert_eq!(alice_free_hdx_balance_after, alice_free_hdx_balance_before);
        assert_eq!(
            alice_reserved_hdx_balance_after,
            alice_reserved_hdx_balance_before - expected_amount_receive
        );
        assert_eq!(alice_dai_balance_after, alice_dai_balance_before + amount);

        // Bob: HDX grows, DAI decreases
        assert_eq!(bob_hdx_balance_after, bob_hdx_balance_before + expected_amount_receive);
        assert_eq!(bob_dai_balance_after, bob_dai_balance_before - amount);

        let order = OTC::orders(1).unwrap();
        assert_eq!(order.amount_buy, expected_new_amount_buy);

        expect_events(vec![Event::OrderPartiallyFilled {
            order_id: 1,
            who: BOB,
            amount: 5 * ONE,
            amount_receive: expected_amount_receive,
        }
        .into()]);
    });
}

#[test]
fn complete_fill_order_should_work_when_order_is_partially_fillable() {
    ExtBuilder::default().build().execute_with(|| {
        let reserve_id = named_reserve_identifier(1);

        // Arrange
        assert_ok!(OTC::place_order(
            Origin::signed(ALICE),
            DAI,
            HDX,
            20 * ONE,
            100 * ONE,
            true
        ));

        let alice_free_hdx_balance_before = Tokens::free_balance(HDX, &ALICE);
        let alice_reserved_hdx_balance_before = Tokens::reserved_balance_named(&reserve_id, HDX, &ALICE);
        let bob_hdx_balance_before = Tokens::free_balance(HDX, &BOB);

        let alice_dai_balance_before = Tokens::free_balance(DAI, &ALICE);
        let bob_dai_balance_before = Tokens::free_balance(DAI, &BOB);

        // Act
        let amount = 20 * ONE;
        assert_ok!(OTC::fill_order(Origin::signed(BOB), 1, DAI, amount));

        // Assert
        let order = OTC::orders(1);
        assert!(order.is_none());

        let alice_free_hdx_balance_after = Tokens::free_balance(HDX, &ALICE);
        let alice_reserved_hdx_balance_after = Tokens::reserved_balance_named(&reserve_id, HDX, &ALICE);
        let bob_hdx_balance_after = Tokens::free_balance(HDX, &BOB);

        let alice_dai_balance_after = Tokens::free_balance(DAI, &ALICE);
        let bob_dai_balance_after = Tokens::free_balance(DAI, &BOB);

        // Alice: HDX *free* balance remains the same, reserved balance decreases with amount_receive; DAI grows
        assert_eq!(alice_free_hdx_balance_after, alice_free_hdx_balance_before);
        assert_eq!(
            alice_reserved_hdx_balance_after,
            alice_reserved_hdx_balance_before - 100 * ONE
        );
        assert_eq!(alice_dai_balance_after, alice_dai_balance_before + amount);

        // Bob: HDX grows, DAI decreases
        assert_eq!(bob_hdx_balance_after, bob_hdx_balance_before + 100 * ONE);
        assert_eq!(bob_dai_balance_after, bob_dai_balance_before - amount);

        expect_events(vec![Event::OrderFilled {
            order_id: 1,
            who: BOB,
            amount: 20 * ONE,
        }
        .into()]);
    });
}

#[test]
fn complete_fill_order_should_work_when_order_is_not_partially_fillable() {
    ExtBuilder::default().build().execute_with(|| {
        let reserve_id = named_reserve_identifier(1);

        // Arrange
        assert_ok!(OTC::place_order(
            Origin::signed(ALICE),
            DAI,
            HDX,
            20 * ONE,
            100 * ONE,
            false
        ));

        let alice_free_hdx_balance_before = Tokens::free_balance(HDX, &ALICE);
        let alice_reserved_hdx_balance_before = Tokens::reserved_balance_named(&reserve_id, HDX, &ALICE);
        let bob_hdx_balance_before = Tokens::free_balance(HDX, &BOB);

        let alice_dai_balance_before = Tokens::free_balance(DAI, &ALICE);
        let bob_dai_balance_before = Tokens::free_balance(DAI, &BOB);

        // Act
        let amount = 20 * ONE;
        assert_ok!(OTC::fill_order(Origin::signed(BOB), 1, DAI, amount));

        // Assert
        let order = OTC::orders(1);
        assert!(order.is_none());

        let alice_free_hdx_balance_after = Tokens::free_balance(HDX, &ALICE);
        let alice_reserved_hdx_balance_after = Tokens::reserved_balance_named(&reserve_id, HDX, &ALICE);
        let bob_hdx_balance_after = Tokens::free_balance(HDX, &BOB);

        let alice_dai_balance_after = Tokens::free_balance(DAI, &ALICE);
        let bob_dai_balance_after = Tokens::free_balance(DAI, &BOB);

        // Alice: HDX *free* balance remains the same, reserved balance decreases with amount_receive; DAI grows
        assert_eq!(alice_free_hdx_balance_after, alice_free_hdx_balance_before);
        assert_eq!(
            alice_reserved_hdx_balance_after,
            alice_reserved_hdx_balance_before - 100 * ONE
        );
        assert_eq!(alice_dai_balance_after, alice_dai_balance_before + amount);

        // Bob: HDX grows, DAI decreases
        assert_eq!(bob_hdx_balance_after, bob_hdx_balance_before + 100 * ONE);
        assert_eq!(bob_dai_balance_after, bob_dai_balance_before - amount);

        expect_events(vec![Event::OrderFilled {
            order_id: 1,
            who: BOB,
            amount: 20 * ONE,
        }
        .into()]);
    });
}

#[test]
fn partial_fill_order_should_throw_error_when_remaining_amounts_are_too_low() {
    ExtBuilder::default().build().execute_with(|| {
        let reserve_id = named_reserve_identifier(1);

        // Arrange
        assert_ok!(OTC::place_order(
            Origin::signed(ALICE),
            DAI,
            HDX,
            20 * ONE,
            100 * ONE,
            true
        ));

        let alice_free_hdx_balance_before = Tokens::free_balance(HDX, &ALICE);
        let alice_reserved_hdx_balance_before = Tokens::reserved_balance_named(&reserve_id, HDX, &ALICE);
        let bob_hdx_balance_before = Tokens::free_balance(HDX, &BOB);

        let alice_dai_balance_before = Tokens::free_balance(DAI, &ALICE);
        let bob_dai_balance_before = Tokens::free_balance(DAI, &BOB);

        // Act
        let amount = 16 * ONE;
        assert_noop!(
            OTC::fill_order(Origin::signed(BOB), 1, DAI, amount),
            Error::<Test>::OrderAmountTooSmall
        );

        // Assert
        let alice_free_hdx_balance_after = Tokens::free_balance(HDX, &ALICE);
        let alice_reserved_hdx_balance_after = Tokens::reserved_balance_named(&reserve_id, HDX, &ALICE);
        let bob_hdx_balance_after = Tokens::free_balance(HDX, &BOB);

        let alice_dai_balance_after = Tokens::free_balance(DAI, &ALICE);
        let bob_dai_balance_after = Tokens::free_balance(DAI, &BOB);

        // Alice: nothing changes
        assert_eq!(alice_free_hdx_balance_after, alice_free_hdx_balance_before);
        assert_eq!(alice_reserved_hdx_balance_after, alice_reserved_hdx_balance_before);
        assert_eq!(alice_dai_balance_after, alice_dai_balance_before);

        // Bob: nothing changes
        assert_eq!(bob_hdx_balance_after, bob_hdx_balance_before);
        assert_eq!(bob_dai_balance_after, bob_dai_balance_before);
    });
}

#[test]
fn partial_fill_order_should_throw_error_when_order_is_not_partially_fillable() {
    ExtBuilder::default().build().execute_with(|| {
        let reserve_id = named_reserve_identifier(1);

        // Arrange
        assert_ok!(OTC::place_order(
            Origin::signed(ALICE),
            DAI,
            HDX,
            20 * ONE,
            100 * ONE,
            false
        ));

        let alice_free_hdx_balance_before = Tokens::free_balance(HDX, &ALICE);
        let alice_reserved_hdx_balance_before = Tokens::reserved_balance_named(&reserve_id, HDX, &ALICE);
        let bob_hdx_balance_before = Tokens::free_balance(HDX, &BOB);

        let alice_dai_balance_before = Tokens::free_balance(DAI, &ALICE);
        let bob_dai_balance_before = Tokens::free_balance(DAI, &BOB);

        // Act
        let amount = 5 * ONE;
        assert_noop!(
            OTC::fill_order(Origin::signed(BOB), 1, DAI, amount),
            Error::<Test>::OrderNotPartiallyFillable
        );

        // Assert
        let alice_free_hdx_balance_after = Tokens::free_balance(HDX, &ALICE);
        let alice_reserved_hdx_balance_after = Tokens::reserved_balance_named(&reserve_id, HDX, &ALICE);
        let bob_hdx_balance_after = Tokens::free_balance(HDX, &BOB);

        let alice_dai_balance_after = Tokens::free_balance(DAI, &ALICE);
        let bob_dai_balance_after = Tokens::free_balance(DAI, &BOB);

        // Alice: nothing changes
        assert_eq!(alice_free_hdx_balance_after, alice_free_hdx_balance_before);
        assert_eq!(alice_reserved_hdx_balance_after, alice_reserved_hdx_balance_before);
        assert_eq!(alice_dai_balance_after, alice_dai_balance_before);

        // Bob: nothing changes
        assert_eq!(bob_hdx_balance_after, bob_hdx_balance_before);
        assert_eq!(bob_dai_balance_after, bob_dai_balance_before);
    });
}

#[test]
fn fill_order_should_throw_error_when_insufficient_balance() {
    ExtBuilder::default().build().execute_with(|| {
        let reserve_id = named_reserve_identifier(1);

        // Arrange
        assert_ok!(OTC::place_order(
            Origin::signed(ALICE),
            DAI,
            HDX,
            200 * ONE,
            100 * ONE,
            true
        ));

        let alice_free_hdx_balance_before = Tokens::free_balance(HDX, &ALICE);
        let alice_reserved_hdx_balance_before = Tokens::reserved_balance_named(&reserve_id, HDX, &ALICE);
        let bob_hdx_balance_before = Tokens::free_balance(HDX, &BOB);

        let alice_dai_balance_before = Tokens::free_balance(DAI, &ALICE);
        let bob_dai_balance_before = Tokens::free_balance(DAI, &BOB);

        // Act
        let amount = 110 * ONE;
        assert_noop!(
            OTC::fill_order(Origin::signed(BOB), 1, DAI, amount),
            Error::<Test>::InsufficientBalance
        );

        // Assert
        let alice_free_hdx_balance_after = Tokens::free_balance(HDX, &ALICE);
        let alice_reserved_hdx_balance_after = Tokens::reserved_balance_named(&reserve_id, HDX, &ALICE);
        let bob_hdx_balance_after = Tokens::free_balance(HDX, &BOB);

        let alice_dai_balance_after = Tokens::free_balance(DAI, &ALICE);
        let bob_dai_balance_after = Tokens::free_balance(DAI, &BOB);

        // Alice: nothing changes
        assert_eq!(alice_free_hdx_balance_after, alice_free_hdx_balance_before);
        assert_eq!(alice_reserved_hdx_balance_after, alice_reserved_hdx_balance_before);
        assert_eq!(alice_dai_balance_after, alice_dai_balance_before);

        // Bob: nothing changes
        assert_eq!(bob_hdx_balance_after, bob_hdx_balance_before);
        assert_eq!(bob_dai_balance_after, bob_dai_balance_before);
    });
}

#[test]
fn fill_order_should_throw_error_when_amount_is_larger_than_order() {
    ExtBuilder::default().build().execute_with(|| {
        let reserve_id = named_reserve_identifier(1);

        // Arrange
        assert_ok!(OTC::place_order(
            Origin::signed(ALICE),
            DAI,
            HDX,
            20 * ONE,
            100 * ONE,
            true
        ));

        let alice_free_hdx_balance_before = Tokens::free_balance(HDX, &ALICE);
        let alice_reserved_hdx_balance_before = Tokens::reserved_balance_named(&reserve_id, HDX, &ALICE);
        let bob_hdx_balance_before = Tokens::free_balance(HDX, &BOB);

        let alice_dai_balance_before = Tokens::free_balance(DAI, &ALICE);
        let bob_dai_balance_before = Tokens::free_balance(DAI, &BOB);

        // Act
        let amount = 30 * ONE;
        assert_noop!(
            OTC::fill_order(Origin::signed(BOB), 1, DAI, amount),
            Error::<Test>::CannotFillMoreThanOrdered
        );

        // Assert
        let alice_free_hdx_balance_after = Tokens::free_balance(HDX, &ALICE);
        let alice_reserved_hdx_balance_after = Tokens::reserved_balance_named(&reserve_id, HDX, &ALICE);
        let bob_hdx_balance_after = Tokens::free_balance(HDX, &BOB);

        let alice_dai_balance_after = Tokens::free_balance(DAI, &ALICE);
        let bob_dai_balance_after = Tokens::free_balance(DAI, &BOB);

        // Alice: nothing changes
        assert_eq!(alice_free_hdx_balance_after, alice_free_hdx_balance_before);
        assert_eq!(alice_reserved_hdx_balance_after, alice_reserved_hdx_balance_before);
        assert_eq!(alice_dai_balance_after, alice_dai_balance_before);

        // Bob: nothing changes
        assert_eq!(bob_hdx_balance_after, bob_hdx_balance_before);
        assert_eq!(bob_dai_balance_after, bob_dai_balance_before);
    });
}
