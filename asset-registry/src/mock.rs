// This file is part of pallet-asset-registry.

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

#![cfg(test)]

use frame_support::parameter_types;
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

use frame_support::traits::{Everything, GenesisBuild};

use polkadot_xcm::v0::MultiLocation;

use crate::{self as asset_registry, Config};

pub type AssetId = u32;
pub type Balance = u128;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test where
     Block = Block,
     NodeBlock = Block,
     UncheckedExtrinsic = UncheckedExtrinsic,
     {
         System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
         Registry: asset_registry::{Pallet, Call, Storage, Event<T>},
     }

);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 63;
    pub const NativeAssetId: AssetId = 0;
    pub const RegistryStringLimit: u32 = 10;
    pub const SequentialIdStart: u32 = 1_000_000;
}

impl system::Config for Test {
    type BaseCallFilter = Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

use codec::{Decode, Encode};
use scale_info::TypeInfo;

#[derive(Debug, Encode, Decode, Clone, PartialEq, Eq, TypeInfo)]
pub struct AssetLocation(pub MultiLocation);

impl Default for AssetLocation {
    fn default() -> Self {
        AssetLocation(MultiLocation::Null)
    }
}

impl Config for Test {
    type Event = Event;
    type RegistryOrigin = frame_system::EnsureRoot<u64>;
    type AssetId = u32;
    type Balance = Balance;
    type AssetNativeLocation = AssetLocation;
    type StringLimit = RegistryStringLimit;
    type SequentialIdStartAt = SequentialIdStart;
    type NativeAssetId = NativeAssetId;
    type WeightInfo = ();
}
pub type AssetRegistryPallet = crate::Pallet<Test>;

#[derive(Default)]
pub struct ExtBuilder {
    registered_assets: Vec<(Vec<u8>, Balance, Option<AssetId>)>,
    native_asset_name: Option<Vec<u8>>,
}

impl ExtBuilder {
    pub fn with_assets(mut self, asset_ids: Vec<(Vec<u8>, Balance, Option<AssetId>)>) -> Self {
        self.registered_assets = asset_ids;
        self
    }

    pub fn with_native_asset_name(mut self, name: Vec<u8>) -> Self {
        self.native_asset_name = Some(name);
        self
    }

    pub fn build(self) -> sp_io::TestExternalities {
        let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

        if let Some(name) = self.native_asset_name {
            crate::GenesisConfig::<Test> {
                registered_assets: self.registered_assets,
                native_asset_name: name,
                native_existential_deposit: 1_000_000u128,
            }
        } else {
            crate::GenesisConfig::<Test> {
                registered_assets: self.registered_assets,
                ..Default::default()
            }
        }
        .assimilate_storage(&mut t)
        .unwrap();
        t.into()
    }
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

pub fn expect_events(e: Vec<Event>) {
    test_utils::expect_events::<Event, Test>(e);
}
