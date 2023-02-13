// This file is part of galacticcouncil/warehouse.
// Copyright (C) 2020-2022  Intergalactic, Limited (GIB). SPDX-License-Identifier: Apache-2.0

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

use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

pub type Balance = u128;
pub type NamedReserveIdentifier = [u8; 8];
pub type OrderId = u32;

#[derive(Encode, Decode, Debug, Eq, PartialEq, Clone, TypeInfo, MaxEncodedLen)]
pub struct Order<AccountId, AssetId> {
    pub owner: AccountId,
    pub asset_buy: AssetId,
    pub asset_sell: AssetId,
    pub amount_buy: Balance,
    pub partially_fillable: bool,
}
