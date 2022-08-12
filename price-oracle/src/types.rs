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
pub struct PriceEntry<BlockNumber> {
    pub price: Price,
    pub volume: Balance,
    pub liquidity: Balance,
    pub timestamp: BlockNumber,
}

impl<BlockNumber> PriceEntry<BlockNumber>
where
    BlockNumber: UniqueSaturatedInto<u64> + Copy,
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
    /// Uses an exponential moving average with a smoothing factor of `alpha = 2 / (N + 1)`.
    /// `alpha = 2 / (N + 1)` leads to the center of mass of the EMA corresponding to an N-length SMA.
    ///
    /// Possible alternatives: `alpha = 1 - 0.5^(1 / N)` for a half-life of N periods or
    /// `alpha = 1 - 0.5^(1 / (0.5N))` to have the same median as an N-length SMA.
    /// See https://en.wikipedia.org/wiki/Moving_average#Relationship_between_SMA_and_EMA
    pub fn calculate_new_ema_entry(&self, period: Period, previous_entry: &Self) -> Option<Self> {
        if period <= 1 {
            return Some(self.clone());
        }
        let alpha = Price::saturating_from_rational(2u32, period.saturating_add(1));
        debug_assert!(alpha <= Price::one());
        let inv_alpha = Price::one() - alpha;

        let mut price = previous_entry.price;
        let mut volume = previous_entry.volume;
        let mut liquidity = previous_entry.liquidity;
        log::debug!("before ema: {:?}", (price, volume, liquidity));
        log::debug!("self before ema: {:?}", (self.price, self.volume, self.liquidity));
        let range = previous_entry.timestamp.saturated_into::<u64>()..self.timestamp.saturated_into::<u64>();
        log::debug!("range: {:?}", range);
        let rounds = range.clone().count() as u64;
        for round in range {
            if round % (rounds / 20).max(1) == 0 || round == rounds - 1 {
                log::debug!("round {}: {:?}", round, (price, volume, liquidity));
            }
            price = price_ema(price, self.price, alpha, inv_alpha)?;
            volume = balance_ema(volume, self.volume, alpha, inv_alpha)?;
            liquidity = balance_ema(liquidity, self.liquidity, alpha, inv_alpha)?;
        }
        log::debug!("after ema: {:?}", (price, volume, liquidity));
        Some(Self {
            price,
            volume,
            liquidity,
            timestamp: self.timestamp,
        })
    }
}

/// Calculate the next exponential moving average for the given price.
pub(crate) fn price_ema(prev: Price, incoming: Price, alpha: Price, inv_alpha: Price) -> Option<Price> {
    debug_assert!(inv_alpha + alpha == Price::one());
    // Safe to use bare `+` because `inv_alpha + apha == 1`.
    // `prev_value * inv_alpha + incoming_value * alpha`
    let price = prev.checked_mul(&inv_alpha)? + incoming.checked_mul(&alpha)?;
    Some(price)
}

/// Calculate the next exponential moving average for the given values.
pub(crate) fn balance_ema(prev: Balance, incoming: Balance, alpha: Price, inv_alpha: Price) -> Option<Balance> {
    debug_assert!(inv_alpha + alpha == Price::one());
    // Safe to use bare `+` because `inv_alpha + apha == 1`.
    // `prev_value * inv_alpha + incoming_value * alpha`
    // `checked_mul` in combination with `Price::from` necessary to avoid rounding errors induced by
    // using `checked_mul_int` with small values.
    let new_value = (inv_alpha.checked_mul(&Price::from(prev))? + alpha.checked_mul(&Price::from(incoming))?)
        .saturating_mul_int(1u32.into());
    Some(new_value)
}
