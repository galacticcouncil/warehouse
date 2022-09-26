// This file is part of pallet-ema-oracle.

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

use codec::{Decode, Encode};
use frame_support::sp_runtime::traits::CheckedMul;
use frame_support::sp_runtime::{FixedU128, RuntimeDebug};
use hydradx_traits::{AggregatedEntry, Volume};
use scale_info::TypeInfo;
use sp_arithmetic::{
    traits::{AtLeast32BitUnsigned, One, SaturatedConversion, Saturating, UniqueSaturatedInto, Zero},
    FixedPointNumber,
};

pub use hydradx_traits::Source;

use sp_std::prelude::*;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

pub type AssetId = u32;
pub type Balance = u128;
pub type Price = FixedU128;

/// A type representing data produced by a trade or liquidity event. Timestamped to the block where
/// it was created.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(RuntimeDebug, Encode, Decode, Clone, PartialEq, Eq, Default, TypeInfo)]
pub struct OracleEntry<BlockNumber> {
    pub price: Price,
    pub volume: Volume<Balance>,
    pub liquidity: Balance,
    pub timestamp: BlockNumber,
}

impl<BlockNumber> OracleEntry<BlockNumber>
where
    BlockNumber: AtLeast32BitUnsigned + Copy + UniqueSaturatedInto<u64>,
{
    /// Convert the `OracleEntry` into an `AggregatedEntry` for consumption. Determines the age by
    /// subtracting `initialized` from the timestamp.
    pub fn into_aggregated(self, initialized: BlockNumber) -> AggregatedEntry<Balance, BlockNumber, Price> {
        AggregatedEntry {
            price: self.price,
            volume: self.volume,
            liquidity: self.liquidity,
            oracle_age: self.timestamp.saturating_sub(initialized),
        }
    }

    /// Return an inverted version of the entry where the meaning of assets a and b are inverted.
    /// So the price of a/b become the price b/a and the volume switches correspondingly.
    pub fn inverted(&self) -> Self {
        // It makes sense for the reciprocal of zero to be zero here.
        let price = self.price.reciprocal().unwrap_or_else(Zero::zero);
        let volume = self.volume.inverted();
        Self {
            price,
            volume,
            liquidity: self.liquidity,
            timestamp: self.timestamp,
        }
    }

    /// Update the volume in `self` by adding in the volume of `incoming` and taking over the other
    /// values.
    pub fn accumulate_volume_and_update_from(&mut self, incoming: &Self) {
        self.volume = incoming.volume.saturating_add(&self.volume);
        self.price = incoming.price;
        self.liquidity = incoming.liquidity;
        self.timestamp = incoming.timestamp;
    }

    /// Determine a new entry based on `self` and a previous entry. Adds the volumes together and
    /// takes the values of `self` for the rest.
    pub fn with_added_volume_from(&self, previous_entry: &Self) -> Self {
        let volume = previous_entry.volume.saturating_add(&self.volume);
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
    pub fn calculate_new_ema_entry(&self, period: BlockNumber, previous_entry: &Self) -> Option<Self> {
        if period <= One::one() {
            return Some(self.clone());
        }
        let alpha = alpha_from_period(period);
        debug_assert!(alpha <= Price::one());
        let complement = Price::one() - alpha;

        debug_assert!(self.timestamp > previous_entry.timestamp);
        let iterations = self.timestamp.checked_sub(&previous_entry.timestamp)?;
        let exp_complement = complement.saturating_pow(iterations.saturated_into::<u32>() as usize);
        debug_assert!(exp_complement <= Price::one());
        let exp_alpha = Price::one() - exp_complement;

        let price = price_ema(previous_entry.price, exp_complement, self.price, exp_alpha)?;
        let volume = volume_ema(&previous_entry.volume, exp_complement, &self.volume, exp_alpha)?;
        let liquidity = balance_ema(previous_entry.liquidity, exp_complement, self.liquidity, exp_alpha)?;

        Some(Self {
            price,
            volume,
            liquidity,
            timestamp: self.timestamp,
        })
    }

    /// Determine a new price entry based on the previous entry (self) and an incoming entry.
    ///
    /// Uses an exponential moving average with a smoothing factor of `alpha = 2 / (N + 1)`.
    /// `alpha = 2 / (N + 1)` leads to the center of mass of the EMA corresponding to an N-length SMA.
    ///
    /// Uses the difference between the `timestamp`s to determine the time to cover and exponentiates
    /// the complement (`1 - alpha`) with that time difference.
    pub fn update_via_ema_with(&mut self, period: BlockNumber, incoming: &Self) -> Option<&mut Self> {
        if period <= One::one() {
            *self = incoming.clone();
            return Some(self);
        }
        let alpha = alpha_from_period(period);
        debug_assert!(alpha <= Price::one());
        let complement = Price::one() - alpha;

        debug_assert!(incoming.timestamp > self.timestamp);
        let iterations = incoming.timestamp.checked_sub(&self.timestamp)?;
        let exp_complement = complement.saturating_pow(iterations.saturated_into::<u32>() as usize);
        debug_assert!(exp_complement <= Price::one());
        let exp_alpha = Price::one() - exp_complement;

        self.price = price_ema(self.price, exp_complement, incoming.price, exp_alpha)?;
        self.volume = volume_ema(&self.volume, exp_complement, &incoming.volume, exp_alpha)?;
        self.liquidity = balance_ema(self.liquidity, exp_complement, incoming.liquidity, exp_alpha)?;
        self.timestamp = incoming.timestamp;
        Some(self)
    }
}

/// Calculates smoothing factor alpha for an exponential moving average based on `period`:
/// `alpha = 2 / (period + 1)`. It leads to the "center of mass" of the EMA corresponding to the
/// "center of mass" of a `period`-length SMA.
///
/// Possible alternatives for `alpha = 2 / (period + 1)`:
/// + `alpha = 1 - 0.5^(1 / period)` for a half-life of `period` or
/// + `alpha = 1 - 0.5^(2 / period)` to have the same median as a `period`-length SMA. See
/// https://en.wikipedia.org/wiki/Moving_average#Relationship_between_SMA_and_EMA (N = period).
pub fn alpha_from_period<BlockNumber>(period: BlockNumber) -> Price
where
    BlockNumber: AtLeast32BitUnsigned + Copy + UniqueSaturatedInto<u64>,
{
    Price::saturating_from_rational(2u64, period.saturating_add(One::one()).saturated_into::<u64>())
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

/// Calculate the next exponential moving average for the given volumes.
/// `prev` is the previous oracle value, `incoming` is the new value to integrate.
/// `weight` is the weight of the new value, `prev_weight` is the weight of the previous value.
///
/// Note: Just delegates to `balance_ema` under the hood.
pub fn volume_ema(
    prev: &Volume<Balance>,
    prev_weight: FixedU128,
    incoming: &Volume<Balance>,
    weight: FixedU128,
) -> Option<Volume<Balance>> {
    debug_assert!(prev_weight + weight == Price::one());
    let Volume {
        a_in: prev_a_in,
        b_out: prev_b_out,
        a_out: prev_a_out,
        b_in: prev_b_in,
    } = prev;
    let Volume {
        a_in,
        b_out,
        a_out,
        b_in,
    } = incoming;
    let volume = Volume {
        a_in: balance_ema(*prev_a_in, prev_weight, *a_in, weight)?,
        b_out: balance_ema(*prev_b_out, prev_weight, *b_out, weight)?,
        a_out: balance_ema(*prev_a_out, prev_weight, *a_out, weight)?,
        b_in: balance_ema(*prev_b_in, prev_weight, *b_in, weight)?,
    };
    Some(volume)
}

impl<BlockNumber> From<(Price, Volume<Balance>, Balance, BlockNumber)> for OracleEntry<BlockNumber> {
    fn from((price, volume, liquidity, timestamp): (Price, Volume<Balance>, Balance, BlockNumber)) -> Self {
        Self {
            price,
            volume,
            liquidity,
            timestamp,
        }
    }
}