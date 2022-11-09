// This file is part of Basilisk-node.

// Copyright (C) 2020-2022  Intergalactic, Limited (GIB).
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

use crate::traits::TransactionMultiPaymentDataProvider;
use crate::{
    error_to_invalid, AcceptedCurrencies, AcceptedCurrencyPrice, CurrencyBalanceCheck, Event, PaymentInfo, Price,
};
pub use crate::{mock::*, Config, Error};

use frame_support::{
    assert_err, assert_noop, assert_ok,
    dispatch::{DispatchError, Dispatchable},
    sp_runtime::{
        traits::{BadOrigin, SignedExtension},
        transaction_validity::ValidTransaction,
    },
    traits::Hooks,
    weights::{DispatchInfo, PostDispatchInfo, Weight},
};
use orml_traits::MultiCurrency;
use pallet_balances::Call as BalancesCall;
use pallet_transaction_payment::ChargeTransactionPayment;
use sp_std::marker::PhantomData;

const CALL: &<Test as frame_system::Config>::Call = &Call::Balances(BalancesCall::transfer { dest: 2, value: 69 });

#[test]
fn on_initialize_should_fill_storage_with_prices() {
    // Arrange
    ExtBuilder::default().build().execute_with(|| {
        // Act
        let current = System::block_number();
        PaymentPallet::on_finalize(current);
        // the block number is not important here and can stay the same
        PaymentPallet::on_initialize(current);

        // Assert
        // verify that all accepted currencies have the price set
        let iter = <AcceptedCurrencies<Test>>::iter();
        for (asset_id, _) in iter {
            assert!(<AcceptedCurrencyPrice<Test>>::contains_key(asset_id));
        }

        // fallback price
        assert_eq!(
            PaymentPallet::currency_price(SUPPORTED_CURRENCY),
            Some(Price::from_float(1.5))
        );
        // price from the spot price provider
        assert_eq!(
            PaymentPallet::currency_price(SUPPORTED_CURRENCY_WITH_PRICE),
            Some(Price::from_float(0.1))
        );
        // not supported
        assert_eq!(PaymentPallet::currency_price(UNSUPPORTED_CURRENCY), None);
    });
}

#[test]
fn on_finalize_should_remove_prices_from_storage() {
    // Arrange
    ExtBuilder::default().build().execute_with(|| {
        let current = System::block_number();

        // verify that the storage is not empty
        assert_eq!(
            PaymentPallet::currency_price(SUPPORTED_CURRENCY),
            Some(Price::from_float(1.5))
        );

        // Act
        PaymentPallet::on_finalize(current);

        // Assert
        let mut iter = <AcceptedCurrencyPrice<Test>>::iter_values();
        assert_eq!(iter.next(), None);
    });
}

#[test]
fn set_unsupported_currency() {
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(
            PaymentPallet::set_currency(Origin::signed(BOB), UNSUPPORTED_CURRENCY),
            Error::<Test>::UnsupportedCurrency
        );

        assert_eq!(PaymentPallet::get_currency(BOB), None);
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
        assert_eq!(Currencies::free_balance(SUPPORTED_CURRENCY, &FEE_RECEIVER), 1_543);
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
            PaymentPallet::set_currency(Origin::signed(BOB), SUPPORTED_CURRENCY_NO_BALANCE),
            Error::<Test>::ZeroBalance
        );

        assert_eq!(PaymentPallet::get_currency(BOB), None);
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
            let call = Call::PaymentPallet(crate::Call::<Test>::set_currency {
                currency: SUPPORTED_CURRENCY,
            });
            assert_noop!(
                call.dispatch(Origin::signed(CHARLIE)),
                orml_tokens::Error::<Test>::BalanceTooLow
            );

            let call = Call::PaymentPallet(crate::Call::<Test>::set_currency { currency: HDX });
            assert_noop!(
                call.dispatch(Origin::signed(CHARLIE)),
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
            let info = info_from_weight(Weight::from_ref_time(5));

            assert!(ChargeTransactionPayment::<Test>::from(0)
                .pre_dispatch(&CHARLIE, CALL, &info, len)
                .is_ok());

            assert_eq!(Balances::free_balance(CHARLIE), 100 - 5 - 5 - 10);
        });
}

#[test]
fn fee_payment_in_non_native_currency() {
    const CHARLIE: AccountId = 5;

    ExtBuilder::default()
        .base_weight(5)
        .account_tokens(CHARLIE, SUPPORTED_CURRENCY_WITH_PRICE, 10_000)
        .with_currencies(vec![(CHARLIE, SUPPORTED_CURRENCY_WITH_PRICE)])
        .build()
        .execute_with(|| {
            // Make sure Charlie ain't got a penny!
            assert_eq!(Balances::free_balance(CHARLIE), 0);

            let len = 1000;
            let info = info_from_weight(Weight::from_ref_time(5));

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
        .account_tokens(CHARLIE, SUPPORTED_CURRENCY, 100)
        .with_currencies(vec![(CHARLIE, SUPPORTED_CURRENCY)])
        .build()
        .execute_with(|| {
            let len = 1000;
            let info = info_from_weight(Weight::from_ref_time(5));

            assert!(ChargeTransactionPayment::<Test>::from(0)
                .pre_dispatch(&CHARLIE, CALL, &info, len)
                .is_err());

            assert_eq!(Tokens::free_balance(SUPPORTED_CURRENCY, &CHARLIE), 100);
        });
}

#[test]
fn add_new_accepted_currency() {
    ExtBuilder::default().base_weight(5).build().execute_with(|| {
        assert_ok!(PaymentPallet::add_currency(Origin::root(), 100, Price::from_float(1.1)));
        expect_events(vec![Event::CurrencyAdded { asset_id: 100 }.into()]);

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
        expect_events(vec![Event::CurrencyRemoved { asset_id: 100 }.into()]);

        assert_eq!(PaymentPallet::currencies(100), None);

        assert_noop!(
            PaymentPallet::remove_currency(Origin::root(), 100),
            Error::<Test>::UnsupportedCurrency
        );
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
            error_to_invalid(Error::<Test>::ZeroBalance).into()
        );
    });
}

#[test]
fn account_currency_works() {
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(PaymentPallet::account_currency(&ALICE), HDX);

        assert_ok!(PaymentPallet::set_currency(Origin::signed(ALICE), SUPPORTED_CURRENCY));
        expect_events(vec![Event::CurrencySet {
            account_id: ALICE,
            asset_id: SUPPORTED_CURRENCY,
        }
        .into()]);

        assert_eq!(PaymentPallet::account_currency(&ALICE), SUPPORTED_CURRENCY);

        assert_ok!(PaymentPallet::set_currency(Origin::signed(ALICE), HDX));
        assert_eq!(PaymentPallet::account_currency(&ALICE), HDX);
    });
}

#[test]
fn data_provider_works() {
    let go_to_next_block = || {
        use frame_support::traits::Hooks;

        let current = System::block_number();
        PaymentPallet::on_finalize(current);

        let next = current + 1;
        System::set_block_number(next);
        // Make sure the prices are up-to-date.
        PaymentPallet::on_initialize(next);
    };

    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(PaymentPallet::get_fee_receiver(), FEE_RECEIVER);
        assert_eq!(
            PaymentPallet::get_currency_and_price(&ALICE),
            Ok((<Test as Config>::NativeAssetId::get(), None))
        );

        assert_ok!(PaymentPallet::set_currency(Origin::signed(ALICE), SUPPORTED_CURRENCY));
        assert_eq!(
            PaymentPallet::get_currency_and_price(&ALICE),
            Ok((SUPPORTED_CURRENCY, Some(Price::from_float(1.5))))
        );

        assert_ok!(PaymentPallet::remove_currency(Origin::root(), SUPPORTED_CURRENCY));
        // price is removed at the end of the block
        go_to_next_block();
        assert_err!(
            PaymentPallet::get_currency_and_price(&ALICE),
            Error::<Test>::FallbackPriceNotFound
        );
    });
}

#[test]
fn transfer_set_fee_with_core_asset_should_work() {
    ExtBuilder::default().base_weight(5).build().execute_with(|| {
        let fb_account = <Test as Config>::FeeReceiver::get();
        let hdx_balance_before = Currencies::free_balance(HDX, &ALICE);
        let fb_balance_before = Currencies::free_balance(HDX, &fb_account);

        assert_ok!(PaymentPallet::transfer_set_fee(&ALICE));
        expect_events(vec![Event::FeeWithdrawn {
            account_id: ALICE,
            asset_id: HDX,
            native_fee_amount: 1029,
            non_native_fee_amount: 1029,
            destination_account_id: FEE_RECEIVER,
        }
        .into()]);

        assert_eq!(hdx_balance_before - 1029, Currencies::free_balance(HDX, &ALICE));
        assert_eq!(fb_balance_before + 1029, Currencies::free_balance(HDX, &fb_account));
    });
}

#[test]
fn transfer_set_fee_should_work() {
    ExtBuilder::default().base_weight(5).build().execute_with(|| {
        assert_ok!(PaymentPallet::set_currency(
            Origin::signed(ALICE),
            SUPPORTED_CURRENCY_WITH_PRICE
        ));

        let balance_before = Currencies::free_balance(SUPPORTED_CURRENCY_WITH_PRICE, &ALICE);
        let fb_acc_balance_before =
            Currencies::free_balance(SUPPORTED_CURRENCY_WITH_PRICE, &<Test as Config>::FeeReceiver::get());

        assert_ok!(PaymentPallet::transfer_set_fee(&ALICE));
        assert_eq!(
            balance_before - 102,
            Currencies::free_balance(SUPPORTED_CURRENCY_WITH_PRICE, &ALICE)
        );
        assert_eq!(
            fb_acc_balance_before + 102,
            Currencies::free_balance(SUPPORTED_CURRENCY_WITH_PRICE, &<Test as Config>::FeeReceiver::get())
        );
    });
}

#[test]
fn weight_to_fee_should_work() {
    ExtBuilder::default().base_weight(5).build().execute_with(|| {
        assert_eq!(PaymentPallet::weight_to_fee(Weight::from_ref_time(1024)), 1024);
        assert_eq!(PaymentPallet::weight_to_fee(Weight::from_ref_time(1)), 1);
        assert_eq!(PaymentPallet::weight_to_fee(Weight::from_ref_time(1025)), 1024);
        assert_eq!(PaymentPallet::weight_to_fee(Weight::from_ref_time(10000)), 1024);
    });
}

#[test]
fn check_balance_should_work() {
    ExtBuilder::default().base_weight(5).build().execute_with(|| {
        assert_ok!(PaymentPallet::check_balance(&ALICE, SUPPORTED_CURRENCY));
        assert_err!(
            PaymentPallet::check_balance(&ALICE, SUPPORTED_CURRENCY_NO_BALANCE).map_err(Into::<DispatchError>::into),
            Error::<Test>::ZeroBalance
        );
    });
}

/// create a transaction info struct from weight. Handy to avoid building the whole struct.
pub fn info_from_weight(w: Weight) -> DispatchInfo {
    // pays_fee: Pays::Yes -- class: DispatchClass::Normal
    DispatchInfo {
        weight: w,
        ..Default::default()
    }
}

fn post_info_from_weight(w: Weight) -> PostDispatchInfo {
    PostDispatchInfo {
        actual_weight: Some(w),
        pays_fee: Default::default(),
    }
}

fn default_post_info() -> PostDispatchInfo {
    PostDispatchInfo {
        actual_weight: None,
        pays_fee: Default::default(),
    }
}

#[test]
fn fee_should_be_transferred_when_paid_in_native_currency() {
    // Arrange
    const CHARLIE: AccountId = 5;

    ExtBuilder::default()
        .account_native_balance(CHARLIE, 100)
        .base_weight(5)
        .build()
        .execute_with(|| {
            let len = 10;
            let tip = 0;
            let dispatch_info = info_from_weight(Weight::from_ref_time(15));

            // Act
            let pre = ChargeTransactionPayment::<Test>::from(tip)
                .pre_dispatch(&CHARLIE, CALL, &dispatch_info, len)
                .unwrap();
            // Assert
            assert_eq!(
                pre,
                (tip, CHARLIE, Info(Some(Some(PaymentInfo::Native(5 + 15 + 10))), None))
            );

            assert_eq!(Balances::free_balance(CHARLIE), 100 - 30);
            assert_eq!(Balances::free_balance(FEE_RECEIVER), 0);

            // Act
            assert_ok!(ChargeTransactionPayment::<Test>::post_dispatch(
                Some(pre),
                &dispatch_info,
                &default_post_info(),
                len,
                &Ok(())
            ));
            // Assert
            assert_eq!(Balances::free_balance(CHARLIE), 100 - 30);
            assert_eq!(Balances::free_balance(FEE_RECEIVER), 30);
        });
}

#[test]
fn fee_should_be_withdrawn_when_paid_in_native_currency() {
    // Arrange
    const CHARLIE: AccountId = 5;

    ExtBuilder::default()
        .account_native_balance(CHARLIE, 100)
        .base_weight(5)
        .with_fee_withdrawal()
        .build()
        .execute_with(|| {
            let len = 10;
            let tip = 0;
            let dispatch_info = info_from_weight(Weight::from_ref_time(15));
            let previous_total_issuance = Balances::total_issuance();

            // Act
            let pre = ChargeTransactionPayment::<Test>::from(tip)
                .pre_dispatch(&CHARLIE, CALL, &dispatch_info, len)
                .unwrap();

            // Assert
            assert_eq!(Balances::free_balance(CHARLIE), 100 - 30);
            assert_eq!(Balances::free_balance(FEE_RECEIVER), 0);

            // Act
            assert_ok!(ChargeTransactionPayment::<Test>::post_dispatch(
                Some(pre),
                &dispatch_info,
                &default_post_info(),
                len,
                &Ok(())
            ));
            // Assert
            assert_eq!(Balances::free_balance(CHARLIE), 100 - 30);
            assert_eq!(Balances::free_balance(FEE_RECEIVER), 0);
            assert_eq!(Balances::total_issuance(), previous_total_issuance - 30);
        });
}

#[test]
fn fee_should_be_transferred_when_paid_in_native_currency_work_with_tip() {
    // Arrange
    const CHARLIE: AccountId = 5;

    ExtBuilder::default()
        .account_native_balance(CHARLIE, 100)
        .base_weight(5)
        .build()
        .execute_with(|| {
            let len = 10;
            let tip = 5;
            let dispatch_info = info_from_weight(Weight::from_ref_time(15));
            let post_dispatch_info = post_info_from_weight(Weight::from_ref_time(10));

            // Act
            let pre = ChargeTransactionPayment::<Test>::from(tip)
                .pre_dispatch(&CHARLIE, CALL, &dispatch_info, len)
                .unwrap();
            // Assert
            assert_eq!(
                pre,
                (
                    tip,
                    CHARLIE,
                    Info(Some(Some(PaymentInfo::Native(5 + 15 + 10 + tip))), None)
                )
            );

            assert_eq!(Balances::free_balance(CHARLIE), 100 - 5 - 10 - 15 - tip);
            assert_eq!(Balances::free_balance(FEE_RECEIVER), 0);

            // Act
            assert_ok!(ChargeTransactionPayment::<Test>::post_dispatch(
                Some(pre),
                &dispatch_info,
                &post_dispatch_info,
                len,
                &Ok(())
            ));

            // Assert
            assert_eq!(Balances::free_balance(CHARLIE), 100 - 5 - 10 - 10 - tip);
            assert_eq!(Balances::free_balance(FEE_RECEIVER), 30);
        });
}

#[test]
fn fee_should_be_withdrawn_when_paid_in_native_currency_work_with_tip() {
    // Arrange
    const CHARLIE: AccountId = 5;

    ExtBuilder::default()
        .account_native_balance(CHARLIE, 100)
        .base_weight(5)
        .with_fee_withdrawal()
        .build()
        .execute_with(|| {
            let len = 10;
            let tip = 5;
            let dispatch_info = info_from_weight(Weight::from_ref_time(15));
            let post_dispatch_info = post_info_from_weight(Weight::from_ref_time(10));
            let previous_total_issuance = Balances::total_issuance();

            // Act
            let pre = ChargeTransactionPayment::<Test>::from(tip)
                .pre_dispatch(&CHARLIE, CALL, &dispatch_info, len)
                .unwrap();

            // Assert
            assert_eq!(Balances::free_balance(CHARLIE), 100 - 5 - 10 - 15 - tip);
            assert_eq!(Balances::free_balance(FEE_RECEIVER), 0);

            // Act
            assert_ok!(ChargeTransactionPayment::<Test>::post_dispatch(
                Some(pre),
                &dispatch_info,
                &post_dispatch_info,
                len,
                &Ok(())
            ));

            // Assert
            assert_eq!(Balances::free_balance(CHARLIE), 100 - 5 - 10 - 10 - tip);
            assert_eq!(Balances::free_balance(FEE_RECEIVER), 0);
            assert_eq!(Balances::total_issuance(), previous_total_issuance - 30);
        });
}

#[test]
fn fee_should_be_transferred_when_paid_in_non_native_currency() {
    // Arrange
    const CHARLIE: AccountId = 5;

    ExtBuilder::default()
        .with_currencies(vec![(CHARLIE, SUPPORTED_CURRENCY)])
        .account_tokens(CHARLIE, SUPPORTED_CURRENCY, 10_000)
        .base_weight(5)
        .build()
        .execute_with(|| {
            let len = 10;
            let tip = 0;
            let dispatch_info = info_from_weight(Weight::from_ref_time(15));

            // Act
            let pre = ChargeTransactionPayment::<Test>::from(tip)
                .pre_dispatch(&CHARLIE, CALL, &dispatch_info, len)
                .unwrap();

            // Assert
            assert_eq!(
                pre,
                (
                    tip,
                    CHARLIE,
                    Info(
                        Some(Some(PaymentInfo::NonNative(
                            45,
                            SUPPORTED_CURRENCY,
                            Price::from_float(1.5)
                        ))),
                        None
                    )
                )
            );

            assert_eq!(Currencies::free_balance(SUPPORTED_CURRENCY, &CHARLIE), 10_000 - 45);
            assert_eq!(
                Currencies::free_balance(SUPPORTED_CURRENCY, &<Test as Config>::FeeReceiver::get()),
                0
            );
            assert_eq!(Balances::free_balance(CHARLIE), 0);
            assert_eq!(Balances::free_balance(FEE_RECEIVER), 0);

            // Act
            assert_ok!(ChargeTransactionPayment::<Test>::post_dispatch(
                Some(pre),
                &dispatch_info,
                &default_post_info(),
                len,
                &Ok(())
            ));

            // Assert
            assert_eq!(Currencies::free_balance(SUPPORTED_CURRENCY, &CHARLIE), 10_000 - 45);
            assert_eq!(
                Currencies::free_balance(SUPPORTED_CURRENCY, &<Test as Config>::FeeReceiver::get()),
                45
            );
            assert_eq!(Balances::free_balance(CHARLIE), 0);
            assert_eq!(Balances::free_balance(FEE_RECEIVER), 0);
        });
}

#[test]
fn fee_should_be_withdrawn_when_paid_in_non_native_currency() {
    // Arrange
    const CHARLIE: AccountId = 5;

    ExtBuilder::default()
        .with_currencies(vec![(CHARLIE, SUPPORTED_CURRENCY)])
        .account_tokens(CHARLIE, SUPPORTED_CURRENCY, 10_000)
        .base_weight(5)
        .with_fee_withdrawal()
        .build()
        .execute_with(|| {
            let len = 10;
            let tip = 0;
            let dispatch_info = info_from_weight(Weight::from_ref_time(15));
            let previous_total_issuance = Tokens::total_issuance(SUPPORTED_CURRENCY);

            // Act
            let pre = ChargeTransactionPayment::<Test>::from(tip)
                .pre_dispatch(&CHARLIE, CALL, &dispatch_info, len)
                .unwrap();

            // Assert
            assert_eq!(Currencies::free_balance(SUPPORTED_CURRENCY, &CHARLIE), 10_000 - 45);
            assert_eq!(
                Currencies::free_balance(SUPPORTED_CURRENCY, &<Test as Config>::FeeReceiver::get()),
                0
            );
            assert_eq!(Balances::free_balance(CHARLIE), 0);
            assert_eq!(Balances::free_balance(FEE_RECEIVER), 0);

            // Act
            assert_ok!(ChargeTransactionPayment::<Test>::post_dispatch(
                Some(pre),
                &dispatch_info,
                &default_post_info(),
                len,
                &Ok(())
            ));

            // Assert
            assert_eq!(Currencies::free_balance(SUPPORTED_CURRENCY, &CHARLIE), 10_000 - 45);
            assert_eq!(
                Currencies::free_balance(SUPPORTED_CURRENCY, &<Test as Config>::FeeReceiver::get()),
                0
            );
            assert_eq!(Balances::free_balance(CHARLIE), 0);
            assert_eq!(Balances::free_balance(FEE_RECEIVER), 0);
            assert_eq!(Tokens::total_issuance(SUPPORTED_CURRENCY), previous_total_issuance - 45);
        });
}

#[test]
fn fee_should_be_transferred_when_paid_in_non_native_currency_with_tip() {
    // Arrange
    const CHARLIE: AccountId = 5;

    ExtBuilder::default()
        .with_currencies(vec![(CHARLIE, SUPPORTED_CURRENCY)])
        .account_tokens(CHARLIE, SUPPORTED_CURRENCY, 10_000)
        .base_weight(5)
        .build()
        .execute_with(|| {
            let len = 10;
            let tip = 5;
            let dispatch_info = info_from_weight(Weight::from_ref_time(15));
            let post_dispatch_info = post_info_from_weight(Weight::from_ref_time(10));

            // Act
            let pre = ChargeTransactionPayment::<Test>::from(tip)
                .pre_dispatch(&CHARLIE, CALL, &dispatch_info, len)
                .unwrap();

            // Assert
            assert_eq!(
                pre,
                (
                    tip,
                    CHARLIE,
                    Info(
                        Some(Some(PaymentInfo::NonNative(
                            52,
                            SUPPORTED_CURRENCY,
                            Price::from_float(1.5)
                        ))),
                        None
                    )
                )
            );

            assert_eq!(Currencies::free_balance(SUPPORTED_CURRENCY, &CHARLIE), 10_000 - 52);
            assert_eq!(
                Currencies::free_balance(SUPPORTED_CURRENCY, &<Test as Config>::FeeReceiver::get()),
                0
            );
            assert_eq!(Balances::free_balance(CHARLIE), 0);
            assert_eq!(Balances::free_balance(FEE_RECEIVER), 0);

            // Act
            assert_ok!(ChargeTransactionPayment::<Test>::post_dispatch(
                Some(pre),
                &dispatch_info,
                &post_dispatch_info,
                len,
                &Ok(())
            ));

            // Assert
            assert_eq!(Currencies::free_balance(SUPPORTED_CURRENCY, &CHARLIE), 10_000 - 45);
            assert_eq!(
                Currencies::free_balance(SUPPORTED_CURRENCY, &<Test as Config>::FeeReceiver::get()),
                45
            );
            assert_eq!(Balances::free_balance(CHARLIE), 0);
            assert_eq!(Balances::free_balance(FEE_RECEIVER), 0);
        });
}

#[test]
fn fee_should_be_withdrawn_and_not_refunded_when_paid_in_non_native_currency_with_tip() {
    // Arrange
    const CHARLIE: AccountId = 5;

    ExtBuilder::default()
        .with_currencies(vec![(CHARLIE, SUPPORTED_CURRENCY)])
        .account_tokens(CHARLIE, SUPPORTED_CURRENCY, 10_000)
        .base_weight(5)
        .with_fee_withdrawal()
        .build()
        .execute_with(|| {
            let len = 10;
            let tip = 5;
            let dispatch_info = info_from_weight(Weight::from_ref_time(15));
            let post_dispatch_info = post_info_from_weight(Weight::from_ref_time(10));
            let previous_total_issuance = Tokens::total_issuance(SUPPORTED_CURRENCY);

            // Act
            let pre = ChargeTransactionPayment::<Test>::from(tip)
                .pre_dispatch(&CHARLIE, CALL, &dispatch_info, len)
                .unwrap();

            // Assert
            assert_eq!(Currencies::free_balance(SUPPORTED_CURRENCY, &CHARLIE), 10_000 - 52);
            assert_eq!(
                Currencies::free_balance(SUPPORTED_CURRENCY, &<Test as Config>::FeeReceiver::get()),
                0
            );
            assert_eq!(Balances::free_balance(CHARLIE), 0);
            assert_eq!(Balances::free_balance(FEE_RECEIVER), 0);

            // Act
            assert_ok!(ChargeTransactionPayment::<Test>::post_dispatch(
                Some(pre),
                &dispatch_info,
                &post_dispatch_info,
                len,
                &Ok(())
            ));

            // Assert
            assert_eq!(Currencies::free_balance(SUPPORTED_CURRENCY, &CHARLIE), 10_000 - 52);
            assert_eq!(
                Currencies::free_balance(SUPPORTED_CURRENCY, &<Test as Config>::FeeReceiver::get()),
                0
            );
            assert_eq!(Balances::free_balance(CHARLIE), 0);
            assert_eq!(Balances::free_balance(FEE_RECEIVER), 0);
            assert_eq!(Tokens::total_issuance(SUPPORTED_CURRENCY), previous_total_issuance - 52);
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
            let info = info_from_weight(Weight::from_ref_time(5));

            assert!(ChargeTransactionPayment::<Test>::from(0)
                .pre_dispatch(&CHARLIE, CALL, &info, len)
                .is_err());

            assert_eq!(Balances::free_balance(CHARLIE), 10);
            assert_eq!(Balances::free_balance(<Test as Config>::FeeReceiver::get()), 0);
        });
}

#[test]
fn fee_payment_in_non_native_currency_with_no_balance() {
    const CHARLIE: AccountId = 5;

    ExtBuilder::default()
        .base_weight(5)
        .account_tokens(CHARLIE, SUPPORTED_CURRENCY, 100)
        .with_currencies(vec![(CHARLIE, SUPPORTED_CURRENCY)])
        .build()
        .execute_with(|| {
            let len = 1000;
            let info = info_from_weight(Weight::from_ref_time(5));

            assert!(ChargeTransactionPayment::<Test>::from(0)
                .pre_dispatch(&CHARLIE, CALL, &info, len)
                .is_err());

            assert_eq!(Tokens::free_balance(SUPPORTED_CURRENCY, &CHARLIE), 100);
            assert_eq!(
                Tokens::free_balance(SUPPORTED_CURRENCY, &<Test as Config>::FeeReceiver::get()),
                0
            );
        });
}

#[test]
fn fee_payment_in_non_native_currency_with_no_price() {
    const CHARLIE: AccountId = 5;

    ExtBuilder::default()
        .base_weight(5)
        .account_tokens(CHARLIE, SUPPORTED_CURRENCY, 10_000)
        .with_currencies(vec![(CHARLIE, SUPPORTED_CURRENCY)])
        .build()
        .execute_with(|| {
            // Make sure Charlie ain't got a penny!
            assert_eq!(Balances::free_balance(CHARLIE), 0);

            let len = 10;
            let info = info_from_weight(Weight::from_ref_time(5));

            assert_eq!(Tokens::free_balance(SUPPORTED_CURRENCY, &FEE_RECEIVER), 0);

            assert!(ChargeTransactionPayment::<Test>::from(0)
                .pre_dispatch(&CHARLIE, CALL, &info, len)
                .is_ok());

            //Native balance check - Charlie should be still broke!
            assert_eq!(Balances::free_balance(CHARLIE), 0);

            assert_eq!(Tokens::free_balance(SUPPORTED_CURRENCY, &CHARLIE), 9970);
            assert_eq!(Tokens::free_balance(SUPPORTED_CURRENCY, &FEE_RECEIVER), 0);
        });
}

#[test]
fn fee_payment_in_unregistered_currency() {
    const CHARLIE: AccountId = 5;

    ExtBuilder::default()
        .base_weight(5)
        .account_tokens(CHARLIE, SUPPORTED_CURRENCY, 100)
        .with_currencies(vec![(CHARLIE, SUPPORTED_CURRENCY)])
        .build()
        .execute_with(|| {
            let len = 1000;
            let info = info_from_weight(Weight::from_ref_time(5));

            assert_ok!(PaymentPallet::remove_currency(Origin::root(), SUPPORTED_CURRENCY));

            assert!(ChargeTransactionPayment::<Test>::from(0)
                .pre_dispatch(&CHARLIE, CALL, &info, len)
                .is_err());

            assert_eq!(Tokens::free_balance(SUPPORTED_CURRENCY, &CHARLIE), 100);
        });
}

#[test]
fn fee_payment_non_native_insufficient_balance_with_no_pool() {
    const CHARLIE: AccountId = 5;

    ExtBuilder::default()
        .base_weight(5)
        .account_tokens(CHARLIE, SUPPORTED_CURRENCY, 100)
        .with_currencies(vec![(CHARLIE, SUPPORTED_CURRENCY)])
        .build()
        .execute_with(|| {
            let len = 1000;
            let info = info_from_weight(Weight::from_ref_time(5));

            assert!(ChargeTransactionPayment::<Test>::from(0)
                .pre_dispatch(&CHARLIE, CALL, &info, len)
                .is_err());

            assert_eq!(Tokens::free_balance(SUPPORTED_CURRENCY, &CHARLIE), 100);
        });
}

#[test]
fn fee_transfer_can_kill_account_when_paid_in_native() {
    // Arrange
    const CHARLIE: AccountId = 5;

    ExtBuilder::default()
        .account_native_balance(CHARLIE, 30)
        .base_weight(5)
        .build()
        .execute_with(|| {
            let len = 10;
            let tip = 0;
            let dispatch_info = info_from_weight(Weight::from_ref_time(15));

            // Act
            let pre = ChargeTransactionPayment::<Test>::from(tip)
                .pre_dispatch(&CHARLIE, CALL, &dispatch_info, len)
                .unwrap();

            // Assert
            assert_eq!(pre, (tip, CHARLIE, Info(Some(Some(PaymentInfo::Native(30))), None)));
            assert_eq!(Balances::free_balance(CHARLIE), 0);
            assert_eq!(Balances::free_balance(FEE_RECEIVER), 0);

            // Act
            assert_ok!(ChargeTransactionPayment::<Test>::post_dispatch(
                Some(pre),
                &dispatch_info,
                &default_post_info(),
                len,
                &Ok(())
            ));

            // Assert
            assert_eq!(Balances::free_balance(CHARLIE), 0); // zero balance indicates that the account can be killed
            assert_eq!(Balances::free_balance(FEE_RECEIVER), 30);
        });
}

#[test]
fn fee_withdrawal_cannot_kill_account_when_paid_in_native() {
    // Arrange
    const CHARLIE: AccountId = 5;

    ExtBuilder::default()
        .account_native_balance(CHARLIE, 30)
        .base_weight(5)
        .with_fee_withdrawal()
        .build()
        .execute_with(|| {
            let len = 10;
            let tip = 0;
            let dispatch_info = info_from_weight(Weight::from_ref_time(15));

            // Act & Assert
            assert_noop!(
                ChargeTransactionPayment::<Test>::from(tip).pre_dispatch(&CHARLIE, CALL, &dispatch_info, len),
                crate::InvalidTransaction::Payment
            );
        });
}

#[test]
fn fee_transfer_can_kill_account_when_paid_in_non_native() {
    // Arrange
    ExtBuilder::default()
        .with_currencies(vec![(ALICE, SUPPORTED_CURRENCY)])
        .base_weight(5)
        .build()
        .execute_with(|| {
            let len = 10;
            let tip = 0;
            let dispatch_info = info_from_weight(Weight::from_ref_time(15));

            assert_ok!(Currencies::withdraw(SUPPORTED_CURRENCY, &ALICE, INITIAL_BALANCE - 45));

            // Act
            let pre = ChargeTransactionPayment::<Test>::from(tip)
                .pre_dispatch(&ALICE, CALL, &dispatch_info, len)
                .unwrap();

            // Assert
            assert_eq!(
                pre,
                (
                    tip,
                    ALICE,
                    Info(
                        Some(Some(PaymentInfo::NonNative(
                            45,
                            SUPPORTED_CURRENCY,
                            Price::from_float(1.5)
                        ))),
                        None
                    )
                )
            );
            assert_eq!(Currencies::free_balance(SUPPORTED_CURRENCY, &ALICE), 0);
            assert_eq!(Currencies::free_balance(SUPPORTED_CURRENCY, &FEE_RECEIVER), 0);

            // Act
            assert_ok!(ChargeTransactionPayment::<Test>::post_dispatch(
                Some(pre),
                &dispatch_info,
                &default_post_info(),
                len,
                &Ok(())
            ));

            // Assert
            assert_eq!(Currencies::free_balance(SUPPORTED_CURRENCY, &ALICE), 0); // zero balance indicates that the account can be killed
            assert_eq!(Currencies::free_balance(SUPPORTED_CURRENCY, &FEE_RECEIVER), 45);
        });
}

#[test]
fn fee_withdrawal_can_kill_account_when_paid_in_non_native() {
    // Arrange
    ExtBuilder::default()
        .with_currencies(vec![(ALICE, SUPPORTED_CURRENCY)])
        .base_weight(5)
        .with_fee_withdrawal()
        .build()
        .execute_with(|| {
            let len = 10;
            let tip = 0;
            let dispatch_info = info_from_weight(Weight::from_ref_time(15));

            assert_ok!(Currencies::withdraw(SUPPORTED_CURRENCY, &ALICE, INITIAL_BALANCE - 45));

            // Act
            let pre = ChargeTransactionPayment::<Test>::from(tip)
                .pre_dispatch(&ALICE, CALL, &dispatch_info, len)
                .unwrap();

            // Assert
            assert_eq!(Currencies::free_balance(SUPPORTED_CURRENCY, &ALICE), 0);
            assert_eq!(Currencies::free_balance(SUPPORTED_CURRENCY, &FEE_RECEIVER), 0);

            // Act
            assert_ok!(ChargeTransactionPayment::<Test>::post_dispatch(
                Some(pre),
                &dispatch_info,
                &default_post_info(),
                len,
                &Ok(())
            ));

            // Assert
            assert_eq!(Currencies::free_balance(SUPPORTED_CURRENCY, &ALICE), 0); // zero balance indicates that the account can be killed
            assert_eq!(Currencies::free_balance(SUPPORTED_CURRENCY, &FEE_RECEIVER), 0);
        });
}

#[test]
fn set_and_remove_currency_on_lifecycle_callbacks() {
    const CHARLIE: AccountId = 5;

    ExtBuilder::default()
        .base_weight(5)
        .account_native_balance(CHARLIE, 10)
        .account_tokens(CHARLIE, SUPPORTED_CURRENCY, 10)
        .build()
        .execute_with(|| {
            assert_ok!(Tokens::transfer(Some(CHARLIE).into(), BOB, SUPPORTED_CURRENCY, 5));

            assert_eq!(Currencies::free_balance(SUPPORTED_CURRENCY, &CHARLIE), 5);
            assert_eq!(Currencies::free_balance(SUPPORTED_CURRENCY, &BOB), 5);
            // Bob's fee currency was set on transfer (due to account creation)
            assert_eq!(PaymentPallet::get_currency(BOB), Some(SUPPORTED_CURRENCY));

            // currency should be removed if account is killed
            assert_ok!(Tokens::transfer_all(
                Some(BOB).into(),
                CHARLIE,
                SUPPORTED_CURRENCY,
                false
            ));
            assert_eq!(PaymentPallet::get_currency(BOB), None);
        });
}

#[test]
fn currency_stays_around_until_reaping() {
    const CHARLIE: AccountId = 5;
    const DAVE: AccountId = 6;

    use frame_support::traits::fungibles::Balanced;

    ExtBuilder::default()
        .base_weight(5)
        .account_native_balance(CHARLIE, 10)
        .account_tokens(CHARLIE, SUPPORTED_CURRENCY, 10)
        .build()
        .execute_with(|| {
            // setup
            assert_ok!(<Tokens as Balanced<AccountId>>::deposit(HIGH_ED_CURRENCY, &DAVE, HIGH_ED * 2).map(|_| ()));
            assert_eq!(Currencies::free_balance(HIGH_ED_CURRENCY, &DAVE), HIGH_ED * 2);
            assert_eq!(PaymentPallet::get_currency(DAVE), Some(HIGH_ED_CURRENCY));

            // currency is not removed when account goes below existential deposit but stays around
            // until the account is reaped
            assert_ok!(Tokens::transfer(Some(DAVE).into(), BOB, HIGH_ED_CURRENCY, HIGH_ED + 1,));
            assert_eq!(PaymentPallet::get_currency(DAVE), Some(HIGH_ED_CURRENCY));
            assert_eq!(PaymentPallet::get_currency(BOB), Some(HIGH_ED_CURRENCY));

            // ... and account is reaped when all funds are transferred
            assert_ok!(Tokens::transfer_all(Some(DAVE).into(), BOB, HIGH_ED_CURRENCY, false));
            assert_eq!(PaymentPallet::get_currency(DAVE), None);
        });
}

#[test]
fn currency_is_removed_when_balance_hits_zero() {
    const CHARLIE: AccountId = 5;
    const DAVE: AccountId = 6;

    use frame_support::traits::fungibles::Balanced;

    ExtBuilder::default()
        .base_weight(5)
        .account_native_balance(CHARLIE, 10)
        .account_tokens(CHARLIE, SUPPORTED_CURRENCY, 10)
        .build()
        .execute_with(|| {
            // setup
            assert_ok!(<Tokens as Balanced<AccountId>>::deposit(SUPPORTED_CURRENCY_WITH_PRICE, &DAVE, 10).map(|_| ()));
            assert_eq!(Currencies::free_balance(SUPPORTED_CURRENCY_WITH_PRICE, &DAVE), 10);
            assert_eq!(PaymentPallet::get_currency(DAVE), Some(SUPPORTED_CURRENCY_WITH_PRICE));

            // currency is removed when all funds of tx fee currency are transferred (even if
            // account still has other funds)
            assert_ok!(Tokens::transfer(Some(CHARLIE).into(), DAVE, SUPPORTED_CURRENCY, 2));
            assert_ok!(Tokens::transfer_all(
                Some(DAVE).into(),
                BOB,
                SUPPORTED_CURRENCY_WITH_PRICE,
                false
            ));
            assert_eq!(PaymentPallet::get_currency(DAVE), None);
        });
}

#[test]
fn currency_is_not_changed_on_unrelated_account_activity() {
    const CHARLIE: AccountId = 5;
    const DAVE: AccountId = 6;

    use frame_support::traits::fungibles::Balanced;

    ExtBuilder::default()
        .base_weight(5)
        .account_native_balance(CHARLIE, 10)
        .account_tokens(CHARLIE, SUPPORTED_CURRENCY, 10)
        .build()
        .execute_with(|| {
            // setup
            assert_ok!(<Tokens as Balanced<AccountId>>::deposit(SUPPORTED_CURRENCY_WITH_PRICE, &DAVE, 10).map(|_| ()));
            assert_eq!(Currencies::free_balance(SUPPORTED_CURRENCY_WITH_PRICE, &DAVE), 10);
            assert_eq!(PaymentPallet::get_currency(DAVE), Some(SUPPORTED_CURRENCY_WITH_PRICE));

            // tx fee currency is not changed when a new currency is added to the account
            assert_ok!(Tokens::transfer(Some(CHARLIE).into(), DAVE, SUPPORTED_CURRENCY, 2));
            assert_eq!(PaymentPallet::get_currency(DAVE), Some(SUPPORTED_CURRENCY_WITH_PRICE));

            // tx fee currency is not removed when an unrelated account is removed
            assert_ok!(Tokens::transfer_all(
                Some(DAVE).into(),
                CHARLIE,
                SUPPORTED_CURRENCY,
                false
            ));
            assert_eq!(PaymentPallet::get_currency(DAVE), Some(SUPPORTED_CURRENCY_WITH_PRICE));
        });
}

#[test]
fn only_set_fee_currency_for_supported_currency() {
    const CHARLIE: AccountId = 5;

    ExtBuilder::default()
        .base_weight(5)
        .account_native_balance(CHARLIE, 10)
        .account_tokens(CHARLIE, UNSUPPORTED_CURRENCY, 10)
        .build()
        .execute_with(|| {
            assert_ok!(Tokens::transfer(Some(CHARLIE).into(), BOB, UNSUPPORTED_CURRENCY, 5));

            assert_eq!(Currencies::free_balance(UNSUPPORTED_CURRENCY, &CHARLIE), 5);
            assert_eq!(Currencies::free_balance(UNSUPPORTED_CURRENCY, &BOB), 5);
            // Bob's fee currency was not set on transfer (due to the currency being unsupported)
            assert_eq!(PaymentPallet::get_currency(BOB), None);
        });
}

#[test]
fn only_set_fee_currency_when_without_native_currency() {
    const CHARLIE: AccountId = 5;

    ExtBuilder::default()
        .account_native_balance(CHARLIE, 10)
        .build()
        .execute_with(|| {
            assert_eq!(PaymentPallet::get_currency(CHARLIE), None);

            assert_ok!(Currencies::transfer(
                Some(ALICE).into(),
                CHARLIE,
                SUPPORTED_CURRENCY,
                10,
            ));

            assert_eq!(PaymentPallet::get_currency(CHARLIE), None);
        });
}

#[test]
fn do_not_set_fee_currency_for_new_native_account() {
    const CHARLIE: AccountId = 5;
    const DAVE: AccountId = 6;

    ExtBuilder::default()
        .account_native_balance(CHARLIE, 10)
        .build()
        .execute_with(|| {
            assert_eq!(PaymentPallet::get_currency(DAVE), None);

            assert_ok!(Currencies::transfer(Some(CHARLIE).into(), DAVE, 0, 10,));

            assert_eq!(PaymentPallet::get_currency(DAVE), None);
        });
}

#[test]
fn returns_prices_for_supported_currencies() {
    use hydradx_traits::NativePriceOracle;

    ExtBuilder::default().build().execute_with(|| {
        // returns constant price of 1 for native asset
        assert_eq!(PaymentPallet::price(HdxAssetId::get()), Some(1.into()));
        // returns default price configured at genesis
        assert_eq!(PaymentPallet::price(SUPPORTED_CURRENCY_NO_BALANCE), Some(1.into()));
        assert_eq!(PaymentPallet::price(SUPPORTED_CURRENCY), Some(Price::from_float(1.5)));
        assert_eq!(PaymentPallet::price(HIGH_ED_CURRENCY), Some(3.into()));
        // returns spot price
        assert_eq!(
            PaymentPallet::price(SUPPORTED_CURRENCY_WITH_PRICE),
            Some(Price::from_float(0.1))
        );
    });
}
