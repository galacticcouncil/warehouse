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

#![cfg(feature = "runtime-benchmarks")]

use super::*;

pub const HDX: AssetId = 1_000;
pub const DOT: AssetId = 2_000;

use frame_benchmarking::benchmarks;
use frame_support::traits::Hooks;

use crate::Pallet as EmaOracle;

pub const MAX_TOKENS: u32 = 700;

benchmarks! {
    on_finalize_no_entry {
        let block_num: u32 = 5;
    }: { EmaOracle::<T>::on_finalize(block_num.into()); }
    verify {
    }

    #[extra]
    on_finalize_insert_one_token {
        let block_num: T::BlockNumber = 5u32.into();
        let prev_block = block_num.saturating_sub(One::one());

        frame_system::Pallet::<T>::set_block_number(prev_block);
        EmaOracle::<T>::on_initialize(prev_block);
        EmaOracle::<T>::on_finalize(prev_block);

        frame_system::Pallet::<T>::set_block_number(block_num);
        EmaOracle::<T>::on_initialize(block_num);

        let (amount_in, amount_out, liquidity) = (1_000_000_000_000, 2_000_000_000_000, 500_000_000_000);
        OnActivityHandler::<T>::on_trade(HDX, DOT, amount_in, amount_out, liquidity);
        let entry = OracleEntry {
            price: Price::from((amount_in, amount_out)),
            volume: Volume::from_a_in_b_out(amount_in, amount_out),
            liquidity,
            timestamp: block_num,
        };

        assert_eq!(Accumulator::<T>::get(), [(derive_name(HDX, DOT), entry.clone())].into_iter().collect());

    }: { EmaOracle::<T>::on_finalize(block_num); }
    verify {
        assert!(Accumulator::<T>::get().is_empty());
        assert_eq!(Oracles::<T>::get(derive_name(HDX, DOT), into_blocks::<T>(&LastBlock)).unwrap(), (entry, block_num));
    }

    #[extra]
    on_finalize_update_one_token {
        let initial_data_block: T::BlockNumber = 5u32.into();
        // higher update time difference might make exponentiation more expensive
        let block_num = initial_data_block.saturating_add(1_000_000u32.into());

        frame_system::Pallet::<T>::set_block_number(initial_data_block);
        EmaOracle::<T>::on_initialize(initial_data_block);
        let (amount_in, amount_out, liquidity) = (1_000_000_000_000, 2_000_000_000_000, 500_000_000_000);
        OnActivityHandler::<T>::on_trade(HDX, DOT, amount_in, amount_out, liquidity);
        EmaOracle::<T>::on_finalize(initial_data_block);

        frame_system::Pallet::<T>::set_block_number(block_num);
        EmaOracle::<T>::on_initialize(block_num);

        OnActivityHandler::<T>::on_trade(HDX, DOT, amount_in, amount_out, liquidity);
        let entry = OracleEntry {
            price: Price::from((amount_in, amount_out)),
            volume: Volume::from_a_in_b_out(amount_in, amount_out),
            liquidity,
            timestamp: block_num,
        };

        assert_eq!(Accumulator::<T>::get(), [(derive_name(HDX, DOT), entry.clone())].into_iter().collect());

    }: { EmaOracle::<T>::on_finalize(block_num); }
    verify {
        assert!(Accumulator::<T>::get().is_empty());
        assert_eq!(Oracles::<T>::get(derive_name(HDX, DOT), into_blocks::<T>(&LastBlock)).unwrap(), (entry, initial_data_block));
    }

    on_finalize_multiple_tokens {
        let b in 1 .. MAX_TOKENS;

        let initial_data_block: T::BlockNumber = 5u32.into();
        let block_num = initial_data_block.saturating_add(1_000_000u32.into());

        frame_system::Pallet::<T>::set_block_number(initial_data_block);
        EmaOracle::<T>::on_initialize(initial_data_block);
        let (amount_in, amount_out, liquidity) = (1_000_000_000_000, 2_000_000_000_000, 500_000_000_000);
        for i in 0 .. b {
            let asset_a = i * 1_000;
            let asset_b = asset_a + 500;
            OnActivityHandler::<T>::on_trade(asset_a, asset_b, amount_in, amount_out, liquidity);
        }
        EmaOracle::<T>::on_finalize(initial_data_block);

        frame_system::Pallet::<T>::set_block_number(block_num);
        EmaOracle::<T>::on_initialize(block_num);
        for i in 0 .. b {
            let asset_a = i * 1_000;
            let asset_b = asset_a + 500;
            OnActivityHandler::<T>::on_trade(asset_a, asset_b, amount_in, amount_out, liquidity);
        }
    }: { EmaOracle::<T>::on_finalize(block_num); }
    verify {
        let entry = OracleEntry {
            price: Price::from((amount_in, amount_out)),
            volume: Volume::from_a_in_b_out(amount_in, amount_out),
            liquidity,
            timestamp: block_num,
        };

        for i in 0 .. b {
            let asset_a = i * 1_000;
            let asset_b = asset_a + 500;
            assert_eq!(Oracles::<T>::get(derive_name(asset_a, asset_b), into_blocks::<T>(&LastBlock)).unwrap(), (entry.clone(), initial_data_block));
        }
    }

    on_trade_multiple_tokens {
        let b in 1 .. MAX_TOKENS;

        let initial_data_block: T::BlockNumber = 5u32.into();
        let block_num = initial_data_block.saturating_add(1_000_000u32.into());

        let mut entries = Vec::new();

        frame_system::Pallet::<T>::set_block_number(initial_data_block);
        EmaOracle::<T>::on_initialize(initial_data_block);
        let (amount_in, amount_out, liquidity) = (1_000_000_000_000, 2_000_000_000_000, 500_000_000_000);
        for i in 0 .. b {
            let asset_a = i * 1_000;
            let asset_b = asset_a + 500;
            OnActivityHandler::<T>::on_trade(asset_a, asset_b, amount_in, amount_out, liquidity);
        }
        EmaOracle::<T>::on_finalize(initial_data_block);

        frame_system::Pallet::<T>::set_block_number(block_num);
        EmaOracle::<T>::on_initialize(block_num);
        let entry = OracleEntry {
            price: Price::from((amount_in, amount_out)),
            volume: Volume::from_a_in_b_out(amount_in, amount_out),
            liquidity,
            timestamp: block_num,
        };
        for i in 0 .. b {
            let asset_a = i * 1_000;
            let asset_b = asset_a + 500;
            OnActivityHandler::<T>::on_trade(asset_a, asset_b, amount_in, amount_out, liquidity);
            entries.push((derive_name(asset_a, asset_b), entry.clone()));
        }
        let asset_a = b * 1_000;
        let asset_b = asset_a + 500;
    }: { OnActivityHandler::<T>::on_trade(asset_a, asset_b, amount_in, amount_out, liquidity); }
    verify {
        entries.push((derive_name(asset_a, asset_b), entry.clone()));

        assert_eq!(Accumulator::<T>::get(), entries.into_iter().collect());
    }

    on_liquidity_changed_multiple_tokens {
        let b in 1 .. MAX_TOKENS;

        let initial_data_block: T::BlockNumber = 5u32.into();
        let block_num = initial_data_block.saturating_add(1_000_000u32.into());

        let mut entries = Vec::new();

        frame_system::Pallet::<T>::set_block_number(initial_data_block);
        EmaOracle::<T>::on_initialize(initial_data_block);
        let (amount_in, amount_out, liquidity) = (1_000_000_000_000, 2_000_000_000_000, 500_000_000_000);
        for i in 0 .. b {
            let asset_a = i * 1_000;
            let asset_b = asset_a + 500;
            OnActivityHandler::<T>::on_trade(asset_a, asset_b, amount_in, amount_out, liquidity);
        }
        EmaOracle::<T>::on_finalize(initial_data_block);

        frame_system::Pallet::<T>::set_block_number(block_num);
        EmaOracle::<T>::on_initialize(block_num);
        let entry = OracleEntry {
            price: Price::from((amount_in, amount_out)),
            volume: Volume::from_a_in_b_out(amount_in, amount_out),
            liquidity,
            timestamp: block_num,
        };
        for i in 0 .. b {
            let asset_a = i * 1_000;
            let asset_b = asset_a + 500;
            OnActivityHandler::<T>::on_trade(asset_a, asset_b, amount_in, amount_out, liquidity);
            entries.push((derive_name(asset_a, asset_b), entry.clone()));
        }
        let asset_a = b * 1_000;
        let asset_b = asset_a + 500;
        let amount_a = amount_in;
        let amount_b = amount_out;
    }: { OnActivityHandler::<T>::on_liquidity_changed(asset_a, asset_b, amount_a, amount_b, liquidity); }
    verify {
        let liquidity_entry = OracleEntry {
            price: Price::from((amount_a, amount_b)),
            volume: Volume::default(),
            liquidity,
            timestamp: block_num,
        };
        entries.push((derive_name(asset_a, asset_b), liquidity_entry.clone()));

        assert_eq!(Accumulator::<T>::get(), entries.into_iter().collect());
    }

    get_entry {
        let b = MAX_TOKENS;

        let initial_data_block: T::BlockNumber = 5u32.into();
        let oracle_age: T::BlockNumber = 999_999u32.into();
        let block_num = initial_data_block.saturating_add(oracle_age.saturating_add(One::one()));

        frame_system::Pallet::<T>::set_block_number(initial_data_block);
        EmaOracle::<T>::on_initialize(initial_data_block);
        let (amount_in, amount_out, liquidity) = (1_000_000_000_000, 2_000_000_000_000, 500_000_000_000);
        let asset_a = 1_000;
        let asset_b = asset_a + 500;
        OnActivityHandler::<T>::on_trade(asset_a, asset_b, amount_in, amount_out, liquidity);
        EmaOracle::<T>::on_finalize(initial_data_block);

        frame_system::Pallet::<T>::set_block_number(block_num);
        EmaOracle::<T>::on_initialize(block_num);

        let res = core::cell::RefCell::new(Err(OracleError::NotPresent));

    }: { let _ = res.replace(EmaOracle::<T>::get_entry(asset_a, asset_b, TenMinutes)); }
    verify {
        assert_eq!(*res.borrow(), Ok(AggregatedEntry {
            price: Price::from((amount_in, amount_out)),
            volume: Volume::from_a_in_b_out(amount_in, amount_out),
            liquidity,
            oracle_age,
        }));
    }

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Test);
}
