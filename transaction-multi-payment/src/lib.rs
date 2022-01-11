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
    transactional,
    weights::DispatchClass,
    weights::WeightToFeePolynomial,
};
use frame_system::ensure_signed;
use sp_runtime::{
    traits::{DispatchInfoOf, PostDispatchInfoOf, Saturating, Zero},
    transaction_validity::{InvalidTransaction, TransactionValidityError},
    FixedU128,
};
use sp_std::prelude::*;

use pallet_transaction_payment::OnChargeTransaction;
use sp_std::marker::PhantomData;

use frame_support::sp_runtime::FixedPointNumber;
use frame_support::sp_runtime::FixedPointOperand;
use frame_support::weights::{Pays, Weight};
use hydradx_traits::pools::SpotPriceProvider;
use orml_traits::{MultiCurrency, MultiCurrencyExtended};

use codec::{Decode, Encode};
use frame_support::sp_runtime::traits::SignedExtension;
use frame_support::sp_runtime::transaction_validity::{TransactionValidity, ValidTransaction};
use frame_support::traits::IsSubType;

use scale_info::TypeInfo;

use crate::traits::{CurrencyWithdraw, PaymentWithdrawResult};
use frame_support::dispatch::DispatchError;

type AssetIdOf<T> = <<T as Config>::Currencies as MultiCurrency<<T as frame_system::Config>::AccountId>>::CurrencyId;
type BalanceOf<T> = <<T as Config>::Currencies as MultiCurrency<<T as frame_system::Config>::AccountId>>::Balance;

/// Spot price type
pub type Price = FixedU128;

type NegativeImbalanceOf<C, T> = <C as Currency<<T as frame_system::Config>::AccountId>>::NegativeImbalance;

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::OriginFor;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

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
        type WeightToFee: WeightToFeePolynomial<Balance = BalanceOf<Self>>;

        /// Native Asset
        #[pallet::constant]
        type NativeAssetId: Get<AssetIdOf<Self>>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// CurrencySet
        /// [who, currency]
        CurrencySet(T::AccountId, AssetIdOf<T>),

        /// New accepted currency added
        /// [currency]
        CurrencyAdded(AssetIdOf<T>),

        /// Accepted currency removed
        /// [currency]
        CurrencyRemoved(AssetIdOf<T>),

        /// Transaction fee paid in non-native currency
        /// [Account, Currency, Native fee amount, Non-native fee amount, Destination account]
        FeeWithdrawn(T::AccountId, AssetIdOf<T>, BalanceOf<T>, BalanceOf<T>, T::AccountId),
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

    /// Curated list of currencies which fees can be paid with
    #[pallet::storage]
    #[pallet::getter(fn currencies)]
    pub type AcceptedCurrencies<T: Config> = StorageMap<_, Twox64Concat, AssetIdOf<T>, Price, OptionQuery>;

    /// Account to use when pool does not exist.
    #[pallet::storage]
    #[pallet::getter(fn fallback_account)]
    pub type FallbackAccount<T: Config> = StorageValue<_, T::AccountId, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub currencies: Vec<(AssetIdOf<T>, Price)>,
        pub fallback_account: T::AccountId,
        pub account_currencies: Vec<(T::AccountId, AssetIdOf<T>)>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            GenesisConfig {
                currencies: vec![],
                fallback_account: Default::default(),
                account_currencies: vec![],
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            if self.fallback_account == Default::default() {
                panic!("Fallback account is not set");
            }

            FallbackAccount::<T>::put(self.fallback_account.clone());

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
        #[transactional]
        pub fn set_currency(origin: OriginFor<T>, currency: AssetIdOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            if currency == T::NativeAssetId::get() || AcceptedCurrencies::<T>::contains_key(&currency) {
                if T::Currencies::free_balance(currency, &who) == BalanceOf::<T>::zero() {
                    return Err(Error::<T>::ZeroBalance.into());
                }

                <AccountCurrencyMap<T>>::insert(who.clone(), currency);

                if T::WithdrawFeeForSetCurrency::get() == Pays::Yes {
                    Self::withdraw_set_fee(&who)?;
                }

                Self::deposit_event(Event::CurrencySet(who, currency));

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
                Self::deposit_event(Event::CurrencyAdded(currency));
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

                Self::deposit_event(Event::CurrencyRemoved(currency));

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

    /// Execute a trade to buy HDX and sell selected currency.
    pub fn withdraw_fee_non_native(
        who: &T::AccountId,
        fee: BalanceOf<T>,
    ) -> Result<PaymentWithdrawResult, DispatchError> {
        let currency = Self::account_currency(who);

        if currency == T::NativeAssetId::get() {
            Ok(PaymentWithdrawResult::Native)
        } else {
            let price = if let Some(spot_price) = T::SpotPriceProvider::spot_price(currency, T::NativeAssetId::get()) {
                spot_price
            } else {
                Self::currencies(currency).ok_or(Error::<T>::FallbackPriceNotFound)?
            };

            let amount = price.checked_mul_int(fee).ok_or(Error::<T>::Overflow)?;

            T::Currencies::transfer(currency, who, &Self::fallback_account(), amount)?;

            Self::deposit_event(Event::FeeWithdrawn(
                who.clone(),
                currency,
                fee,
                amount,
                Self::fallback_account(),
            ));

            Ok(PaymentWithdrawResult::Transferred)
        }
    }

    pub fn withdraw_set_fee(who: &T::AccountId) -> DispatchResult {
        let base_fee = Self::weight_to_fee(T::BlockWeights::get().get(DispatchClass::Normal).base_extrinsic);
        let adjusted_weight_fee = Self::weight_to_fee(T::WeightInfo::set_currency());
        let fee = base_fee.saturating_add(adjusted_weight_fee);

        let result = Self::withdraw(who, fee)?;
        match result {
            PaymentWithdrawResult::Transferred => Ok(()),
            PaymentWithdrawResult::Native => T::Currencies::withdraw(T::NativeAssetId::get(), who, fee),
        }
    }

    fn weight_to_fee(weight: Weight) -> BalanceOf<T> {
        // cap the weight to the maximum defined in runtime, otherwise it will be the
        // `Bounded` maximum of its data type, which is not desired.
        let capped_weight: Weight = weight.min(T::BlockWeights::get().max_block);
        <T as Config>::WeightToFee::calc(&capped_weight)
    }

    fn check_balance(account: &T::AccountId, currency: AssetIdOf<T>) -> Result<(), Error<T>> {
        if T::Currencies::free_balance(currency, account) == BalanceOf::<T>::zero() {
            return Err(Error::<T>::ZeroBalance);
        };
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
pub struct MultiCurrencyAdapter<C, OU, SW>(PhantomData<(C, OU, SW)>);

impl<T, C, OU, SW> OnChargeTransaction<T> for MultiCurrencyAdapter<C, OU, SW>
where
    T: Config,
    T::TransactionByteFee: Get<<C as Currency<<T as frame_system::Config>::AccountId>>::Balance>,
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
                Err(error) => InvalidTransaction::Custom(error.as_u8()).into(),
            },
            _ => Ok(Default::default()),
        }
    }
}

impl<T: Config + Send + Sync> CurrencyBalanceCheck<T> {
    #[cfg_attr(feature = "cargo-clippy", allow(clippy::new_without_default))]
    pub fn new() -> Self {
        Self(sp_std::marker::PhantomData)
    }
}
