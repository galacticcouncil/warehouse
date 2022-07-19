// This file is part of galacticcouncil/warehouse.

// Copyright (C) 2020-2022  Intergalactic, Limited (GIB).
// SPDX-License-Identifier: Apache-2.0

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

use frame_support::{dispatch::DispatchResult, traits::tokens::nonfungibles::Create};

pub trait CreateTypedClass<AccountId, ClassId, ClassType>: Create<AccountId> {
    /// This function reate nft class of `class_type` type.
    fn create_typed_class(owner: AccountId, class_id: ClassId, class_type: ClassType) -> DispatchResult;
}

pub trait ReserveClassIdUpTo<ClassId> {
    /// This function return `true` if class id is from reserved range, `false` otherwise.
    fn is_id_reserved(id: ClassId) -> bool;
}
