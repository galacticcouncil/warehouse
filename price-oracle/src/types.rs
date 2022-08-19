// This file is part of pallet-price-oracle.

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

use codec::{Decode, Encode};
use frame_support::sp_runtime::traits::CheckedMul;
use frame_support::sp_runtime::{FixedU128, RuntimeDebug};
use scale_info::TypeInfo;
use sp_arithmetic::{
    traits::{One, SaturatedConversion, UniqueSaturatedInto},
    FixedPointNumber,
};

use sp_arithmetic::traits::CheckedSub;

use sp_std::prelude::*;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

pub type AssetId = u32;
pub type Balance = u128;
pub type Price = FixedU128;
pub type Period = u32;

/// A type representing data produced by a trade.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(RuntimeDebug, Encode, Decode, Copy, Clone, PartialEq, Eq, Default, TypeInfo)]
pub struct OracleEntry<BlockNumber> {
    pub price: Price,
    pub volume: Balance,
    pub liquidity: Balance,
    pub timestamp: BlockNumber,
}

impl<BlockNumber> OracleEntry<BlockNumber>
where
    BlockNumber: CheckedSub + Copy + PartialOrd + UniqueSaturatedInto<u32>,
{
    /// Determine a new entry based on `self` and a previous entry. Adds the volumes together and
    /// takes the values of `self` for the rest.
    pub fn accumulate_volume(&self, previous_entry: &Self) -> Self {
        let volume = previous_entry.volume.saturating_add(self.volume);
        Self {
            price: self.price,
            volume,
            liquidity: self.liquidity,
            timestamp: self.timestamp,
        }
    }

    /// Determine a new price entry based on self and a previous entry.
    ///
    /// Uses an exponential moving average with a smoothing factor of `alpha = 2 / (N + 1)`.
    /// `alpha = 2 / (N + 1)` leads to the center of mass of the EMA corresponding to an N-length SMA.
    ///
    /// Uses the difference between the `timestamp`s to determine the time to cover and exponentiates
    /// the complement (`1 - alpha`) with that time difference.
    ///
    /// Possible alternatives for `alpha = 2 / (N + 1)`:
    /// + `alpha = 1 - 0.5^(1 / N)` for a half-life of N periods or
    /// + `alpha = 1 - 0.5^(1 / (0.5N))` to have the same median as an N-length SMA.
    /// See https://en.wikipedia.org/wiki/Moving_average#Relationship_between_SMA_and_EMA
    pub fn calculate_new_ema_entry(&self, period: Period, previous_entry: &Self) -> Option<Self> {
        use sp_arithmetic::traits::Saturating;
        if period <= 1 {
            return Some(self.clone());
        }
        let alpha = Price::saturating_from_rational(2u32, period.saturating_add(1));
        debug_assert!(alpha <= Price::one());
        let complement = Price::one() - alpha;

        debug_assert!(self.timestamp > previous_entry.timestamp);
        let iterations = self.timestamp.checked_sub(&previous_entry.timestamp)?;
        let exp_complement = complement.saturating_pow(iterations.saturated_into::<u32>() as usize);
        debug_assert!(exp_complement <= Price::one());
        let exp_alpha = Price::one() - exp_complement;

        let price = price_ema(previous_entry.price, exp_complement, self.price, exp_alpha)?;
        let volume = balance_ema(previous_entry.volume, exp_complement, self.volume, exp_alpha)?;
        let liquidity = balance_ema(previous_entry.liquidity, exp_complement, self.liquidity, exp_alpha)?;

        Some(Self {
            price,
            volume,
            liquidity,
            timestamp: self.timestamp,
        })
    }
}

/// Calculate the next exponential moving average for the given prices.
/// `prev` is the previous oracle value, `incoming` is the new value to integrate.
pub fn price_ema(prev: Price, prev_weight: FixedU128, incoming: Price, weight: FixedU128) -> Option<Price> {
    debug_assert!(prev_weight + weight == Price::one());
    // Safe to use bare `+` because `prev_weight + weight == 1`.
    // `prev_value * prev_weight + incoming_value * weight`
    let price = prev.checked_mul(&prev_weight)? + incoming.checked_mul(&weight)?;
    Some(price)
}

/// Calculate the next exponential moving average for the given values.
/// `prev` is the previous oracle value, `incoming` is the new value to integrate.
/// `weight` is the weight of the new value, `prev_weight` is the weight of the previous value.
pub fn balance_ema(prev: Balance, prev_weight: FixedU128, incoming: Balance, weight: FixedU128) -> Option<Balance> {
    debug_assert!(prev_weight + weight == Price::one());
    // Safe to use bare `+` because `prev_weight + apha == 1`.
    // `prev_value * prev_weight + incoming_value * weight`
    let new_value = if prev < u64::MAX.into() && incoming < u64::MAX.into() {
        // We use `checked_mul` in combination with `Price::from` to avoid rounding errors induced
        // by using `checked_mul_int` with small values.
        (prev_weight.checked_mul(&Price::from(prev))? + weight.checked_mul(&Price::from(incoming))?)
            .saturating_mul_int(Balance::one())
    } else {
        // We use `checked_mul_int` to avoid saturating the fixed point type for big balance values.
        // Note: Incurs rounding errors for small balance values, but the relative error is small
        // because the other value is greater than `u64::MAX`.
        prev_weight.checked_mul_int(prev)? + weight.checked_mul_int(incoming)?
    };
    Some(new_value)
}
