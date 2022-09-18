// This file is part of galacticcouncil/warehouse.

// Copyright (C) 2020-2022  Intergalactic, Limited (GIB).
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

use codec::HasCompact;
use frame_support::{
    dispatch::DispatchResult,
    ensure,
    traits::{tokens::nonfungibles::*, Get},
    BoundedVec,
};
use frame_system::ensure_signed;
use pallet_uniques::DestroyWitness;

use hydradx_traits::nft::{CreateTypedCollection, ReserveCollectionId};
use sp_runtime::{
    traits::{AtLeast32BitUnsigned, StaticLookup, Zero},
    DispatchError,
};
use sp_std::boxed::Box;
pub use types::*;
use weights::WeightInfo;

mod benchmarking;
pub mod types;
pub mod weights;
pub mod migration;

#[cfg(test)]
pub mod mock;

#[cfg(test)]
mod tests;

pub type BoundedVecOfUnq<T> = BoundedVec<u8, <T as pallet_uniques::Config>::StringLimit>;
type CollectionInfoOf<T> = CollectionInfo<<T as Config>::CollectionType, BoundedVecOfUnq<T>>;
pub type ItemInfoOf<T> = ItemInfo<BoundedVec<u8, <T as pallet_uniques::Config>::StringLimit>>;

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {

    use super::*;
    use frame_support::{pallet_prelude::*, traits::EnsureOrigin};
    use frame_system::pallet_prelude::OriginFor;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_uniques::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type WeightInfo: WeightInfo;
        type ProtocolOrigin: EnsureOrigin<Self::Origin>;
        type NftCollectionId: Member
            + Parameter
            + Default
            + Copy
            + HasCompact
            + AtLeast32BitUnsigned
            + Into<Self::CollectionId>
            + From<Self::CollectionId>
            + MaxEncodedLen;
        type NftItemId: Member
            + Parameter
            + Default
            + Copy
            + HasCompact
            + AtLeast32BitUnsigned
            + Into<Self::ItemId>
            + From<Self::ItemId>
            + MaxEncodedLen;
        type CollectionType: Member + Parameter + Default + Copy + MaxEncodedLen;
        type Permissions: NftPermission<Self::CollectionType>;
        /// Collection IDs reserved for runtime up to the following constant
        #[pallet::constant]
        type ReserveCollectionIdUpTo: Get<Self::NftCollectionId>;
    }

    #[pallet::storage]
    #[pallet::getter(fn collections)]
    /// Stores collection info
    pub type Collections<T: Config> = StorageMap<_, Twox64Concat, T::NftCollectionId, CollectionInfoOf<T>>;

    #[pallet::storage]
    #[pallet::getter(fn items)]
    /// Stores item info
    pub type Items<T: Config> =
        StorageDoubleMap<_, Twox64Concat, T::NftCollectionId, Twox64Concat, T::NftItemId, ItemInfoOf<T>>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Creates an NFT collection of the given collection type
        /// and sets its metadata
        ///
        /// Parameters:
        /// - `collection_id`: Identifier of a collection
        /// - `collection_type`: The collection type determines its purpose and usage
        /// - `metadata`: Arbitrary data about a collection, e.g. IPFS hash or name
        ///
        /// Emits CollectionCreated event
        #[pallet::weight(<T as Config>::WeightInfo::create_collection())]
        pub fn create_collection(
            origin: OriginFor<T>,
            collection_id: T::NftCollectionId,
            collection_type: T::CollectionType,
            metadata: BoundedVecOfUnq<T>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            ensure!(!Self::is_id_reserved(collection_id), Error::<T>::IdReserved);
            ensure!(T::Permissions::can_create(&collection_type), Error::<T>::NotPermitted);

            Self::do_create_collection(sender, collection_id, collection_type, metadata)?;

            Ok(())
        }

        /// Mints an NFT in the specified collection
        /// and sets its metadata
        ///
        /// Parameters:
        /// - `collection_id`: The collection of the asset to be minted.
        /// - `item_id`: The item of the asset to be minted.
        /// - `metadata`: Arbitrary data about an item, e.g. IPFS hash or symbol
        #[pallet::weight(<T as Config>::WeightInfo::mint())]
        pub fn mint(
            origin: OriginFor<T>,
            collection_id: T::NftCollectionId,
            item_id: T::NftItemId,
            metadata: BoundedVecOfUnq<T>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let collection_type = Self::collections(collection_id)
                .map(|c| c.collection_type)
                .ok_or(Error::<T>::CollectionUnknown)?;

            ensure!(T::Permissions::can_mint(&collection_type), Error::<T>::NotPermitted);

            Self::do_mint(sender, collection_id, item_id, metadata)?;

            Ok(())
        }

        /// Transfers NFT from account A to account B
        /// Only the ProtocolOrigin can send NFT to another account
        /// This is to prevent creating deposit burden for others
        ///
        /// Parameters:
        /// - `collection_id`: The collection of the asset to be transferred.
        /// - `item_id`: The instance of the asset to be transferred.
        /// - `dest`: The account to receive ownership of the asset.
        #[pallet::weight(<T as Config>::WeightInfo::transfer())]
        pub fn transfer(
            origin: OriginFor<T>,
            collection_id: T::NftCollectionId,
            item_id: T::NftItemId,
            dest: <T::Lookup as StaticLookup>::Source,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let dest = T::Lookup::lookup(dest)?;

            let collection_type = Self::collections(collection_id)
                .map(|c| c.collection_type)
                .ok_or(Error::<T>::CollectionUnknown)?;

            ensure!(T::Permissions::can_transfer(&collection_type), Error::<T>::NotPermitted);

            Self::do_transfer(collection_id, item_id, sender, dest)?;

            Ok(())
        }

        /// Removes a token from existence
        ///
        /// Parameters:
        /// - `collection_id`: The collection of the asset to be burned.
        /// - `item_id`: The instance of the asset to be burned.
        #[pallet::weight(<T as Config>::WeightInfo::burn())]
        pub fn burn(origin: OriginFor<T>, collection_id: T::NftCollectionId, item_id: T::NftItemId) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let collection_type = Self::collections(collection_id)
                .map(|c| c.collection_type)
                .ok_or(Error::<T>::CollectionUnknown)?;

            ensure!(T::Permissions::can_burn(&collection_type), Error::<T>::NotPermitted);

            Self::do_burn(sender, collection_id, item_id)?;

            Ok(())
        }

        /// Removes a collection from existence
        ///
        /// Parameters:
        /// - `collection_id`: The identifier of the asset collection to be destroyed.
        #[pallet::weight(<T as Config>::WeightInfo::destroy_collection())]
        pub fn destroy_collection(origin: OriginFor<T>, collection_id: T::NftCollectionId) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let collection_type = Self::collections(collection_id)
                .map(|c| c.collection_type)
                .ok_or(Error::<T>::CollectionUnknown)?;

            ensure!(T::Permissions::can_destroy(&collection_type), Error::<T>::NotPermitted);

            Self::do_destroy_collection(sender, collection_id)?;

            Ok(())
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A collection was created
        CollectionCreated {
            owner: T::AccountId,
            collection_id: T::NftCollectionId,
            collection_type: T::CollectionType,
            metadata: BoundedVecOfUnq<T>,
        },
        /// An item was minted
        ItemMinted {
            owner: T::AccountId,
            collection_id: T::NftCollectionId,
            item_id: T::NftItemId,
            metadata: BoundedVecOfUnq<T>,
        },
        /// An item was transferred
        ItemTransferred {
            from: T::AccountId,
            to: T::AccountId,
            collection_id: T::NftCollectionId,
            item_id: T::NftItemId,
        },
        /// An item was burned
        ItemBurned {
            owner: T::AccountId,
            collection_id: T::NftCollectionId,
            item_id: T::NftItemId,
        },
        /// A collection was destroyed
        CollectionDestroyed {
            owner: T::AccountId,
            collection_id: T::NftCollectionId,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Count of items overflown
        NoAvailableItemId,
        /// Count of collections overflown
        NoAvailableCollectionId,
        /// Collection still contains minted tokens
        TokenCollectionNotEmpty,
        /// Collection does not exist
        CollectionUnknown,
        /// Item does not exist
        ItemUnknown,
        /// Operation not permitted
        NotPermitted,
        /// ID reserved for runtime
        IdReserved,
    }
}

impl<T: Config> Pallet<T> {
    pub fn collection_owner(collection_id: T::NftCollectionId) -> Option<T::AccountId> {
        pallet_uniques::Pallet::<T>::collection_owner(collection_id.into())
    }

    pub fn owner(collection_id: T::NftCollectionId, item_id: T::NftItemId) -> Option<T::AccountId> {
        pallet_uniques::Pallet::<T>::owner(collection_id.into(), item_id.into())
    }

    fn do_create_collection(
        owner: T::AccountId,
        collection_id: T::NftCollectionId,
        collection_type: T::CollectionType,
        metadata: BoundedVecOfUnq<T>,
    ) -> DispatchResult {
        let deposit_info = match T::Permissions::has_deposit(&collection_type) {
            false => (Zero::zero(), true),
            true => (T::CollectionDeposit::get(), false),
        };

        pallet_uniques::Pallet::<T>::do_create_collection(
            collection_id.into(),
            owner.clone(),
            owner.clone(),
            deposit_info.0,
            deposit_info.1,
            pallet_uniques::Event::Created {
                collection: collection_id.into(),
                creator: owner.clone(),
                owner: owner.clone(),
            },
        )?;

        Collections::<T>::insert(
            collection_id,
            CollectionInfo {
                collection_type,
                metadata: metadata.clone(),
            },
        );

        Self::deposit_event(Event::CollectionCreated {
            owner,
            collection_id,
            collection_type,
            metadata,
        });

        Ok(())
    }

    fn do_mint(
        owner: T::AccountId,
        collection_id: T::NftCollectionId,
        item_id: T::NftItemId,
        metadata: BoundedVecOfUnq<T>,
    ) -> DispatchResult {
        ensure!(
            Collections::<T>::contains_key(collection_id),
            Error::<T>::CollectionUnknown
        );

        pallet_uniques::Pallet::<T>::do_mint(collection_id.into(), item_id.into(), owner.clone(), |_details| Ok(()))?;

        Items::<T>::insert(
            collection_id,
            item_id,
            ItemInfo {
                metadata: metadata.clone(),
            },
        );

        Self::deposit_event(Event::ItemMinted {
            owner,
            collection_id,
            item_id,
            metadata,
        });

        Ok(())
    }

    fn do_transfer(
        collection_id: T::NftCollectionId,
        item_id: T::NftItemId,
        from: T::AccountId,
        to: T::AccountId,
    ) -> DispatchResult {
        if from == to {
            return Ok(());
        }

        pallet_uniques::Pallet::<T>::do_transfer(
            collection_id.into(),
            item_id.into(),
            to.clone(),
            |_collection_details, _item_details| {
                let owner = Self::owner(collection_id, item_id).ok_or(Error::<T>::ItemUnknown)?;
                ensure!(owner == from, Error::<T>::NotPermitted);
                Self::deposit_event(Event::ItemTransferred {
                    from,
                    to,
                    collection_id,
                    item_id,
                });
                Ok(())
            },
        )
    }

    fn do_burn(owner: T::AccountId, collection_id: T::NftCollectionId, item_id: T::NftItemId) -> DispatchResult {
        pallet_uniques::Pallet::<T>::do_burn(
            collection_id.into(),
            item_id.into(),
            |_collection_details, _item_details| {
                let iowner = Self::owner(collection_id, item_id).ok_or(Error::<T>::ItemUnknown)?;
                ensure!(owner == iowner, Error::<T>::NotPermitted);
                Ok(())
            },
        )?;

        Items::<T>::remove(collection_id, item_id);

        Self::deposit_event(Event::ItemBurned {
            owner,
            collection_id,
            item_id,
        });

        Ok(())
    }

    fn do_destroy_collection(owner: T::AccountId, collection_id: T::NftCollectionId) -> DispatchResult {
        let witness = pallet_uniques::Pallet::<T>::get_destroy_witness(&collection_id.into())
            .ok_or(Error::<T>::CollectionUnknown)?;

        // witness struct is empty because we don't allow destroying a collection with existing items
        ensure!(witness.items == 0u32, Error::<T>::TokenCollectionNotEmpty);

        pallet_uniques::Pallet::<T>::do_destroy_collection(collection_id.into(), witness, Some(owner.clone()))?;
        Collections::<T>::remove(collection_id);

        Self::deposit_event(Event::CollectionDestroyed { owner, collection_id });
        Ok(())
    }
}

impl<T: Config> Inspect<T::AccountId> for Pallet<T> {
    type ItemId = T::NftItemId;
    type CollectionId = T::NftCollectionId;

    fn owner(collection: &Self::CollectionId, item: &Self::ItemId) -> Option<T::AccountId> {
        Self::owner(*collection, *item)
    }

    fn collection_owner(collection: &Self::CollectionId) -> Option<T::AccountId> {
        Self::collection_owner(*collection)
    }

    fn can_transfer(collection: &Self::CollectionId, _item: &Self::ItemId) -> bool {
        let maybe_collection_type = Self::collections(collection).map(|c| c.collection_type);

        match maybe_collection_type {
            Some(collection_type) => T::Permissions::can_transfer(&collection_type),
            _ => false,
        }
    }
}

impl<T: Config> InspectEnumerable<T::AccountId> for Pallet<T> {
    fn collections() -> Box<dyn Iterator<Item = Self::CollectionId>> {
        Box::new(Collections::<T>::iter_keys())
    }

    fn items(collection: &Self::CollectionId) -> Box<dyn Iterator<Item = Self::ItemId>> {
        Box::new(Items::<T>::iter_key_prefix(collection))
    }

    fn owned(who: &T::AccountId) -> Box<dyn Iterator<Item = (Self::CollectionId, Self::ItemId)>> {
        Box::new(
            pallet_uniques::Pallet::<T>::owned(who)
                .map(|(collection_id, item_id)| (collection_id.into(), item_id.into())),
        )
    }

    fn owned_in_collection(
        collection: &Self::CollectionId,
        who: &T::AccountId,
    ) -> Box<dyn Iterator<Item = Self::ItemId>> {
        Box::new(
            pallet_uniques::Pallet::<T>::owned_in_collection(
                &(Into::<<T as pallet_uniques::Config>::CollectionId>::into(*collection)),
                who,
            )
            .map(|i| i.into()),
        )
    }
}

impl<T: Config> Create<T::AccountId> for Pallet<T> {
    fn create_collection(collection: &Self::CollectionId, who: &T::AccountId, _admin: &T::AccountId) -> DispatchResult {
        Self::do_create_collection(who.clone(), *collection, Default::default(), BoundedVec::default())?;

        Ok(())
    }
}

impl<T: Config> Destroy<T::AccountId> for Pallet<T> {
    type DestroyWitness = pallet_uniques::DestroyWitness;

    fn get_destroy_witness(collection: &Self::CollectionId) -> Option<Self::DestroyWitness> {
        pallet_uniques::Pallet::<T>::get_destroy_witness(
            &(Into::<<T as pallet_uniques::Config>::CollectionId>::into(*collection)),
        )
    }

    fn destroy(
        collection: Self::CollectionId,
        _witness: Self::DestroyWitness,
        _maybe_check_owner: Option<T::AccountId>,
    ) -> Result<Self::DestroyWitness, DispatchError> {
        let owner = Self::collection_owner(collection).ok_or(Error::<T>::CollectionUnknown)?;

        Self::do_destroy_collection(owner, collection)?;

        // We can return empty struct here because we don't allow destroying a collection with existing items
        Ok(DestroyWitness {
            items: 0,
            item_metadatas: 0,
            attributes: 0,
        })
    }
}

impl<T: Config> Mutate<T::AccountId> for Pallet<T> {
    fn mint_into(collection: &Self::CollectionId, item: &Self::ItemId, who: &T::AccountId) -> DispatchResult {
        Self::do_mint(who.clone(), *collection, *item, BoundedVec::default())?;

        Ok(())
    }

    fn burn(
        collection: &Self::CollectionId,
        item: &Self::ItemId,
        _maybe_check_owner: Option<&T::AccountId>,
    ) -> DispatchResult {
        let owner = Self::owner(*collection, *item).ok_or(Error::<T>::ItemUnknown)?;

        Self::do_burn(owner, *collection, *item)?;

        Ok(())
    }
}

impl<T: Config> Transfer<T::AccountId> for Pallet<T> {
    fn transfer(collection: &Self::CollectionId, item: &Self::ItemId, destination: &T::AccountId) -> DispatchResult {
        let owner = Self::owner(*collection, *item).ok_or(Error::<T>::ItemUnknown)?;

        Self::do_transfer(*collection, *item, owner, destination.clone())
    }
}

impl<T: Config> CreateTypedCollection<T::AccountId, T::NftCollectionId, T::CollectionType> for Pallet<T> {
    fn create_typed_collection(
        owner: T::AccountId,
        collection_id: T::NftCollectionId,
        collection_type: T::CollectionType,
    ) -> DispatchResult {
        Self::do_create_collection(owner, collection_id, collection_type, Default::default())
    }
}

impl<T: Config> ReserveCollectionId<T::NftCollectionId> for Pallet<T> {
    fn is_id_reserved(id: T::NftCollectionId) -> bool {
        id <= T::ReserveCollectionIdUpTo::get()
    }
}
