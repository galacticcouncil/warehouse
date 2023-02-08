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

use crate::{Config, UpdateAndRetrieveFees, Volume, VolumeProvider};

use frame_support::{
    construct_runtime, parameter_types,
    traits::{ConstU32, ConstU64},
};
use orml_traits::GetByKey;
pub use orml_traits::MultiCurrency;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    FixedU128, Permill,
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
use crate::tests::oracle::Oracle;
use sp_runtime::traits::{One, Zero};

pub type Balance = u128;
pub type AssetId = u32;
pub type AccountId = u64;

pub const HDX: AssetId = 0;

pub const ONE: Balance = 1_000_000_000_000;

pub(crate) type Fee = Permill;

thread_local! {
    pub static PAIRS: RefCell<Vec<(AssetId, AssetId)>> = RefCell::new(vec![]);
    pub static ORACLE: RefCell<Box<dyn CustomOracle>> = RefCell::new(Box::new(Oracle::new()));
    pub static BLOCK: RefCell<usize> = RefCell::new(0);
    pub static ASSET_MIN_FEE: RefCell<Fee> = RefCell::new(Fee::from_percent(1));
    pub static ASSET_MAX_FEE: RefCell<Fee> = RefCell::new(Fee::from_percent(40));
    pub static ASSET_FEE_DECAY: RefCell<FixedU128> = RefCell::new(FixedU128::zero());
    pub static ASSET_FEE_AMPLIFICATION: RefCell<FixedU128> = RefCell::new(FixedU128::one());

    pub static PROTOCOL_MIN_FEE: RefCell<Fee> = RefCell::new(Fee::from_percent(1));
    pub static PROTOCOL_MAX_FEE: RefCell<Fee> = RefCell::new(Fee::from_percent(40));
    pub static PROTOCOL_FEE_DECAY: RefCell<FixedU128> = RefCell::new(FixedU128::zero());
    pub static PROTOCOL_FEE_AMPLIFICATION: RefCell<FixedU128> = RefCell::new(FixedU128::one());
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
    pub AssetFeeDecay: FixedU128= ASSET_FEE_DECAY.with(|v| *v.borrow());
    pub AssetFeeAmplification: FixedU128= ASSET_FEE_AMPLIFICATION.with(|v| *v.borrow());
    pub AssetMinimumFee: Fee = ASSET_MIN_FEE.with(|v| *v.borrow());
    pub AssetMaximumFee: Fee = ASSET_MAX_FEE.with(|v| *v.borrow());

    pub ProtocolFeeDecay: FixedU128= PROTOCOL_FEE_DECAY.with(|v| *v.borrow());
    pub ProtocolFeeAmplification: FixedU128= PROTOCOL_FEE_AMPLIFICATION.with(|v| *v.borrow());
    pub ProtocolMinimumFee: Fee = PROTOCOL_MIN_FEE.with(|v| *v.borrow());
    pub ProtocolMaximumFee: Fee = PROTOCOL_MAX_FEE.with(|v| *v.borrow());
}

impl Config for Test {
    type Event = Event;
    type AssetId = AssetId;
    type OraclePeriod = u16;
    type BlockNumberProvider = System;
    type Oracle = OracleProvider;
    type SelectedPeriod = SelectedPeriod;
    type AssetFeeDecay = AssetFeeDecay;
    type AssetFeeAmplification = AssetFeeAmplification;
    type AssetMinimumFee = AssetMinimumFee;
    type AssetMaximumFee = AssetMaximumFee;
    type ProtocolFeeDecay = ProtocolFeeDecay;
    type ProtocolFeeAmplification = ProtocolFeeAmplification;
    type ProtocolMinimumFee = ProtocolMinimumFee;
    type ProtocolMaximumFee = ProtocolMaximumFee;
    type Fee = Fee;
}

pub struct ExtBuilder {
    initial_fee: (Fee, Fee, u64),
}

impl Default for ExtBuilder {
    fn default() -> Self {
        ASSET_MIN_FEE.with(|v| {
            *v.borrow_mut() = Fee::from_percent(1);
        });
        ASSET_MAX_FEE.with(|v| {
            *v.borrow_mut() = Fee::from_percent(40);
        });
        ASSET_FEE_DECAY.with(|v| {
            *v.borrow_mut() = FixedU128::zero();
        });
        ASSET_FEE_AMPLIFICATION.with(|v| {
            *v.borrow_mut() = FixedU128::one();
        });
        ORACLE.with(|v| {
            *v.borrow_mut() = Box::new(Oracle::new());
        });

        Self {
            initial_fee: (Fee::zero(), Fee::zero(), 0),
        }
    }
}

impl ExtBuilder {
    pub fn with_asset_fee_params(self, min_fee: Fee, max_fee: Fee, decay: FixedU128, amplification: FixedU128) -> Self {
        ASSET_MIN_FEE.with(|v| {
            *v.borrow_mut() = min_fee;
        });
        ASSET_MAX_FEE.with(|v| {
            *v.borrow_mut() = max_fee;
        });
        ASSET_FEE_DECAY.with(|v| {
            *v.borrow_mut() = decay;
        });
        ASSET_FEE_AMPLIFICATION.with(|v| {
            *v.borrow_mut() = amplification;
        });

        self
    }

    pub fn with_protocol_fee_params(
        self,
        min_fee: Fee,
        max_fee: Fee,
        decay: FixedU128,
        amplification: FixedU128,
    ) -> Self {
        PROTOCOL_MIN_FEE.with(|v| {
            *v.borrow_mut() = min_fee;
        });
        PROTOCOL_MAX_FEE.with(|v| {
            *v.borrow_mut() = max_fee;
        });
        PROTOCOL_FEE_DECAY.with(|v| {
            *v.borrow_mut() = decay;
        });
        PROTOCOL_FEE_AMPLIFICATION.with(|v| {
            *v.borrow_mut() = amplification;
        });

        self
    }

    pub fn with_oracle(self, oracle: impl CustomOracle + 'static) -> Self {
        ORACLE.with(|v| {
            *v.borrow_mut() = Box::new(oracle);
        });
        self
    }

    pub fn with_initial_fees(mut self, asset_fee: Fee, protocol_fee: Fee, block_number: u64) -> Self {
        self.initial_fee = (asset_fee, protocol_fee, block_number);
        self
    }

    pub fn build(self) -> sp_io::TestExternalities {
        let mut r: sp_io::TestExternalities = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap()
            .into();
        r.execute_with(|| {
            crate::AssetFee::<Test>::insert(HDX, (self.initial_fee.0, self.initial_fee.1, self.initial_fee.2));
        });

        r
    }
}

pub struct OracleProvider;

impl VolumeProvider<AssetId, Balance, u16> for OracleProvider {
    type Volume = AssetVolume;

    fn asset_volume(asset_id: AssetId, _period: u16) -> Option<Self::Volume> {
        let volume = ORACLE.with(|v| v.borrow().volume(asset_id, BLOCK.with(|v| *v.borrow())));
        Some(volume)
    }

    fn asset_liquidity(asset_id: AssetId, _period: u16) -> Option<Balance> {
        let liquidity = ORACLE.with(|v| v.borrow().liquidity(asset_id, BLOCK.with(|v| *v.borrow())));
        Some(liquidity)
    }
}

#[derive(Default, Clone, Debug)]
pub struct AssetVolume {
    pub(crate) amount_in: Balance,
    pub(crate) amount_out: Balance,
}

impl Volume<Balance> for AssetVolume {
    fn amount_in(&self) -> Balance {
        self.amount_in
    }

    fn amount_out(&self) -> Balance {
        self.amount_out
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

pub trait CustomOracle {
    fn volume(&self, _asset_id: AssetId, block: usize) -> AssetVolume;

    fn liquidity(&self, _asset_id: AssetId, block: usize) -> Balance;
}

pub(crate) fn retrieve_fee_entry(asset_id: AssetId) -> (Fee, Fee) {
    <UpdateAndRetrieveFees<Test> as GetByKey<AssetId, (Fee, Fee)>>::get(&asset_id)
}
