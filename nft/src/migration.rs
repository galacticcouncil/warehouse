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
// limitations under the License..

use crate::{Collections, Config, ItemInfoOf, Items, Pallet};
use frame_support::{
    log,
    traits::{Get, PalletInfoAccess, StorageVersion},
    weights::Weight,
};

/// Storage names are changed from Classes to Collections and from Instances to Items.
pub mod v1 {
    use super::*;
    use frame_support::{
        migration::move_prefix,
        storage::{storage_prefix, unhashed, StoragePrefixedMap},
        storage_alias, Twox64Concat,
    };
    use sp_io::hashing::twox_128;

    #[storage_alias]
    type Classes<T: Config> =
        StorageMap<Pallet<T>, Twox64Concat, <T as Config>::NftCollectionId, crate::CollectionInfoOf<T>>;

    #[storage_alias]
    type Instances<T: Config> = StorageDoubleMap<
        Pallet<T>,
        Twox64Concat,
        <T as Config>::NftCollectionId,
        Twox64Concat,
        <T as Config>::NftItemId,
        ItemInfoOf<T>,
    >;

    pub mod move_old_storage{
        use super::*;

        pub fn pre_migrate<T: Config>() {
            assert_eq!(StorageVersion::get::<Pallet<T>>(), 0, "Storage version too high.");

            log::info!(
                target: "runtime::nft",
                "move_old_storage migration: PRE checks successful!"
            );
        }

        pub fn migrate<T: Config>() -> Weight {
            log::info!(
                target: "runtime::nft",
                "Running move_old_storage migration to v1"
            );

            let pallet_name = <Pallet<T> as PalletInfoAccess>::name().as_bytes();

            // move Classes to Collections
            let new_storage_prefix = storage_prefix(pallet_name, Collections::<T>::storage_prefix());
            let old_storage_prefix = storage_prefix(pallet_name, Classes::<T>::storage_prefix());

            move_prefix(&old_storage_prefix, &new_storage_prefix);
            if let Some(value) = unhashed::get_raw(&old_storage_prefix) {
                unhashed::put_raw(&new_storage_prefix, &value);
                unhashed::kill(&old_storage_prefix);
            }

            // move Instances to Items
            let new_storage_prefix = storage_prefix(pallet_name, Items::<T>::storage_prefix());
            let old_storage_prefix = storage_prefix(pallet_name, Instances::<T>::storage_prefix());

            move_prefix(&old_storage_prefix, &new_storage_prefix);
            if let Some(value) = unhashed::get_raw(&old_storage_prefix) {
                unhashed::put_raw(&new_storage_prefix, &value);
                unhashed::kill(&old_storage_prefix);
            }

            StorageVersion::new(1).put::<Pallet<T>>();

            <T as frame_system::Config>::BlockWeights::get().max_block
        }

        pub fn post_migrate<T: Config>() {
            assert_eq!(StorageVersion::get::<Pallet<T>>(), 1, "Unexpected storage version.");

            let pallet_name = <Pallet<T> as PalletInfoAccess>::name().as_bytes();

            // Assert that no `Classes` storage remains at the old prefix.
            let old_storage_prefix = Classes::<T>::storage_prefix();
            let old_key = [&twox_128(pallet_name), &twox_128(old_storage_prefix)[..]].concat();
            let old_key_iter =
                frame_support::storage::KeyPrefixIterator::new(old_key.to_vec(), old_key.to_vec(), |_| Ok(()));
            assert_eq!(old_key_iter.count(), 0);

            // Assert that no `Instances` storage remains at the old prefix.
            let old_storage_prefix = Instances::<T>::storage_prefix();
            let old_key = [&twox_128(pallet_name), &twox_128(old_storage_prefix)[..]].concat();
            let old_key_iter =
                frame_support::storage::KeyPrefixIterator::new(old_key.to_vec(), old_key.to_vec(), |_| Ok(()));
            assert_eq!(old_key_iter.count(), 0);

            log::info!(
                target: "runtime::nft",
                "move_old_storage migration: POST checks successful!"
            );
        }
    }

    // kill the old storages and update the storage version to v1
    pub mod kill_old_storage{
        use super::*;

        pub fn pre_migrate<T: Config>() {
            assert_eq!(StorageVersion::get::<Pallet<T>>(), 0, "Storage version too high.");

            log::info!(
                target: "runtime::nft",
                "kill_old_storage migration: PRE checks successful!"
            );
        }

        pub fn migrate<T: Config>() -> Weight {
            log::info!(
                target: "runtime::nft",
                "Running kill_old_storage migration to v1"
            );

            let pallet_name = <Pallet<T> as PalletInfoAccess>::name().as_bytes();

            // kill Classes storage
            let old_storage_prefix = storage_prefix(pallet_name, Classes::<T>::storage_prefix());
            unhashed::kill(&old_storage_prefix);

            // kill Instances storage
            let old_storage_prefix = storage_prefix(pallet_name, Instances::<T>::storage_prefix());
            unhashed::kill(&old_storage_prefix);

            StorageVersion::new(1).put::<Pallet<T>>();

            <T as frame_system::Config>::BlockWeights::get().max_block
        }

        pub fn post_migrate<T: Config>() {
            assert_eq!(StorageVersion::get::<Pallet<T>>(), 1, "Unexpected storage version.");

            let pallet_name = <Pallet<T> as PalletInfoAccess>::name().as_bytes();

            // Assert that no `Classes` storage remains at the old prefix.
            let old_storage_prefix = Classes::<T>::storage_prefix();
            let old_key = [&twox_128(pallet_name), &twox_128(old_storage_prefix)[..]].concat();
            let old_key_iter =
                frame_support::storage::KeyPrefixIterator::new(old_key.to_vec(), old_key.to_vec(), |_| Ok(()));
            assert_eq!(old_key_iter.count(), 0);

            // Assert that no `Instances` storage remains at the old prefix.
            let old_storage_prefix = Instances::<T>::storage_prefix();
            let old_key = [&twox_128(pallet_name), &twox_128(old_storage_prefix)[..]].concat();
            let old_key_iter =
                frame_support::storage::KeyPrefixIterator::new(old_key.to_vec(), old_key.to_vec(), |_| Ok(()));
            assert_eq!(old_key_iter.count(), 0);

            log::info!(
                target: "runtime::nft",
                "kill_old_storage migration: POST checks successful!"
            );
        }
    }
}
