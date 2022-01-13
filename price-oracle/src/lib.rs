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

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet_prelude::Weight;
use frame_support::sp_runtime::traits::{CheckedDiv, Zero};
use frame_support::sp_runtime::FixedPointNumber;
use hydradx_traits::{OnCreatePoolHandler, OnTradeHandler};
use sp_std::convert::TryInto;
use sp_std::marker::PhantomData;
use sp_std::prelude::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod types;
pub use types::*;

#[allow(clippy::all)]
pub mod weights;
use weights::WeightInfo;

mod benchmarking; // TODO: rebenchmark

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;
use frame_support::sp_runtime::DispatchError;

/// Unique identifier for an asset pair.
/// AMM pools derive their own unique identifiers for asset pairs,
/// but this one is meant to not be bounded to one particular AMM pool.
pub type AssetPairId = Vec<u8>;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Weight information for the extrinsics.
        type WeightInfo: WeightInfo;
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Calculation error occurred while calculating average price
        PriceComputationError,

        /// An unexpected overflow occurred
        UpdateDataOverflow,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Pool was registered. [asset a, asset b]
        PoolRegistered(AssetId, AssetId),
    }

    /// The number of assets registered and handled by this pallet.
    #[pallet::storage]
    #[pallet::getter(fn num_of_assets)]
    pub type TrackedAssetsCount<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Sorted array of newly registered assets.
    /// All assets are processed and removed from the storage at the end of a block.
    /// Trades start to be processed from the next block.
    /// All trades in the same block as the asset registration are ignored.
    #[pallet::storage]
    #[pallet::getter(fn new_assets)]
    pub type NewAssets<T: Config> = StorageValue<_, Vec<AssetPairId>, ValueQuery>;

    /// Processed or partially processed data generated by trades.
    /// Data generated by trades are processed sequentially.
    /// Each new entry is combined with the previous value to produce new intermediate value.
    /// The last entry creates the resulting average price and volume.
    #[pallet::storage]
    #[pallet::getter(fn price_accumulator)]
    pub type PriceDataAccumulator<T: Config> = StorageMap<_, Twox64Concat, AssetPairId, PriceEntry, ValueQuery>;

    /// The last ten average values corresponding to the last ten blocks.
    #[pallet::storage]
    #[pallet::getter(fn price_data_ten)]
    pub type PriceDataTen<T: Config> = StorageValue<_, Vec<(AssetPairId, BucketQueue)>, ValueQuery>;

    /// The last ten average values corresponding to the last hundred blocks.
    /// Each average value corresponds to an interval of length ten blocks.
    #[pallet::storage]
    #[pallet::getter(fn price_data_hundred)]
    pub type PriceDataHundred<T: Config> = StorageMap<_, Twox64Concat, AssetPairId, BucketQueue, ValueQuery>;

    /// The last ten average values corresponding to the last thousand blocks.
    /// Each average value corresponds to an interval of length hundred blocks.
    #[pallet::storage]
    #[pallet::getter(fn price_data_thousand)]
    pub type PriceDataThousand<T: Config> = StorageMap<_, Twox64Concat, AssetPairId, BucketQueue, ValueQuery>;

    #[pallet::genesis_config]
    #[derive(Default)]
	pub struct GenesisConfig {
        pub price_data: Vec<((AssetId, AssetId), Price, Balance)>,
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
            for &(asset_pair, avg_price, volume) in self.price_data.iter() {
                let pair_id = Pallet::<T>::get_name(asset_pair.0, asset_pair.1);

                let data_ten = PriceDataTen::<T>::get();
                assert!(!data_ten.iter().any(|bucket_tuple| bucket_tuple.0 == pair_id), "Assets already registered!");

                let mut bucket = BucketQueue::default();
                bucket.update_last(PriceInfo{ avg_price, volume });
                PriceDataTen::<T>::append((pair_id, bucket));
            }

            TrackedAssetsCount::<T>::set(self.price_data.len().try_into().unwrap());
		}
	}

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
        fn on_initialize(_n: T::BlockNumber) -> Weight {
            T::WeightInfo::on_finalize_multiple_tokens_all_bucket_levels(Self::num_of_assets())
        }

        fn on_finalize(_n: T::BlockNumber) {
            // update average values in the storage
            Self::update_data();

            // clear the price buffer
            PriceDataAccumulator::<T>::remove_all(None);

            // add newly registered assets
            let _ = TrackedAssetsCount::<T>::try_mutate(|value| -> Result<(), DispatchError> {
                *value = value
                    .checked_add(
                        Self::new_assets()
                            .len()
                            .try_into()
                            .map_err(|_| Error::<T>::PriceComputationError)?,
                    )
                    .ok_or(Error::<T>::PriceComputationError)?;
                Ok(())
                // We don't want to throw an error here because this method is used in different extrinsics.
                // We also do not expect to have more than 2^32 assets registered.
            })
            .map_err(|_| panic!("Max number of assets reached!"));

            for new_asset in Self::new_assets().iter() {
                PriceDataTen::<T>::append((new_asset, BucketQueue::default()));
            }
            NewAssets::<T>::kill();
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {}
}

impl<T: Config> Pallet<T> {
    pub fn on_create_pool(asset_a: AssetId, asset_b: AssetId) {
        let data = PriceDataTen::<T>::get();
        if !data.iter().any(|bucket_tuple| bucket_tuple.0 == Self::get_name(asset_a, asset_b)) {
            let _ = NewAssets::<T>::try_mutate(|new_assets| -> Result<(), ()> {
                // Keep the NewAssets vector sorted. It makes it easy to find duplicates.
                match new_assets.binary_search(&Self::get_name(asset_a, asset_b)) {
                    Ok(_pos) => Err(()), // new asset is already in vector
                    Err(pos) => {
                        new_assets.insert(pos, Self::get_name(asset_a, asset_b));
                        Self::deposit_event(Event::PoolRegistered(asset_a, asset_b));
                        Ok(())
                    }
                }
            })
            .map_err(|_| {});
        }
    }

    pub fn on_trade(asset_a: AssetId, asset_b: AssetId, price_entry: PriceEntry) {
        let _ = PriceDataAccumulator::<T>::mutate(Self::get_name(asset_a, asset_b), |previous_price_entry| {
            let maybe_new_price_entry = previous_price_entry.calculate_new_price_entry(&price_entry);
            // Invalid values are ignored and not added to the queue.
            if let Some(new_price_entry) = maybe_new_price_entry {
                *previous_price_entry = new_price_entry;
            }
        });
    }

    fn update_data() {
        PriceDataTen::<T>::mutate(|data_ten| {
            for (asset_pair_id, data) in data_ten.iter_mut() {
                let maybe_price = PriceDataAccumulator::<T>::try_get(asset_pair_id);
                let result = if let Ok(price_entry) = maybe_price {
                    PriceInfo {
                        avg_price: price_entry.price,
                        volume: price_entry.trade_amount,
                    }
                } else {
                    data.get_last()
                };

                data.update_last(result);
            }
        });

        let now = <frame_system::Pallet<T>>::block_number();

        // check if it's time to update "hundred" values
        if (now % T::BlockNumber::from(BUCKET_SIZE)) == T::BlockNumber::from(BUCKET_SIZE - 1) {
            for element_from_ten in PriceDataTen::<T>::get().iter() {
                PriceDataHundred::<T>::mutate(element_from_ten.0.clone(), |data| {
                    data.update_last(element_from_ten.1.calculate_average());
                });
            }
        }

        // check if it's time to update "thousand" values
        if (now % T::BlockNumber::from(BUCKET_SIZE.pow(2))) == T::BlockNumber::from(BUCKET_SIZE.pow(2) - 1) {
            for element_from_hundred in PriceDataHundred::<T>::iter() {
                PriceDataThousand::<T>::mutate(element_from_hundred.0.clone(), |data| {
                    data.update_last(element_from_hundred.1.calculate_average());
                });
            }
        }
    }

    /// Calculate price from ordered assets
    pub fn normalize_price(
        asset_a: AssetId, asset_b: AssetId, amount_in: Balance, amount_out: Balance
    ) -> Option<(Price, Balance)> {
        let ordered_asset_pair = Self::ordered_pair(asset_a, asset_b);
        let (balance_a, balance_b) = if ordered_asset_pair.0 == asset_a {
            (amount_in, amount_out)
        } else {
            (amount_out, amount_in)
        };

        let price_a = Price::checked_from_integer(balance_a)?;
        let price_b = Price::checked_from_integer(balance_b)?;
        let price = price_a.checked_div(&price_b);
        price.map(|p| (p, balance_a))
    }

    /// Return ordered asset tuple (A,B) where A < B
	/// Used in storage
    /// The implementation is the same as for AssetPair
	pub fn ordered_pair(asset_a: AssetId, asset_b: AssetId) -> (AssetId, AssetId) {
		match asset_a <= asset_b {
			true => (asset_a, asset_b),
			false => (asset_b, asset_a),
		}
	}

    /// Return share token name
    /// The implementation is the same as for AssetPair
	pub fn get_name(asset_a: AssetId, asset_b: AssetId) -> Vec<u8> {
		let mut buf: Vec<u8> = Vec::new();

		let (asset_left, asset_right) = Self::ordered_pair(asset_a, asset_b);

		buf.extend_from_slice(&asset_left.to_le_bytes());
		buf.extend_from_slice(b"HDT");
		buf.extend_from_slice(&asset_right.to_le_bytes());

		buf
	}
}

pub struct PriceOracleHandler<T>(PhantomData<T>);
impl<T: Config> OnCreatePoolHandler<AssetId> for PriceOracleHandler<T> {
    fn on_create_pool(asset_a: AssetId, asset_b: AssetId) {
        Pallet::<T>::on_create_pool(asset_a, asset_b);
    }
}

impl<T: Config> OnTradeHandler<AssetId, Balance> for PriceOracleHandler<T> {
    fn on_trade(asset_a: AssetId, asset_b: AssetId, amount_in: Balance, amount_out: Balance, liq_amount: Balance) {
        let (price, amount) = if let Some(price_tuple) = Pallet::<T>::normalize_price(asset_a, asset_b, amount_in, amount_out) {
            price_tuple
        } else {
            // We don't want to throw an error here because this method is used in different extrinsics.
            // Invalid prices are ignored and not added to the queue.
            return;
        };

        // We assume that zero values are not valid.
        // Zero values are ignored and not added to the queue.
        if price.is_zero() || amount.is_zero() || liq_amount.is_zero() {
            return;
        }

        let price_entry = PriceEntry {
            price,
            trade_amount: amount,
            liquidity_amount: liq_amount,
        };

        Pallet::<T>::on_trade(asset_a, asset_b, price_entry);
    }

    fn on_trade_weight() -> Weight {
        T::WeightInfo::on_finalize_one_token() - T::WeightInfo::on_finalize_no_entry()
    }
}
