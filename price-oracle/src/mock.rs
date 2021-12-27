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

use crate as price_oracle;
use crate::Config;
use frame_support::parameter_types;
use frame_support::traits::{Everything, Get};
use hydradx_traits::AssetPairAccountIdFor;
use orml_traits::parameter_type_with_key;
use price_oracle::PriceEntry;
use hydra_dx_math::fee::Fee;
use sp_core::H256;
use frame_support::sp_runtime::{
    FixedU128,
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};
use std::cell::RefCell;

pub type Amount = i128;
pub type AssetId = u32;
pub type Balance = u128;
pub type Price = FixedU128;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

pub const HDX: AssetId = 1_000;
pub const DOT: AssetId = 2_000;
pub const ACA: AssetId = 3_000;
pub const ETH: AssetId = 4_000;

pub const PRICE_ENTRY_1: PriceEntry = PriceEntry {
    price: Price::from_inner(2000000000000000000),
    trade_amount: 1_000,
    liquidity_amount: 2_000,
};
pub const PRICE_ENTRY_2: PriceEntry = PriceEntry {
    price: Price::from_inner(5000000000000000000),
    trade_amount: 3_000,
    liquidity_amount: 4_000,
};

frame_support::construct_runtime!(
    pub enum Test where
     Block = Block,
     NodeBlock = Block,
     UncheckedExtrinsic = UncheckedExtrinsic,
     {
         System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
         PriceOracle: price_oracle::{Pallet, Call, Storage, Event<T>},
         Currency: orml_tokens::{Pallet, Event<T>},
     }

);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
}

impl frame_system::Config for Test {
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
    type SS58Prefix = ();
    type OnSetCode = ();
}

pub struct AssetPairAccountIdTest();

impl AssetPairAccountIdFor<AssetId, u64> for AssetPairAccountIdTest {
    fn from_assets(asset_a: AssetId, asset_b: AssetId, _: &str) -> u64 {
        let mut a = asset_a as u128;
        let mut b = asset_b as u128;
        if a > b {
            std::mem::swap(&mut a, &mut b);
        }
        (a * 1000 + b) as u64
    }
}

thread_local! {
        static EXCHANGE_FEE: RefCell<Fee> = RefCell::new(Fee {numerator: 2, denominator: 1_000});
}

struct ExchangeFee;
impl Get<Fee> for ExchangeFee {
    fn get() -> Fee {
        EXCHANGE_FEE.with(|v| *v.borrow())
    }
}

parameter_type_with_key! {
    pub ExistentialDeposits: |_currency_id: AssetId| -> Balance {
        1_000_000
    };
}

impl orml_tokens::Config for Test {
    type Event = Event;
    type Balance = Balance;
    type Amount = Amount;
    type CurrencyId = AssetId;
    type WeightInfo = ();
    type ExistentialDeposits = ExistentialDeposits;
    type OnDust = ();
    type MaxLocks = ();
    type DustRemovalWhitelist = Everything;
}

impl Config for Test {
    type Event = Event;
    type WeightInfo = ();
}

pub struct ExtBuilder;

impl ExtBuilder {
    pub fn build(self) -> sp_io::TestExternalities {
        let storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
        sp_io::TestExternalities::from(storage)
    }
}
