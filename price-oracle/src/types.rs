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
use frame_support::sp_runtime::traits::{CheckedAdd, CheckedDiv, CheckedMul, Zero};
use frame_support::sp_runtime::{FixedU128, RuntimeDebug};
use scale_info::TypeInfo;
use sp_arithmetic::{
    traits::{One, SaturatedConversion, UniqueSaturatedInto},
    FixedPointNumber,
};
use sp_std::iter::Sum;
use sp_std::ops::{Add, Index, IndexMut};
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
    /// Updates the previous average value with a new entry.
    pub fn calculate_new_price_entry(&self, previous_price_entry: &Self) -> Option<Self> {
        let total_liquidity = previous_price_entry.liquidity.checked_add(self.liquidity)?;
        let product_of_old_values = previous_price_entry
            .price
            .checked_mul(&Price::from_inner(previous_price_entry.liquidity))?;
        let product_of_new_values = self.price.checked_mul(&Price::from_inner(self.liquidity))?;
        Some(Self {
            price: product_of_old_values
                .checked_add(&product_of_new_values)?
                .checked_div(&Price::from_inner(total_liquidity))?,
            volume: previous_price_entry.volume.checked_add(self.volume)?,
            liquidity: total_liquidity,
            timestamp: self.timestamp,
        })
    }

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
            (price, volume, liquidity) = ema(
                (price, volume, liquidity),
                (self.price, self.volume, self.liquidity),
                alpha,
                inv_alpha,
            )?;
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

pub(crate) fn ema(
    (prev_price, prev_volume, prev_liquidity): (Price, Balance, Balance),
    (new_price, new_volume, new_liquidity): (Price, Balance, Balance),
    alpha: Price,
    inv_alpha: Price,
) -> Option<(Price, Balance, Balance)> {
    // All three should follow `old_value * inv_alpha + incoming_value * alpha`.
    // Safe to use bare `+` because `inv_alpha + apha == 1`.
    let price = prev_price.checked_mul(&inv_alpha)? + new_price.checked_mul(&alpha)?;
    let volume = (inv_alpha.checked_mul(&Price::from(prev_volume))? + alpha.checked_mul(&Price::from(new_volume))?)
        .saturating_mul_int(1u32.into());
    // `checked_mul` in combination with `Price::from` necessary to avoid rounding errors
    // induced by using `checked_mul_int` with small values.
    let liquidity = (inv_alpha.checked_mul(&Price::from(prev_liquidity))?
        + alpha.checked_mul(&Price::from(new_liquidity))?)
    .saturating_mul_int(1u32.into());
    Some((price, volume, liquidity))
}
