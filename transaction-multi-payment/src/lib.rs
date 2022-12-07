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

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

pub mod weights;

use weights::WeightInfo;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;
mod traits;

use frame_support::{
    dispatch::DispatchResult,
    ensure,
    traits::{Currency, ExistenceRequirement, Get, Imbalance, OnUnbalanced, WithdrawReasons},
    weights::{DispatchClass, WeightToFee},
};
use frame_system::ensure_signed;
use sp_runtime::{
    traits::{DispatchInfoOf, One, PostDispatchInfoOf, Saturating, Zero},
    transaction_validity::{InvalidTransaction, TransactionValidityError},
    FixedU128, ModuleError,
};
use sp_std::prelude::*;

use pallet_transaction_payment::OnChargeTransaction;
use sp_std::marker::PhantomData;

use frame_support::sp_runtime::FixedPointNumber;
use frame_support::sp_runtime::FixedPointOperand;
use frame_support::weights::{Pays, Weight};
use hydradx_traits::{pools::SpotPriceProvider, NativePriceOracle};
use orml_traits::{Happened, MultiCurrency, MultiCurrencyExtended};

use codec::{Decode, Encode};
use frame_support::sp_runtime::traits::SignedExtension;
use frame_support::sp_runtime::transaction_validity::{TransactionValidity, ValidTransaction};
use frame_support::traits::IsSubType;

use scale_info::TypeInfo;

pub use crate::traits::*;
use frame_support::dispatch::DispatchError;

type AssetIdOf<T> = <<T as Config>::Currencies as MultiCurrency<<T as frame_system::Config>::AccountId>>::CurrencyId;
type BalanceOf<T> = <<T as Config>::Currencies as MultiCurrency<<T as frame_system::Config>::AccountId>>::Balance;
type NegativeImbalanceOf<C, T> = <C as Currency<<T as frame_system::Config>::AccountId>>::NegativeImbalance;

/// Spot price type
pub type Price = FixedU128;

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::OriginFor;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
        fn on_initialize(_n: T::BlockNumber) -> Weight {
            let native_asset = T::NativeAssetId::get();

            let mut weight: u64 = 0;

            for (asset_id, fallback_price) in <AcceptedCurrencies<T>>::iter() {
                let maybe_price = T::SpotPriceProvider::spot_price(native_asset, asset_id);

                let price = maybe_price.unwrap_or(fallback_price);

                AcceptedCurrencyPrice::<T>::insert(asset_id, price);

                weight += T::WeightInfo::get_spot_price().ref_time();
            }

            Weight::from_ref_time(weight)
        }

        fn on_finalize(_n: T::BlockNumber) {
            let _ = <AcceptedCurrencyPrice<T>>::clear(u32::MAX, None);
        }
    }

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_transaction_payment::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The origin which can add/remove accepted currencies
        type AcceptedCurrencyOrigin: EnsureOrigin<Self::Origin>;

        /// Multi Currency
        type Currencies: MultiCurrencyExtended<Self::AccountId>;

        /// Spot price provider
        type SpotPriceProvider: SpotPriceProvider<AssetIdOf<Self>, Price = Price>;

        /// Weight information for the extrinsics.
        type WeightInfo: WeightInfo;

        /// Should fee be paid for setting a currency
        #[pallet::constant]
        type WithdrawFeeForSetCurrency: Get<Pays>;

        /// Convert a weight value into a deductible fee based on the currency type.
        type WeightToFee: WeightToFee<Balance = BalanceOf<Self>>;

        /// Native Asset
        #[pallet::constant]
        type NativeAssetId: Get<AssetIdOf<Self>>;

        /// Account where fees are deposited
        #[pallet::constant]
        type FeeReceiver: Get<Self::AccountId>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// CurrencySet
        /// [who, currency]
        CurrencySet {
            account_id: T::AccountId,
            asset_id: AssetIdOf<T>,
        },

        /// New accepted currency added
        /// [currency]
        CurrencyAdded { asset_id: AssetIdOf<T> },

        /// Accepted currency removed
        /// [currency]
        CurrencyRemoved { asset_id: AssetIdOf<T> },

        /// Transaction fee paid in non-native currency
        /// [Account, Currency, Native fee amount, Non-native fee amount, Destination account]
        FeeWithdrawn {
            account_id: T::AccountId,
            asset_id: AssetIdOf<T>,
            native_fee_amount: BalanceOf<T>,
            non_native_fee_amount: BalanceOf<T>,
            destination_account_id: T::AccountId,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Selected currency is not supported.
        UnsupportedCurrency,

        /// Account balance should be non-zero.
        ZeroBalance,

        /// Currency is already in the list of accepted currencies.
        AlreadyAccepted,

        /// It is not allowed to add Core Asset as accepted currency. Core asset is accepted by design.
        CoreAssetNotAllowed,

        /// Fallback price cannot be zero.
        ZeroPrice,

        /// Fallback price was not found.
        FallbackPriceNotFound,

        /// Math overflow
        Overflow,
    }

    /// Account currency map
    #[pallet::storage]
    #[pallet::getter(fn get_currency)]
    pub type AccountCurrencyMap<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, AssetIdOf<T>, OptionQuery>;

    /// Curated list of currencies which fees can be paid mapped to corresponding fallback price
    #[pallet::storage]
    #[pallet::getter(fn currencies)]
    pub type AcceptedCurrencies<T: Config> = StorageMap<_, Twox64Concat, AssetIdOf<T>, Price, OptionQuery>;

    /// Asset prices from the spot price provider or the fallback price if the price is not available. Updated at the beginning of every block.
    #[pallet::storage]
    #[pallet::getter(fn currency_price)]
    pub type AcceptedCurrencyPrice<T: Config> = StorageMap<_, Twox64Concat, AssetIdOf<T>, Price, OptionQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub currencies: Vec<(AssetIdOf<T>, Price)>,
        pub account_currencies: Vec<(T::AccountId, AssetIdOf<T>)>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            GenesisConfig {
                currencies: vec![],
                account_currencies: vec![],
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            for (asset, price) in &self.currencies {
                AcceptedCurrencies::<T>::insert(asset, price);
            }

            for (account, asset) in &self.account_currencies {
                <AccountCurrencyMap<T>>::insert(account, asset);
            }
        }
    }
    #[pallet::call]
    impl<T: Config> Pallet<T>
    where
        BalanceOf<T>: FixedPointOperand,
    {
        /// Set selected currency for given account.
        ///
        /// This allows to set a currency for an account in which all transaction fees will be paid.
        /// Account balance cannot be zero.
        ///
        /// Chosen currency must be in the list of accepted currencies.
        ///
        /// When currency is set, fixed fee is withdrawn from the account to pay for the currency change
        ///
        /// Emits `CurrencySet` event when successful.
        #[pallet::weight((<T as Config>::WeightInfo::set_currency(), DispatchClass::Normal, Pays::No))]
        pub fn set_currency(origin: OriginFor<T>, currency: AssetIdOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            if currency == T::NativeAssetId::get() || AcceptedCurrencies::<T>::contains_key(currency) {
                if T::Currencies::free_balance(currency, &who) == BalanceOf::<T>::zero() {
                    return Err(Error::<T>::ZeroBalance.into());
                }

                <AccountCurrencyMap<T>>::insert(who.clone(), currency);

                if T::WithdrawFeeForSetCurrency::get() == Pays::Yes {
                    Self::transfer_set_fee(&who)?;
                }

                Self::deposit_event(Event::CurrencySet {
                    account_id: who,
                    asset_id: currency,
                });

                return Ok(());
            }

            Err(Error::<T>::UnsupportedCurrency.into())
        }

        /// Add a currency to the list of accepted currencies.
        ///
        /// Only member can perform this action.
        ///
        /// Currency must not be already accepted. Core asset id cannot be explicitly added.
        ///
        /// Emits `CurrencyAdded` event when successful.
        #[pallet::weight((<T as Config>::WeightInfo::add_currency(), DispatchClass::Normal, Pays::No))]
        pub fn add_currency(origin: OriginFor<T>, currency: AssetIdOf<T>, price: Price) -> DispatchResult {
            T::AcceptedCurrencyOrigin::ensure_origin(origin)?;

            ensure!(currency != T::NativeAssetId::get(), Error::<T>::CoreAssetNotAllowed);

            AcceptedCurrencies::<T>::try_mutate_exists(currency, |maybe_price| -> DispatchResult {
                if maybe_price.is_some() {
                    return Err(Error::<T>::AlreadyAccepted.into());
                }

                *maybe_price = Some(price);
                Self::deposit_event(Event::CurrencyAdded { asset_id: currency });
                Ok(())
            })
        }

        /// Remove currency from the list of supported currencies
        /// Only selected members can perform this action
        ///
        /// Core asset cannot be removed.
        ///
        /// Emits `CurrencyRemoved` when successful.
        #[pallet::weight((<T as Config>::WeightInfo::remove_currency(), DispatchClass::Normal, Pays::No))]
        pub fn remove_currency(origin: OriginFor<T>, currency: AssetIdOf<T>) -> DispatchResult {
            T::AcceptedCurrencyOrigin::ensure_origin(origin)?;

            ensure!(currency != T::NativeAssetId::get(), Error::<T>::CoreAssetNotAllowed);

            AcceptedCurrencies::<T>::try_mutate(currency, |x| -> DispatchResult {
                if x.is_none() {
                    return Err(Error::<T>::UnsupportedCurrency.into());
                }

                *x = None;

                Self::deposit_event(Event::CurrencyRemoved { asset_id: currency });

                Ok(())
            })
        }
    }
}

impl<T: Config> Pallet<T>
where
    BalanceOf<T>: FixedPointOperand,
{
    fn account_currency(who: &T::AccountId) -> AssetIdOf<T> {
        Pallet::<T>::get_currency(who).unwrap_or_else(T::NativeAssetId::get)
    }

    /// Transfer fee without executing an AMM trade
    pub fn transfer_set_fee(who: &T::AccountId) -> DispatchResult {
        let base_fee = Self::weight_to_fee(T::BlockWeights::get().get(DispatchClass::Normal).base_extrinsic);
        let adjusted_weight_fee = Self::weight_to_fee(T::WeightInfo::set_currency());
        let fee = base_fee.saturating_add(adjusted_weight_fee);
        let (currency, maybe_price) = Self::get_currency_and_price(who)?;

        let amount = match maybe_price {
            None => fee,
            Some(price) => price.checked_mul_int(fee).ok_or(Error::<T>::Overflow)?,
        };

        T::Currencies::transfer(currency, who, &T::FeeReceiver::get(), amount)?;

        Self::deposit_event(Event::FeeWithdrawn {
            account_id: who.clone(),
            asset_id: currency,
            native_fee_amount: fee,
            non_native_fee_amount: amount,
            destination_account_id: T::FeeReceiver::get(),
        });

        Ok(())
    }

    fn weight_to_fee(weight: Weight) -> BalanceOf<T> {
        // cap the weight to the maximum defined in runtime, otherwise it will be the
        // `Bounded` maximum of its data type, which is not desired.
        let capped_weight: Weight = weight.min(T::BlockWeights::get().max_block);
        <T as Config>::WeightToFee::weight_to_fee(&capped_weight)
    }

    fn check_balance(account: &T::AccountId, currency: AssetIdOf<T>) -> Result<(), Error<T>> {
        if T::Currencies::free_balance(currency, account) == BalanceOf::<T>::zero() {
            return Err(Error::<T>::ZeroBalance);
        };
        Ok(())
    }

    fn get_currency_and_price(
        who: &<T as frame_system::Config>::AccountId,
    ) -> Result<(AssetIdOf<T>, Option<Price>), DispatchError> {
        let native_currency = T::NativeAssetId::get();
        let currency = Self::account_currency(who);
        if currency == T::NativeAssetId::get() {
            Ok((native_currency, None))
        } else {
            let price = if let Some(spot_price) = Self::currency_price(currency) {
                spot_price
            } else {
                // If not loaded in on_init, let's try first the spot price provider again
                // This is unlikely scenario as the price would be retrieved in on_init for each block
                if let Some(spot_price) = T::SpotPriceProvider::spot_price(T::NativeAssetId::get(), currency) {
                    spot_price
                } else {
                    Self::currencies(currency).ok_or(Error::<T>::FallbackPriceNotFound)?
                }
            };

            Ok((currency, Some(price)))
        }
    }

    // This method is required by WithdrawFee
    /// Execute a trade to buy HDX and sell selected currency.
    pub fn withdraw_fee_non_native(
        who: &T::AccountId,
        fee: BalanceOf<T>,
    ) -> Result<PaymentWithdrawResult, DispatchError> {
        let currency = Self::account_currency(who);

        if currency == T::NativeAssetId::get() {
            Ok(PaymentWithdrawResult::Native)
        } else {
            let price = if let Some(spot_price) = Self::currency_price(currency) {
                spot_price
            } else {
                // If not loaded in on_init, let's try first the spot price provider again
                // This is unlikely scenario as the price would be retrieved in on_init for each block
                if let Some(spot_price) = T::SpotPriceProvider::spot_price(T::NativeAssetId::get(), currency) {
                    spot_price
                } else {
                    Self::currencies(currency).ok_or(Error::<T>::FallbackPriceNotFound)?
                }
            };

            let amount = price.checked_mul_int(fee).ok_or(Error::<T>::Overflow)?;

            T::Currencies::withdraw(currency, who, amount)?;

            Self::deposit_event(Event::FeeWithdrawn {
                account_id: who.clone(),
                asset_id: currency,
                native_fee_amount: fee,
                non_native_fee_amount: amount,
                destination_account_id: T::FeeReceiver::get(),
            });

            Ok(PaymentWithdrawResult::Transferred)
        }
    }
}

impl<T: Config> TransactionMultiPaymentDataProvider<<T as frame_system::Config>::AccountId, AssetIdOf<T>, Price>
    for Pallet<T>
where
    BalanceOf<T>: FixedPointOperand,
{
    fn get_currency_and_price(
        who: &<T as frame_system::Config>::AccountId,
    ) -> Result<(AssetIdOf<T>, Option<Price>), DispatchError> {
        Self::get_currency_and_price(who)
    }

    fn get_fee_receiver() -> <T as frame_system::Config>::AccountId {
        T::FeeReceiver::get()
    }
}

/// Deposits all fees to some account
pub struct DepositAll<T>(PhantomData<T>);

impl<T: Config> DepositFee<T::AccountId, AssetIdOf<T>, BalanceOf<T>> for DepositAll<T> {
    fn deposit_fee(who: &T::AccountId, currency: AssetIdOf<T>, amount: BalanceOf<T>) -> DispatchResult {
        <T as Config>::Currencies::deposit(currency, who, amount)?;
        Ok(())
    }
}

/// Implements the transaction payment for native as well as non-native currencies
pub struct TransferFees<MC, DP, DF>(PhantomData<(MC, DP, DF)>);

impl<T, MC, DP, DF> OnChargeTransaction<T> for TransferFees<MC, DP, DF>
where
    T: Config,
    MC: MultiCurrency<<T as frame_system::Config>::AccountId>,
    AssetIdOf<T>: Into<MC::CurrencyId>,
    MC::Balance: FixedPointOperand,
    DP: TransactionMultiPaymentDataProvider<T::AccountId, AssetIdOf<T>, Price>,
    DF: DepositFee<T::AccountId, MC::CurrencyId, MC::Balance>,
{
    type LiquidityInfo = Option<PaymentInfo<Self::Balance, AssetIdOf<T>, Price>>;
    type Balance = <MC as MultiCurrency<<T as frame_system::Config>::AccountId>>::Balance;

    /// Withdraw the predicted fee from the transaction origin.
    ///
    /// Note: The `fee` already includes the `tip`.
    fn withdraw_fee(
        who: &T::AccountId,
        _call: &T::Call,
        _info: &DispatchInfoOf<T::Call>,
        fee: Self::Balance,
        _tip: Self::Balance,
    ) -> Result<Self::LiquidityInfo, TransactionValidityError> {
        if fee.is_zero() {
            return Ok(None);
        }
        // get the currency in which fees are paid. In case of non-native currency, the price is required to calculate final fee.
        let currency_data = DP::get_currency_and_price(who)
            .map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;

        match currency_data {
            (_, None) => match MC::withdraw(T::NativeAssetId::get().into(), who, fee) {
                Ok(()) => Ok(Some(PaymentInfo::Native(fee))),
                Err(_) => Err(InvalidTransaction::Payment.into()),
            },
            (currency, Some(price)) => {
                let converted_fee = price
                    .checked_mul_int(fee)
                    .ok_or(TransactionValidityError::Invalid(InvalidTransaction::Payment))?;
                match MC::withdraw(currency.into(), who, converted_fee) {
                    Ok(()) => Ok(Some(PaymentInfo::NonNative(converted_fee, currency, price))),
                    Err(_) => Err(InvalidTransaction::Payment.into()),
                }
            }
        }
    }

    /// Since the predicted fee might have been too high, parts of the fee may
    /// be refunded.
    ///
    /// Note: The `fee` already includes the `tip`.
    fn correct_and_deposit_fee(
        who: &T::AccountId,
        _dispatch_info: &DispatchInfoOf<T::Call>,
        _post_info: &PostDispatchInfoOf<T::Call>,
        corrected_fee: Self::Balance,
        tip: Self::Balance,
        already_withdrawn: Self::LiquidityInfo,
    ) -> Result<(), TransactionValidityError> {
        let fee_receiver = DP::get_fee_receiver();

        if let Some(paid) = already_withdrawn {
            // Calculate how much refund we should return
            let (currency, refund, fee, tip) = match paid {
                PaymentInfo::Native(paid_fee) => (
                    T::NativeAssetId::get().into(),
                    paid_fee.saturating_sub(corrected_fee),
                    corrected_fee.saturating_sub(tip),
                    tip,
                ),
                PaymentInfo::NonNative(paid_fee, currency, price) => {
                    // calculate corrected_fee in the non-native currency
                    let converted_corrected_fee = price
                        .checked_mul_int(corrected_fee)
                        .ok_or(TransactionValidityError::Invalid(InvalidTransaction::Payment))?;
                    let refund = paid_fee.saturating_sub(converted_corrected_fee);
                    let converted_tip = price
                        .checked_mul_int(tip)
                        .ok_or(TransactionValidityError::Invalid(InvalidTransaction::Payment))?;
                    (
                        currency.into(),
                        refund,
                        converted_corrected_fee.saturating_sub(converted_tip),
                        converted_tip,
                    )
                }
            };

            // refund to the account that paid the fees
            MC::deposit(currency, who, refund)
                .map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;

            // deposit the fee
            DF::deposit_fee(&fee_receiver, currency, fee + tip)
                .map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;
        }

        Ok(())
    }
}

impl<T: Config> CurrencyWithdraw<<T as frame_system::Config>::AccountId, BalanceOf<T>> for Pallet<T>
where
    BalanceOf<T>: FixedPointOperand,
{
    fn withdraw(who: &T::AccountId, fee: BalanceOf<T>) -> Result<PaymentWithdrawResult, DispatchError> {
        Self::withdraw_fee_non_native(who, fee)
    }
}

/// Implements the transaction payment for native as well as non-native currencies
pub struct WithdrawFees<C, OU, SW>(PhantomData<(C, OU, SW)>);

impl<T, C, OU, SW> OnChargeTransaction<T> for WithdrawFees<C, OU, SW>
where
    T: Config,
    C: Currency<<T as frame_system::Config>::AccountId>,
    C::PositiveImbalance:
        Imbalance<<C as Currency<<T as frame_system::Config>::AccountId>>::Balance, Opposite = C::NegativeImbalance>,
    C::NegativeImbalance:
        Imbalance<<C as Currency<<T as frame_system::Config>::AccountId>>::Balance, Opposite = C::PositiveImbalance>,
    OU: OnUnbalanced<NegativeImbalanceOf<C, T>>,
    C::Balance: Into<BalanceOf<T>>,
    SW: CurrencyWithdraw<T::AccountId, BalanceOf<T>>,
    BalanceOf<T>: FixedPointOperand,
{
    type LiquidityInfo = Option<NegativeImbalanceOf<C, T>>;
    type Balance = <C as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// Withdraw the predicted fee from the transaction origin.
    ///
    /// Note: The `fee` already includes the `tip`.
    fn withdraw_fee(
        who: &T::AccountId,
        _call: &T::Call,
        _info: &DispatchInfoOf<T::Call>,
        fee: Self::Balance,
        tip: Self::Balance,
    ) -> Result<Self::LiquidityInfo, TransactionValidityError> {
        if fee.is_zero() {
            return Ok(None);
        }

        let withdraw_reason = if tip.is_zero() {
            WithdrawReasons::TRANSACTION_PAYMENT
        } else {
            WithdrawReasons::TRANSACTION_PAYMENT | WithdrawReasons::TIP
        };

        if let Ok(detail) = SW::withdraw(who, fee.into()) {
            match detail {
                PaymentWithdrawResult::Transferred => Ok(None),
                PaymentWithdrawResult::Native => {
                    match C::withdraw(who, fee, withdraw_reason, ExistenceRequirement::KeepAlive) {
                        Ok(imbalance) => Ok(Some(imbalance)),
                        Err(_) => Err(InvalidTransaction::Payment.into()),
                    }
                }
            }
        } else {
            Err(InvalidTransaction::Payment.into())
        }
    }

    /// Hand the fee and the tip over to the `[OnUnbalanced]` implementation.
    /// Since the predicted fee might have been too high, parts of the fee may
    /// be refunded.
    ///
    /// Note: The `fee` already includes the `tip`.
    /// Note: This is the default implementation
    fn correct_and_deposit_fee(
        who: &T::AccountId,
        _dispatch_info: &DispatchInfoOf<T::Call>,
        _post_info: &PostDispatchInfoOf<T::Call>,
        corrected_fee: Self::Balance,
        tip: Self::Balance,
        already_withdrawn: Self::LiquidityInfo,
    ) -> Result<(), TransactionValidityError> {
        if let Some(paid) = already_withdrawn {
            // Calculate how much refund we should return
            let refund_amount = paid.peek().saturating_sub(corrected_fee);
            // refund to the the account that paid the fees. If this fails, the
            // account might have dropped below the existential balance. In
            // that case we don't refund anything.
            let refund_imbalance =
                C::deposit_into_existing(who, refund_amount).unwrap_or_else(|_| C::PositiveImbalance::zero());
            // merge the imbalance caused by paying the fees and refunding parts of it again.
            let adjusted_paid = paid
                .offset(refund_imbalance)
                .same()
                .map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;
            // Call someone else to handle the imbalance (fee and tip separately)
            let imbalances = adjusted_paid.split(tip);
            OU::on_unbalanceds(Some(imbalances.0).into_iter().chain(Some(imbalances.1)));
        }
        Ok(())
    }
}

/// Signed extension that checks for the `set_currency` call and in that case, it checks the account balance
#[derive(Encode, Decode, Clone, Eq, PartialEq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct CurrencyBalanceCheck<T: Config + Send + Sync>(PhantomData<T>);

impl<T: Config + Send + Sync> sp_std::fmt::Debug for CurrencyBalanceCheck<T> {
    fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        write!(f, "CurrencyBalanceCheck")
    }
}

/// convert an Error to a custom InvalidTransaction with the inner code being the error
/// number.
pub fn error_to_invalid<T: Config>(error: Error<T>) -> InvalidTransaction {
    let error_number = match error.into() {
        DispatchError::Module(ModuleError { error, .. }) => error[0],
        _ => 0, // this case should never happen because an Error is always converted to DispatchError::Module(ModuleError)
    };
    InvalidTransaction::Custom(error_number)
}

impl<T: Config + Send + Sync> SignedExtension for CurrencyBalanceCheck<T>
where
    <T as frame_system::Config>::Call: IsSubType<Call<T>>,
    BalanceOf<T>: FixedPointOperand,
{
    const IDENTIFIER: &'static str = "CurrencyBalanceCheck";
    type AccountId = T::AccountId;
    type Call = <T as frame_system::Config>::Call;
    type AdditionalSigned = ();
    type Pre = ();

    fn additional_signed(&self) -> sp_std::result::Result<(), TransactionValidityError> {
        Ok(())
    }

    fn validate(
        &self,
        who: &Self::AccountId,
        call: &Self::Call,
        _info: &DispatchInfoOf<Self::Call>,
        _len: usize,
    ) -> TransactionValidity {
        match call.is_sub_type() {
            Some(Call::set_currency { currency }) => match Pallet::<T>::check_balance(who, *currency) {
                Ok(_) => Ok(ValidTransaction::default()),
                Err(error) => error_to_invalid(error).into(),
            },
            _ => Ok(Default::default()),
        }
    }

    fn pre_dispatch(
        self,
        who: &Self::AccountId,
        call: &Self::Call,
        _info: &DispatchInfoOf<Self::Call>,
        _len: usize,
    ) -> Result<Self::Pre, TransactionValidityError> {
        match call.is_sub_type() {
            Some(Call::set_currency { currency }) => match Pallet::<T>::check_balance(who, *currency) {
                Ok(_) => Ok(()),
                Err(error) => Err(TransactionValidityError::Invalid(error_to_invalid(error))),
            },
            _ => Ok(()),
        }
    }
}

impl<T: Config + Send + Sync> CurrencyBalanceCheck<T> {
    #[cfg_attr(feature = "cargo-clippy", allow(clippy::new_without_default))]
    pub fn new() -> Self {
        Self(sp_std::marker::PhantomData)
    }
}

/// We provide an oracle for the price of all currencies accepted as fee payment.
impl<T: Config> NativePriceOracle<AssetIdOf<T>, Price> for Pallet<T> {
    fn price(currency: AssetIdOf<T>) -> Option<Price> {
        if currency == T::NativeAssetId::get() {
            Some(Price::one())
        } else {
            Pallet::<T>::currency_price(currency)
        }
    }
}

/// Type to automatically add a fee currency for an account on account creation.
pub struct AddTxAssetOnAccount<T>(PhantomData<T>);
impl<T: Config> Happened<(T::AccountId, AssetIdOf<T>)> for AddTxAssetOnAccount<T> {
    fn happened((who, currency): &(T::AccountId, AssetIdOf<T>)) {
        if !AccountCurrencyMap::<T>::contains_key(who)
            && AcceptedCurrencies::<T>::contains_key(currency)
            && T::Currencies::total_balance(T::NativeAssetId::get(), who).is_zero()
        {
            AccountCurrencyMap::<T>::insert(who, currency);
        }
    }
}

/// Type to automatically remove the fee currency for an account on account deletion.
///
/// Note: The fee currency is only removed if the system account is gone or the account
/// corresponding to the fee currency is empty.
pub struct RemoveTxAssetOnKilled<T>(PhantomData<T>);
impl<T: Config> Happened<(T::AccountId, AssetIdOf<T>)> for RemoveTxAssetOnKilled<T> {
    fn happened((who, _currency): &(T::AccountId, AssetIdOf<T>)) {
        if !frame_system::Pallet::<T>::account_exists(who) {
            AccountCurrencyMap::<T>::remove(who);
        } else if let Some(currency) = AccountCurrencyMap::<T>::get(who) {
            if T::Currencies::total_balance(currency, who).is_zero() {
                AccountCurrencyMap::<T>::remove(who);
            }
        }
    }
}
