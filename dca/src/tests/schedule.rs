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
use crate::{Error, Event, Order, Recurrence, Schedule};
use frame_support::{assert_noop, assert_ok};
use frame_system::pallet_prelude::BlockNumberFor;
use hydradx_traits::router::PoolType;
use pretty_assertions::assert_eq;
use sp_runtime::DispatchError;
use sp_runtime::DispatchError::BadOrigin;

#[test]
fn schedule() {
    ExtBuilder::default().build().execute_with(|| {
        //Arrange
        let schedule = Schedule {
            period: 1,
            order: Order {
                asset_in: 3,
                asset_out: 4,
                amount_in: 1000,
                amount_out: 2000,
                limit: 0,
                route: vec![]
            },
            recurrence: Recurrence::Fixed
        };

        //Act
        assert_ok!(Dca::schedule(
            Origin::signed(ALICE),
            schedule
        ));

        //Assert
    });
}
