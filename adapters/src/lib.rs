// This file is part of hydradx-adapters.

// Copyright (C) 2022  Intergalactic, Limited (GIB).
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

use frame_support::weights::{Weight, WeightToFeePolynomial};
use hydradx_traits::PriceOracle;
use polkadot_xcm::latest::prelude::*;
use sp_runtime::{
    traits::{Convert, Saturating, Zero},
    FixedPointNumber, FixedPointOperand, FixedU128, SaturatedConversion,
};
use sp_std::{collections::btree_map::BTreeMap, marker::PhantomData};
use xcm_builder::TakeRevenue;
use xcm_executor::{traits::WeightTrader, Assets};

pub type Price = FixedU128;

/// Weight trader which uses `WeightToFee` in combination with a `PriceOracle` to set the right
/// price for weight. Keeps track of the assets used to pay for weight and can refund them one
/// by one (interface only allows returning one asset per refund).
pub struct MultiCurrencyTrader<
    AssetId,
    Balance: FixedPointOperand + TryInto<u128>,
    WeightToFee: WeightToFeePolynomial<Balance = Balance>,
    AcceptedCurrencyPrices: PriceOracle<AssetId, Price>,
    ConvertCurrency: Convert<MultiAsset, Option<AssetId>>,
    Revenue: TakeRevenue,
> {
    weight: Weight,
    paid_assets: BTreeMap<(MultiLocation, Price), u128>,
    _phantom: PhantomData<(
        AssetId,
        Balance,
        WeightToFee,
        AcceptedCurrencyPrices,
        ConvertCurrency,
        Revenue,
    )>,
}

impl<
        AssetId,
        Balance: FixedPointOperand + TryInto<u128>,
        WeightToFee: WeightToFeePolynomial<Balance = Balance>,
        AcceptedCurrencyPrices: PriceOracle<AssetId, Price>,
        ConvertCurrency: Convert<MultiAsset, Option<AssetId>>,
        Revenue: TakeRevenue,
    > MultiCurrencyTrader<AssetId, Balance, WeightToFee, AcceptedCurrencyPrices, ConvertCurrency, Revenue>
{
    /// Get the asset id of the first asset in `payment` and try to determine its price via the
    /// price oracle.
    fn get_asset_and_price(&mut self, payment: &Assets) -> Option<(MultiLocation, Price)> {
        if let Some(asset) = payment.fungible_assets_iter().next() {
            ConvertCurrency::convert(asset.clone())
                .and_then(|currency| AcceptedCurrencyPrices::price(currency))
                .and_then(|price| match asset.id.clone() {
                    Concrete(location) => Some((location, price)),
                    _ => None,
                })
        } else {
            None
        }
    }
}

impl<
        AssetId,
        Balance: FixedPointOperand + TryInto<u128>,
        WeightToFee: WeightToFeePolynomial<Balance = Balance>,
        AcceptedCurrencyPrices: PriceOracle<AssetId, Price>,
        ConvertCurrency: Convert<MultiAsset, Option<AssetId>>,
        Revenue: TakeRevenue,
    > WeightTrader
    for MultiCurrencyTrader<AssetId, Balance, WeightToFee, AcceptedCurrencyPrices, ConvertCurrency, Revenue>
{
    fn new() -> Self {
        Self {
            weight: 0,
            paid_assets: Default::default(),
            _phantom: PhantomData,
        }
    }

    /// Will try to buy weight with the first asset in `payment`.
    /// The fee is determined by `WeightToFee` in combination with the price determined by
    /// `AcceptedCurrencyPrices`.
    fn buy_weight(&mut self, weight: Weight, payment: Assets) -> Result<Assets, XcmError> {
        log::trace!(
            target: "xcm::weight", "MultiCurrencyTrader::buy_weight weight: {:?}, payment: {:?}",
            weight, payment
        );
        let (asset_loc, price) = self.get_asset_and_price(&payment).ok_or(XcmError::AssetNotFound)?;
        let fee = WeightToFee::calc(&weight);
        let converted_fee = price.checked_mul_int(fee).ok_or(XcmError::Overflow)?;
        let amount: u128 = converted_fee.try_into().map_err(|_| XcmError::Overflow)?;
        let required = (Concrete(asset_loc.clone()), amount).into();
        let unused = payment.checked_sub(required).map_err(|_| XcmError::TooExpensive)?;
        self.weight = self.weight.saturating_add(weight);
        let key = (asset_loc, price);
        match self.paid_assets.get_mut(&key) {
            Some(v) => v.saturating_accrue(amount),
            None => {
                self.paid_assets.insert(key, amount);
            }
        }
        Ok(unused)
    }

    /// Will refund up to `weight` from the first asset tracked by the trader.
    fn refund_weight(&mut self, weight: Weight) -> Option<MultiAsset> {
        log::trace!(
            target: "xcm::weight", "MultiCurrencyTrader::refund_weight weight: {:?}, paid_assets: {:?}",
            weight, self.paid_assets
        );
        let weight = weight.min(self.weight);
        self.weight -= weight; // Will not underflow because of `min()` above.
        let fee = WeightToFee::calc(&weight);
        if let Some(((asset_loc, price), amount)) = self.paid_assets.iter_mut().next() {
            let converted_fee: u128 = price.saturating_mul_int(fee).saturated_into();
            let refund = converted_fee.min(*amount);
            *amount -= refund; // Will not underflow because of `min()` above.

            let refund_asset = asset_loc.clone();
            if amount.is_zero() {
                let key = (asset_loc.clone(), *price);
                self.paid_assets.remove(&key);
            }
            Some((Concrete(refund_asset), refund).into())
        } else {
            None
        }
    }
}

impl<
        AssetId,
        Balance: FixedPointOperand + TryInto<u128>,
        WeightToFee: WeightToFeePolynomial<Balance = Balance>,
        AcceptedCurrencyPrices: PriceOracle<AssetId, Price>,
        ConvertCurrency: Convert<MultiAsset, Option<AssetId>>,
        Revenue: TakeRevenue,
    > Drop for MultiCurrencyTrader<AssetId, Balance, WeightToFee, AcceptedCurrencyPrices, ConvertCurrency, Revenue>
{
    fn drop(&mut self) {
        for ((asset_loc, _), amount) in self.paid_assets.iter() {
            Revenue::take_revenue((asset_loc.clone(), *amount).into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codec::{Decode, Encode};
    use frame_support::weights::IdentityFee;
    use smallvec::smallvec;
    use sp_runtime::{traits::One, Perbill};
    use sp_std::collections::btree_set::BTreeSet;
    use std::sync::Mutex;

    type AssetId = u32;
    type Balance = u128;

    const CORE_ASSET_ID: AssetId = 0;
    const TEST_ASSET_ID: AssetId = 123;
    const CHEAP_ASSET_ID: AssetId = 420;
    const OVERFLOW_ASSET_ID: AssetId = 1_000;

    struct MockOracle;
    impl PriceOracle<AssetId, Price> for MockOracle {
        fn price(currency: AssetId) -> Option<Price> {
            match currency {
                CORE_ASSET_ID => Some(Price::one()),
                TEST_ASSET_ID => Some(Price::from_float(0.5)),
                CHEAP_ASSET_ID => Some(Price::saturating_from_integer(4)),
                OVERFLOW_ASSET_ID => Some(Price::saturating_from_integer(2_147_483_647)),
                _ => None,
            }
        }
    }

    struct MockConvert;
    impl Convert<AssetId, Option<MultiLocation>> for MockConvert {
        fn convert(id: AssetId) -> Option<MultiLocation> {
            match id {
                CORE_ASSET_ID | TEST_ASSET_ID | CHEAP_ASSET_ID | OVERFLOW_ASSET_ID => {
                    Some(MultiLocation::new(0, X1(GeneralKey(id.encode()))))
                }
                _ => None,
            }
        }
    }

    impl Convert<MultiLocation, Option<AssetId>> for MockConvert {
        fn convert(location: MultiLocation) -> Option<AssetId> {
            match location {
                MultiLocation {
                    parents: 0,
                    interior: X1(GeneralKey(key)),
                } => {
                    if let Ok(currency_id) = AssetId::decode(&mut &key[..]) {
                        // we currently have only one native asset
                        match currency_id {
                            CORE_ASSET_ID | TEST_ASSET_ID | CHEAP_ASSET_ID | OVERFLOW_ASSET_ID => Some(currency_id),
                            _ => None,
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
    }

    impl Convert<MultiAsset, Option<AssetId>> for MockConvert {
        fn convert(asset: MultiAsset) -> Option<AssetId> {
            if let MultiAsset {
                id: Concrete(location), ..
            } = asset
            {
                Self::convert(location)
            } else {
                None
            }
        }
    }

    macro_rules! generate_revenue_type {
        ($name:ident) => {
            lazy_static::lazy_static! {
                pub static ref TAKEN: Mutex<BTreeSet<MultiAsset>> = Mutex::new(BTreeSet::new());
                pub static ref EXPECTED: Mutex<BTreeSet<MultiAsset>> = Mutex::new(BTreeSet::new());
            };

            struct $name;
            impl $name {
                #[allow(unused)]
                fn register_expected_asset(asset: MultiAsset) {
                    EXPECTED.lock().unwrap().insert(asset);
                }

                #[allow(unused)]
                fn expect_revenue() {
                    for asset in EXPECTED.lock().unwrap().iter() {
                        assert!(TAKEN.lock().unwrap().contains(dbg!(asset)));
                    }
                }

                #[allow(unused)]
                fn expect_no_revenue() {
                    assert!(
                        TAKEN.lock().unwrap().is_empty(),
                        "There should be no revenue taken."
                    );
                }

                /// Reset the global mutable state.
                #[allow(unused)]
                fn reset() {
                    *TAKEN.lock().unwrap() = BTreeSet::new();
                    *EXPECTED.lock().unwrap() = BTreeSet::new();
                }
            }

            impl TakeRevenue for $name {
                fn take_revenue(asset: MultiAsset) {
                    TAKEN.lock().unwrap().insert(asset);
                }
            }
        };
    }

    #[test]
    fn can_buy_weight() {
        generate_revenue_type!(ExpectRevenue);

        type Trader =
            MultiCurrencyTrader<AssetId, Balance, IdentityFee<Balance>, MockOracle, MockConvert, ExpectRevenue>;

        let core_id = MockConvert::convert(CORE_ASSET_ID).unwrap();
        let test_id = MockConvert::convert(TEST_ASSET_ID).unwrap();
        let cheap_id = MockConvert::convert(CHEAP_ASSET_ID).unwrap();

        {
            let mut trader = Trader::new();

            let core_payment: MultiAsset = (Concrete(core_id.clone()), 1_000_000).into();
            let res = dbg!(trader.buy_weight(1_000_000, core_payment.clone().into()));
            assert!(res
                .expect("buy_weight should succeed because payment == weight")
                .is_empty());
            ExpectRevenue::register_expected_asset(core_payment);

            let test_payment: MultiAsset = (Concrete(test_id), 500_000).into();
            let res = dbg!(trader.buy_weight(1_000_000, test_payment.clone().into()));
            assert!(res
                .expect("buy_weight should succeed because payment == 0.5 * weight")
                .is_empty());
            ExpectRevenue::register_expected_asset(test_payment);

            let cheap_payment: MultiAsset = (Concrete(cheap_id), 4_000_000).into();
            let res = dbg!(trader.buy_weight(1_000_000, cheap_payment.clone().into()));
            assert!(res
                .expect("buy_weight should succeed because payment == 4 * weight")
                .is_empty());
            ExpectRevenue::register_expected_asset(cheap_payment);
        }
        ExpectRevenue::expect_revenue();
    }

    #[test]
    fn cannot_buy_with_too_few_tokens() {
        type Trader = MultiCurrencyTrader<AssetId, Balance, IdentityFee<Balance>, MockOracle, MockConvert, ()>;

        let core_id = MockConvert::convert(CORE_ASSET_ID).unwrap();

        let mut trader = Trader::new();

        let payment: MultiAsset = (Concrete(core_id), 69).into();
        let res = dbg!(trader.buy_weight(1_000_000, payment.into()));
        assert_eq!(res, Err(XcmError::TooExpensive));
    }

    #[test]
    fn cannot_buy_with_unknown_token() {
        type Trader = MultiCurrencyTrader<AssetId, Balance, IdentityFee<Balance>, MockOracle, MockConvert, ()>;

        let unknown_token = GeneralKey(9876u32.encode());

        let mut trader = Trader::new();
        let payment: MultiAsset = (Concrete(unknown_token.into()), 1_000_000).into();
        let res = dbg!(trader.buy_weight(1_000_000, payment.into()));
        assert_eq!(res, Err(XcmError::AssetNotFound));
    }

    #[test]
    fn overflow_errors() {
        use frame_support::weights::{WeightToFeeCoefficient, WeightToFeeCoefficients};
        // Create a mock fee calculator that always returns `max_value`.
        pub struct MaxFee;
        impl WeightToFeePolynomial for MaxFee {
            type Balance = Balance;

            fn polynomial() -> WeightToFeeCoefficients<Balance> {
                smallvec!(WeightToFeeCoefficient {
                    coeff_integer: Balance::max_value(),
                    coeff_frac: Perbill::zero(),
                    negative: false,
                    degree: 1,
                })
            }
        }
        type Trader = MultiCurrencyTrader<AssetId, Balance, MaxFee, MockOracle, MockConvert, ()>;

        let overflow_id = MockConvert::convert(OVERFLOW_ASSET_ID).unwrap();

        let mut trader = Trader::new();

        let amount = 1_000;
        let payment: MultiAsset = (Concrete(overflow_id), amount).into();
        let weight = 1_000;
        let res = dbg!(trader.buy_weight(weight, payment.into()));
        assert_eq!(res, Err(XcmError::Overflow));
    }

    #[test]
    fn refunds_first_asset_completely() {
        generate_revenue_type!(ExpectRevenue);

        type Trader =
            MultiCurrencyTrader<AssetId, Balance, IdentityFee<Balance>, MockOracle, MockConvert, ExpectRevenue>;

        let core_id = MockConvert::convert(CORE_ASSET_ID).unwrap();

        {
            let mut trader = Trader::new();

            let weight = 1_000_000;
            let tokens = 1_000_000;
            let core_payment: MultiAsset = (Concrete(core_id), tokens).into();
            let res = dbg!(trader.buy_weight(weight, core_payment.clone().into()));
            assert!(res
                .expect("buy_weight should succeed because payment == weight")
                .is_empty());
            assert_eq!(trader.refund_weight(weight), Some(core_payment.into()));
        }
        ExpectRevenue::expect_no_revenue();
    }

    #[test]
    fn needs_multiple_refunds_for_multiple_currencies() {
        generate_revenue_type!(ExpectRevenue);

        type Trader =
            MultiCurrencyTrader<AssetId, Balance, IdentityFee<Balance>, MockOracle, MockConvert, ExpectRevenue>;

        let core_id = MockConvert::convert(CORE_ASSET_ID).unwrap();
        let test_id = MockConvert::convert(TEST_ASSET_ID).unwrap();

        {
            let mut trader = Trader::new();

            let weight = 1_000_000;
            let core_payment: MultiAsset = (Concrete(core_id), 1_000_000).into();
            let res = dbg!(trader.buy_weight(weight, core_payment.clone().into()));
            assert!(res
                .expect("buy_weight should succeed because payment == weight")
                .is_empty());

            let test_payment: MultiAsset = (Concrete(test_id), 500_000).into();
            let res = dbg!(trader.buy_weight(weight, test_payment.clone().into()));
            assert!(res
                .expect("buy_weight should succeed because payment == 0.5 * weight")
                .is_empty());

            assert_eq!(trader.refund_weight(weight), Some(core_payment.into()));
            assert_eq!(trader.refund_weight(weight), Some(test_payment.into()));
        }
        ExpectRevenue::expect_no_revenue();
    }
}
