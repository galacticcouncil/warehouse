// This file is part of warehouse

// Copyright (C) 2020-2023  Intergalactic, Limited (GIB).
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Test environment for Assets pallet.

use sp_std::prelude::*;
use std::cell::RefCell;

use crate::{Config, Volume, VolumeProvider};

use frame_support::{
    construct_runtime, parameter_types,
    traits::{ConstU32, ConstU64},
};
pub use orml_traits::MultiCurrency;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    FixedU128,
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
use crate::tests::oracle::Oracle;
use sp_runtime::Permill;

pub type Balance = u128;
pub type AssetId = u32;
pub type AccountId = u64;

pub const HDX: AssetId = 0;
pub const LRNA: AssetId = 1;

thread_local! {
    pub static PAIRS: RefCell<Vec<(AssetId, AssetId)>> = RefCell::new(vec![]);
    pub static ORACLE: RefCell<Oracle> = RefCell::new(Oracle::new());
    pub static BLOCK: RefCell<usize> = RefCell::new(0);
}

construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        DynamicFees: crate::{Pallet, Call, Storage, Event<T>},
    }
);

impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = ConstU64<250>;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

parameter_types! {
    pub const SelectedPeriod: u16 = 300;
    //pub Decay: FixedU128= FixedU128::from_float(0.0005);
    pub Decay: FixedU128= FixedU128::from_float(0.0);
    pub Amplification: FixedU128= FixedU128::from_float(1.0);
    pub MinimumFee: Permill = Permill::from_rational(25u32, 10000u32);
    pub MaximumFee: Permill = Permill::from_percent(40);
}

impl Config for Test {
    type Event = Event;
    type AssetId = AssetId;
    type OraclePeriod = u16;
    type BlockNumberProvider = System;
    type Oracle = OracleProvider;
    type SelectedPeriod = SelectedPeriod;
    type Decay = Decay;
    type Amplification = Amplification;
    type MinimumFee = MinimumFee;
    type MaximumFee = MaximumFee;
}

#[derive(Default)]
pub struct ExtBuilder {}

impl ExtBuilder {
    pub fn build(self) -> sp_io::TestExternalities {
        frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap()
            .into()
    }
}

pub struct OracleProvider {}

impl VolumeProvider<AssetId, Balance, u16> for OracleProvider {
    type Volume = AssetVolume;

    fn asset_pair_volume(pair: (AssetId, AssetId), _period: u16) -> Option<Self::Volume> {
        let volume = ORACLE.with(|v| v.borrow().volume(pair, BLOCK.with(|v| v.borrow().clone())).clone());
        Some(volume)
    }

    fn asset_pair_liquidity(pair: (AssetId, AssetId), _period: u16) -> Option<Balance> {
        let liquidity = ORACLE.with(|v| v.borrow().liquidity(pair, BLOCK.with(|v| v.borrow().clone())).clone());
        Some(liquidity)
    }
}

#[derive(Default, Clone, Debug)]
pub struct AssetVolume {
    pub(crate) amount_in: Balance,
    pub(crate) amount_out: Balance,
}

impl Volume<Balance> for AssetVolume {
    fn amount_a_in(&self) -> Balance {
        self.amount_in
    }

    fn amount_b_in(&self) -> Balance {
        todo!()
    }

    fn amount_a_out(&self) -> Balance {
        self.amount_out
    }

    fn amount_b_out(&self) -> Balance {
        todo!()
    }
}

impl From<(Balance, Balance, Balance)> for AssetVolume {
    fn from(value: (Balance, Balance, Balance)) -> Self {
        Self {
            amount_in: value.0,
            amount_out: value.1,
        }
    }
}
