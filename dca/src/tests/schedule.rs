// This file is part of HydraDX.

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

use crate::tests::mock::*;
use crate::{Error, Event, Order, PoolType, Recurrence, Schedule, Trade};
use frame_support::{assert_noop, assert_ok};
use frame_system::pallet_prelude::BlockNumberFor;
use pretty_assertions::assert_eq;
use sp_runtime::DispatchError;
use sp_runtime::DispatchError::BadOrigin;
use sp_runtime::BoundedVec;

#[test]
fn schedule_should_store_schedule_for_next_block_when_no_blocknumber_specified() {
    ExtBuilder::default().build().execute_with(|| {
        //Arrange
        let trades = vec![Trade {
            asset_in: 3,
            asset_out: 4,
            pool: PoolType::XYK
        }];

        let bounded_vec: BoundedVec<Trade, sp_runtime::traits::ConstU32<5>> =
            trades.try_into().unwrap();

        let schedule = Schedule {
            period: 1,
            order: Order {
                asset_in: 3,
                asset_out: 4,
                amount_in: 1000,
                amount_out: 2000,
                limit: 0,
                route: bounded_vec
            },
            recurrence: Recurrence::Fixed
        };

        //Act
        assert_ok!(Dca::schedule(
            Origin::signed(ALICE),
            schedule,
            Option::None
        ));

        let stored_schedule = Dca::schedules(1).unwrap();


        //Assert
    });
}

//TODO: add negative case for validating block numbers