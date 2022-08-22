// This file is part of HydraDX.

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
#[warn(non_upper_case_globals)]
use crate as router;
use crate::Config;
use frame_support::parameter_types;
use frame_support::traits::{Everything, Nothing};
use frame_system as system;
use hydradx_traits::router::{Executor, ExecutorError, PoolType};
use orml_traits::parameter_type_with_key;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup, One},
};
use std::borrow::Borrow;
use std::ops::Deref;
use std::{cell::RefCell};
use crate::types::Trade;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

pub type AssetId = u32;
pub type Balance = u128;

frame_support::construct_runtime!(
    pub enum Test where
     Block = Block,
     NodeBlock = Block,
     UncheckedExtrinsic = UncheckedExtrinsic,
     {
         System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
         Router: router::{Pallet, Call,Event<T>},
         Currency: orml_tokens::{Pallet, Event<T>},
     }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 63;
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
    type Event = ();
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

pub type Amount = i128;

parameter_type_with_key! {
    pub ExistentialDeposits: |_currency_id: AssetId| -> Balance {
        One::one()
    };
}

impl orml_tokens::Config for Test {
    type Event = ();
    type Balance = Balance;
    type Amount = Amount;
    type CurrencyId = AssetId;
    type WeightInfo = ();
    type ExistentialDeposits = ExistentialDeposits;
    type OnDust = ();
    type MaxLocks = ();
    type DustRemovalWhitelist = Nothing;
    type OnNewTokenAccount = ();
    type OnKilledTokenAccount = ();
}

type Pools = (XYK, StableSwap, OmniPool);

impl Config for Test {
    type Event = ();
    type AssetId = AssetId;
    type Balance = Balance;
    type Currency = Currency;
    type AMM = Pools;
}

pub type AccountId = u64;

pub const ALICE: AccountId = 1;

pub const BSX: AssetId = 1000;
pub const AUSD: AssetId = 1001;
pub const MOVR: AssetId = 1002;
pub const KSM: AssetId = 1003;


pub const XYK_SELL_CALCULATION_RESULT: u128 = 6;
pub const XYK_BUY_CALCULATION_RESULT: u128 = 5;
pub const STABLESWAP_SELL_CALCULATION_RESULT: u128 = 4;
pub const STABLESWAP_BUY_CALCULATION_RESULT: u128 = 3;
pub const OMNIPOOL_SELL_CALCULATION_RESULT: u128 = 2;
pub const OMNIPOOL_BUY_CALCULATION_RESULT: u128 = 1;
pub const INVALID_CALCULATION_AMOUNT: u128 = 999999999;

pub const BSX_AUSD_TRADE_IN_XYK : Trade<AssetId> = Trade {
    pool: PoolType::XYK,
    asset_in: BSX,
    asset_out: AUSD,
};

pub struct ExtBuilder {
}

// Returns default values for genesis config
impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
        }
    }
}

impl ExtBuilder {

    pub fn build(self) -> sp_io::TestExternalities {
        let t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

        t.into()
    }
}

thread_local! {
    pub static EXECUTED_SELLS: RefCell<Vec<(PoolType<AssetId>, Balance, AssetId, AssetId)>> = RefCell::new(Vec::default());
    pub static EXECUTED_BUYS: RefCell<Vec<(PoolType<AssetId>, Balance, AssetId, AssetId)>> = RefCell::new(Vec::default());
}

macro_rules! impl_fake_executor {
    ($pool_struct:ident, $pool_type: pat, $sell_calculation_result: expr, $buy_calculation_result: expr)=>{
            impl Executor<AccountId, AssetId, Balance> for $pool_struct {
                type Output = Balance;
                type Error = ();

                fn calculate_sell(
                    pool_type: PoolType<AssetId>,
                    _asset_in: AssetId,
                    _asset_out: AssetId,
                    amount_in: Balance,
                ) -> Result<Self::Output, ExecutorError<Self::Error>> {
                    if !matches!(pool_type, $pool_type) {
                        return Err(ExecutorError::NotSupported);
                    }

                    if amount_in == INVALID_CALCULATION_AMOUNT {
                        return Err(ExecutorError::Error(()));
                    }

                    Ok($sell_calculation_result)
                }

                fn calculate_buy(
                    pool_type: PoolType<AssetId>,
                    _asset_in: AssetId,
                    _asset_out: AssetId,
                    amount_out: Balance,
                ) -> Result<Self::Output, ExecutorError<Self::Error>> {
                    if !matches!(pool_type, $pool_type) {
                        return Err(ExecutorError::NotSupported);
                    }

                    if amount_out == INVALID_CALCULATION_AMOUNT {
                        return Err(ExecutorError::Error(()));
                    }

                    Ok($buy_calculation_result)
                }

                fn execute_sell(
                    pool_type: PoolType<AssetId>,
                    _who: &AccountId,
                    asset_in: AssetId,
                    asset_out: AssetId,
                    amount_in: Balance,
                ) -> Result<(), ExecutorError<Self::Error>> {
                    EXECUTED_SELLS.with(|v| {
                        let mut m = v.borrow_mut();
                        m.push((pool_type, amount_in, asset_in, asset_out));
                    });

                    Ok(())
                }

                fn execute_buy(
                    pool_type: PoolType<AssetId>,
                    _who: &AccountId,
                    asset_in: AssetId,
                    asset_out: AssetId,
                    amount_out: Balance,
                ) -> Result<(), ExecutorError<Self::Error>> {
                    EXECUTED_BUYS.with(|v| {
                        let mut m = v.borrow_mut();
                        m.push((pool_type, amount_out, asset_in, asset_out));
                    });

                    Ok(())
                }
            }
    }
}

pub struct XYK;
pub struct StableSwap;
pub struct OmniPool;

impl_fake_executor!(XYK, PoolType::XYK, XYK_SELL_CALCULATION_RESULT, XYK_BUY_CALCULATION_RESULT);
impl_fake_executor!(StableSwap, PoolType::Stableswap(_), STABLESWAP_SELL_CALCULATION_RESULT, STABLESWAP_BUY_CALCULATION_RESULT);
impl_fake_executor!(OmniPool, PoolType::Omnipool, OMNIPOOL_SELL_CALCULATION_RESULT, OMNIPOOL_BUY_CALCULATION_RESULT);

pub fn assert_executed_sell_trades(expected_trades: Vec<(PoolType<AssetId>, Balance, AssetId, AssetId)>) {
    EXECUTED_SELLS.borrow().with(|v| {
        let trades = v.borrow().deref().clone();
        assert_eq!(expected_trades, trades);
    });
}
