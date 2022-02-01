// This file is part of Basilisk-node.

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

pub use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok, assert_storage_noop};
use pallet_transaction_payment::ChargeTransactionPayment;
use sp_runtime::traits::SignedExtension;

use crate::traits::{CurrencyWithdraw, PaymentWithdrawResult};
use crate::CurrencyBalanceCheck;
use crate::Price;
use frame_support::sp_runtime::transaction_validity::{InvalidTransaction, ValidTransaction};
use frame_support::weights::DispatchInfo;
use orml_traits::MultiCurrency;
use pallet_balances::Call as BalancesCall;
use sp_runtime::traits::BadOrigin;
use sp_std::marker::PhantomData;

const CALL: &<Test as frame_system::Config>::Call = &Call::Balances(BalancesCall::transfer { dest: 2, value: 69 });

#[test]
fn set_unsupported_currency() {
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(
            PaymentPallet::set_currency(Origin::signed(ALICE), UNSUPPORTED_CURRENCY),
            Error::<Test>::UnsupportedCurrency
        );

        assert_eq!(PaymentPallet::get_currency(ALICE), None);
    });
}

#[test]
fn set_supported_currency_without_spot_price() {
    ExtBuilder::default().base_weight(5).build().execute_with(|| {
        assert_ok!(PaymentPallet::set_currency(Origin::signed(ALICE), SUPPORTED_CURRENCY),);

        assert_eq!(PaymentPallet::get_currency(ALICE), Some(SUPPORTED_CURRENCY));

        assert_eq!(
            Currencies::free_balance(SUPPORTED_CURRENCY, &ALICE),
            999_999_999_998_457
        );
        assert_eq!(Currencies::free_balance(SUPPORTED_CURRENCY, &FALLBACK_ACCOUNT), 1_543);
    });
}

#[test]
fn set_supported_currency_with_price() {
    ExtBuilder::default().base_weight(5).build().execute_with(|| {
        assert_ok!(PaymentPallet::set_currency(
            Origin::signed(ALICE),
            SUPPORTED_CURRENCY_WITH_PRICE
        ),);

        assert_eq!(PaymentPallet::get_currency(ALICE), Some(SUPPORTED_CURRENCY_WITH_PRICE));

        assert_eq!(
            Currencies::free_balance(SUPPORTED_CURRENCY_WITH_PRICE, &ALICE),
            999_999_999_999_898
        );
    });
}

#[test]
fn set_supported_currency_with_no_balance() {
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(
            PaymentPallet::set_currency(Origin::signed(ALICE), SUPPORTED_CURRENCY_NO_BALANCE),
            Error::<Test>::ZeroBalance
        );

        assert_eq!(PaymentPallet::get_currency(ALICE), None);
    });
}

#[test]
fn set_native_currency() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(PaymentPallet::set_currency(Origin::signed(ALICE), HDX),);

        assert_eq!(PaymentPallet::get_currency(ALICE), Some(HDX));
    });
}

#[test]
fn set_native_currency_with_no_balance() {
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(
            PaymentPallet::set_currency(Origin::signed(BOB), HDX),
            Error::<Test>::ZeroBalance
        );
    });
}

#[test]
fn set_currency_with_insufficient_balance() {
    const CHARLIE: AccountId = 5;

    ExtBuilder::default()
        .base_weight(5)
        .account_native_balance(CHARLIE, 10)
        .account_tokens(CHARLIE, SUPPORTED_CURRENCY, 10)
        .build()
        .execute_with(|| {
            assert_noop!(
                PaymentPallet::set_currency(Origin::signed(CHARLIE), SUPPORTED_CURRENCY),
                orml_tokens::Error::<Test>::BalanceTooLow
            );

            assert_noop!(
                PaymentPallet::set_currency(Origin::signed(CHARLIE), HDX),
                pallet_balances::Error::<Test>::InsufficientBalance
            );

            assert_eq!(Currencies::free_balance(SUPPORTED_CURRENCY, &CHARLIE), 10);
            assert_eq!(Currencies::free_balance(HDX, &CHARLIE), 10);
        });
}

#[test]
fn fee_payment_in_native_currency() {
    const CHARLIE: AccountId = 5;

    ExtBuilder::default()
        .base_weight(5)
        .account_native_balance(CHARLIE, 100)
        .build()
        .execute_with(|| {
            let len = 10;
            let info = DispatchInfo {
                weight: 5,
                ..Default::default()
            };
            assert!(ChargeTransactionPayment::<Test>::from(0)
                .pre_dispatch(&CHARLIE, CALL, &info, len)
                .is_ok());

            assert_eq!(Balances::free_balance(CHARLIE), 100 - 5 - 5 - 10);
        });
}

#[test]
fn fee_payment_in_native_currency_with_no_balance() {
    const CHARLIE: AccountId = 5;

    ExtBuilder::default()
        .base_weight(5)
        .account_native_balance(CHARLIE, 10)
        .build()
        .execute_with(|| {
            let len = 10;
            let info = DispatchInfo {
                weight: 5,
                ..Default::default()
            };
            assert!(ChargeTransactionPayment::<Test>::from(0)
                .pre_dispatch(&CHARLIE, CALL, &info, len)
                .is_err());

            assert_eq!(Balances::free_balance(CHARLIE), 10);
        });
}

#[test]
fn fee_payment_in_non_native_currency() {
    const CHARLIE: AccountId = 5;

    ExtBuilder::default()
        .base_weight(5)
        .account_native_balance(CHARLIE, 0)
        .account_tokens(CHARLIE, SUPPORTED_CURRENCY_WITH_PRICE, 10_000)
        .with_currencies(vec![(CHARLIE, SUPPORTED_CURRENCY_WITH_PRICE)])
        .build()
        .execute_with(|| {
            // Make sure Charlie ain't got a penny!
            assert_eq!(Balances::free_balance(CHARLIE), 0);

            let len = 1000;
            let info = DispatchInfo {
                weight: 5,
                ..Default::default()
            };

            assert_eq!(Tokens::free_balance(SUPPORTED_CURRENCY_WITH_PRICE, &CHARLIE), 10_000);

            assert!(ChargeTransactionPayment::<Test>::from(0)
                .pre_dispatch(&CHARLIE, CALL, &info, len)
                .is_ok());

            //Native balance check - Charlie should be still broke!
            assert_eq!(Balances::free_balance(CHARLIE), 0);

            assert_eq!(Tokens::free_balance(SUPPORTED_CURRENCY_WITH_PRICE, &CHARLIE), 9899);
        });
}

#[test]
fn fee_payment_non_native_insufficient_balance() {
    const CHARLIE: AccountId = 5;

    ExtBuilder::default()
        .base_weight(5)
        .account_native_balance(CHARLIE, 0)
        .account_tokens(CHARLIE, SUPPORTED_CURRENCY, 1_00)
        .with_currencies(vec![(CHARLIE, SUPPORTED_CURRENCY)])
        .build()
        .execute_with(|| {
            let len = 1000;
            let info = DispatchInfo {
                weight: 5,
                ..Default::default()
            };

            assert!(ChargeTransactionPayment::<Test>::from(0)
                .pre_dispatch(&CHARLIE, CALL, &info, len)
                .is_err());

            assert_eq!(Tokens::free_balance(SUPPORTED_CURRENCY, &CHARLIE), 1_00);
        });
}

#[test]
fn add_new_accepted_currency() {
    ExtBuilder::default().base_weight(5).build().execute_with(|| {
        assert_ok!(PaymentPallet::add_currency(Origin::root(), 100, Price::from_float(1.1)));
        assert_eq!(PaymentPallet::currencies(100), Some(Price::from_float(1.1)));
        assert_noop!(
            PaymentPallet::add_currency(Origin::signed(ALICE), 1000, Price::from_float(1.2)),
            BadOrigin
        );
        assert_noop!(
            PaymentPallet::add_currency(Origin::root(), 100, Price::from(10)),
            Error::<Test>::AlreadyAccepted
        );
        assert_eq!(PaymentPallet::currencies(100), Some(Price::from_float(1.1)));
    });
}

#[test]
fn removed_accepted_currency() {
    ExtBuilder::default().base_weight(5).build().execute_with(|| {
        assert_ok!(PaymentPallet::add_currency(Origin::root(), 100, Price::from(3)));
        assert_eq!(PaymentPallet::currencies(100), Some(Price::from(3)));

        assert_noop!(PaymentPallet::remove_currency(Origin::signed(ALICE), 100), BadOrigin);

        assert_noop!(
            PaymentPallet::remove_currency(Origin::root(), 1000),
            Error::<Test>::UnsupportedCurrency
        );

        assert_ok!(PaymentPallet::remove_currency(Origin::root(), 100));

        assert_eq!(PaymentPallet::currencies(100), None);

        assert_noop!(
            PaymentPallet::remove_currency(Origin::root(), 100),
            Error::<Test>::UnsupportedCurrency
        );
    });
}

#[test]
fn fee_payment_in_non_native_currency_with_no_price() {
    const CHARLIE: AccountId = 5;

    ExtBuilder::default()
        .base_weight(5)
        .account_native_balance(CHARLIE, 0)
        .account_tokens(CHARLIE, SUPPORTED_CURRENCY, 10_000)
        .with_currencies(vec![(CHARLIE, SUPPORTED_CURRENCY)])
        .build()
        .execute_with(|| {
            // Make sure Charlie ain't got a penny!
            assert_eq!(Balances::free_balance(CHARLIE), 0);

            let len = 10;
            let info = DispatchInfo {
                weight: 5,
                ..Default::default()
            };

            assert_eq!(Tokens::free_balance(SUPPORTED_CURRENCY, &FALLBACK_ACCOUNT), 0);

            assert!(ChargeTransactionPayment::<Test>::from(0)
                .pre_dispatch(&CHARLIE, CALL, &info, len)
                .is_ok());

            //Native balance check - Charlie should be still broke!
            assert_eq!(Balances::free_balance(CHARLIE), 0);

            assert_eq!(Tokens::free_balance(SUPPORTED_CURRENCY, &CHARLIE), 9970);
            assert_eq!(Tokens::free_balance(SUPPORTED_CURRENCY, &FALLBACK_ACCOUNT), 30);
        });
}

#[test]
fn fee_payment_non_native_insufficient_balance_with_no_pool() {
    const CHARLIE: AccountId = 5;

    ExtBuilder::default()
        .base_weight(5)
        .account_native_balance(CHARLIE, 0)
        .account_tokens(CHARLIE, SUPPORTED_CURRENCY, 100)
        .with_currencies(vec![(CHARLIE, SUPPORTED_CURRENCY)])
        .build()
        .execute_with(|| {
            let len = 1000;
            let info = DispatchInfo {
                weight: 5,
                ..Default::default()
            };

            assert!(ChargeTransactionPayment::<Test>::from(0)
                .pre_dispatch(&CHARLIE, CALL, &info, len)
                .is_err());

            assert_eq!(Tokens::free_balance(SUPPORTED_CURRENCY, &CHARLIE), 100);
        });
}

#[test]
fn check_balance_extension_works() {
    const CHARLIE: AccountId = 5;

    ExtBuilder::default()
        .account_tokens(CHARLIE, SUPPORTED_CURRENCY, 1000)
        .build()
        .execute_with(|| {
            let call = Call::PaymentPallet(multi_payment::Call::set_currency {
                currency: SUPPORTED_CURRENCY,
            });
            let info = DispatchInfo::default();

            assert_eq!(
                CurrencyBalanceCheck::<Test>(PhantomData).validate(&CHARLIE, &call, &info, 150),
                Ok(ValidTransaction::default())
            );

            let call = Call::PaymentPallet(multi_payment::Call::add_currency {
                currency: SUPPORTED_CURRENCY,
                price: Price::from(1),
            });

            assert_eq!(
                CurrencyBalanceCheck::<Test>(PhantomData).validate(&CHARLIE, &call, &info, 150),
                Ok(ValidTransaction::default())
            );
        });
}

#[test]
fn check_balance_extension_fails() {
    const NOT_CHARLIE: AccountId = 6;

    ExtBuilder::default().build().execute_with(|| {
        let call = Call::PaymentPallet(multi_payment::Call::set_currency {
            currency: SUPPORTED_CURRENCY,
        });
        let info = DispatchInfo::default();

        assert_eq!(
            CurrencyBalanceCheck::<Test>(PhantomData).validate(&NOT_CHARLIE, &call, &info, 150),
            InvalidTransaction::Custom(Error::<Test>::ZeroBalance.as_u8()).into()
        );
    });
}

#[test]
fn account_currency_works() {
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(PaymentPallet::account_currency(&ALICE), HDX);

        assert_ok!(PaymentPallet::set_currency(Origin::signed(ALICE), SUPPORTED_CURRENCY));
        assert_eq!(PaymentPallet::account_currency(&ALICE), SUPPORTED_CURRENCY);

        assert_ok!(PaymentPallet::set_currency(Origin::signed(ALICE), HDX));
        assert_eq!(PaymentPallet::account_currency(&ALICE), HDX);
    });
}

#[test]
fn withdraw_currency_should_work() {
    ExtBuilder::default().base_weight(5).build().execute_with(|| {
        assert_storage_noop!(PaymentPallet::withdraw_fee_non_native(&ALICE, 10000).unwrap());

        assert_ok!(PaymentPallet::set_currency(
            Origin::signed(ALICE),
            SUPPORTED_CURRENCY_WITH_PRICE
        ));

        assert_ok!(PaymentPallet::withdraw_fee_non_native(&ALICE, 10000));

        assert_eq!(
            999999999998898,
            Currencies::free_balance(SUPPORTED_CURRENCY_WITH_PRICE, &ALICE)
        );

        assert_eq!(
            1102,
            Currencies::free_balance(
                SUPPORTED_CURRENCY_WITH_PRICE,
                &PaymentPallet::fallback_account().unwrap()
            )
        );
    });
}

#[test]
fn withdraw_set_fee_with_core_asset_should_work() {
    ExtBuilder::default().base_weight(5).build().execute_with(|| {
        let hdx_balance_before = Currencies::free_balance(HDX, &ALICE);
        assert_ok!(PaymentPallet::withdraw_set_fee(&ALICE));
        assert_eq!(hdx_balance_before - 1029, Currencies::free_balance(HDX, &ALICE));
    });
}

#[test]
fn withdraw_set_fee_should_work() {
    ExtBuilder::default().base_weight(5).build().execute_with(|| {
        assert_ok!(PaymentPallet::set_currency(
            Origin::signed(ALICE),
            SUPPORTED_CURRENCY_WITH_PRICE
        ));

        let balance_before = Currencies::free_balance(SUPPORTED_CURRENCY_WITH_PRICE, &ALICE);
        let fb_acc_balance_before = Currencies::free_balance(
            SUPPORTED_CURRENCY_WITH_PRICE,
            &PaymentPallet::fallback_account().unwrap(),
        );

        assert_ok!(PaymentPallet::withdraw_set_fee(&ALICE));
        assert_eq!(
            balance_before - 102,
            Currencies::free_balance(SUPPORTED_CURRENCY_WITH_PRICE, &ALICE)
        );
        assert_eq!(
            fb_acc_balance_before + 102,
            Currencies::free_balance(
                SUPPORTED_CURRENCY_WITH_PRICE,
                &PaymentPallet::fallback_account().unwrap()
            )
        );
    });
}

#[test]
fn weight_to_fee_should_work() {
    ExtBuilder::default().base_weight(5).build().execute_with(|| {
        assert_eq!(PaymentPallet::weight_to_fee(1024), 1024);
        assert_eq!(PaymentPallet::weight_to_fee(1), 1);
        assert_eq!(PaymentPallet::weight_to_fee(1025), 1024);
        assert_eq!(PaymentPallet::weight_to_fee(10000), 1024);
    });
}

#[test]
fn check_balance_should_work() {
    ExtBuilder::default().base_weight(5).build().execute_with(|| {
        assert_ok!(PaymentPallet::check_balance(&ALICE, SUPPORTED_CURRENCY));
        assert_eq!(
            PaymentPallet::check_balance(&ALICE, SUPPORTED_CURRENCY_NO_BALANCE)
                .err()
                .unwrap()
                .as_u8(),
            1_u8
        );
    });
}

#[test]
fn withdraw_with_price_should_work() {
    ExtBuilder::default().base_weight(5).build().execute_with(|| {
        assert_eq!(
            PaymentPallet::withdraw(&ALICE, 1000).unwrap(),
            PaymentWithdrawResult::Native
        );

        assert_ok!(PaymentPallet::set_currency(
            Origin::signed(ALICE),
            SUPPORTED_CURRENCY_WITH_PRICE
        ));
        assert_eq!(
            PaymentPallet::withdraw(&ALICE, 1000).unwrap(),
            PaymentWithdrawResult::Transferred
        );
    });
}

#[test]
fn withdraw_should_not_work() {
    ExtBuilder::default().base_weight(5).build().execute_with(|| {
        assert_ok!(PaymentPallet::set_currency(Origin::signed(ALICE), SUPPORTED_CURRENCY));

        assert_ok!(PaymentPallet::remove_currency(Origin::root(), SUPPORTED_CURRENCY));
        assert_noop!(
            PaymentPallet::withdraw(&ALICE, 1000),
            Error::<Test>::FallbackPriceNotFound
        );
    });
}
