// This file is part of pallet-route-executor.

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

#![cfg(feature = "runtime-benchmarks")]

use super::*;

use frame_benchmarking::{account, benchmarks};
use sp_runtime::traits::UniqueSaturatedInto;
use frame_support::traits::{tokens::nonfungibles::InspectEnumerable, Currency, Get};
use frame_system::RawOrigin;
use sp_std::convert::TryInto;
use types::Trade;
use hydradx_traits::router::{ExecutorError, TradeCalculation, PoolType};


use crate::Pallet as RouteExecutor;


const CLASS_ID_0: u32 = 1_000_000;

const ENDOWMENT: u128 = 100_000_000_000_000_000_000;
//const INITIAL_BALANCE: Balance = 100_000_000_000_000_000;
const SEED: u32 = 0;

/*
fn create_account<T: Config>(name: &'static str, index: u32) -> T::AccountId {
    let caller: T::AccountId = account(name, index, SEED);
    <T as pallet_balances::Config>::Balance::set_balance(&caller, ENDOWMENT);
    caller
}*/


fn create_account<T: Config>(name: &'static str, index: u32) -> T::AccountId {
    let caller: T::AccountId = account(name, index, SEED);
    //<T::Currency as pallet_balances::Config>::transfer(&caller, ENDOWMENT.unique_saturated_into());
    caller
}

/*fn funded_account<T: Config>(name: &'static str, index: u32) -> T::AccountId {
    let caller: T::AccountId = account(name, index, SEED);

    <T as pallet_route_executor::Config>::Currency::deposit(
        BSX,
        &caller,
        INITIAL_BALANCE,
    )
        .unwrap();

    caller
}*/


benchmarks! {
    execute_sell {
        let caller = create_account::<T>("caller", 1);
        let routes = vec![
            Trade {
                pool: PoolType::XYK,
                asset_in: 1000u32.into(),
                asset_out: 1001u32.into(),
            }
        ];
    }: _(RawOrigin::Signed(caller), 1000u32.into(), 1001u32.into(), 10u32.into(), 9u32.into(),routes)
    verify {
        assert_eq!(3,3);
    }

}

#[cfg(test)]
mod bench_tests {
    use super::*;
    use crate::tests::mock::*;
    use frame_support::assert_ok;
    use frame_benchmarking::impl_benchmark_test_suite;

    impl_benchmark_test_suite!(Pallet, super::ExtBuilder::default().build(), super::Test);
}
