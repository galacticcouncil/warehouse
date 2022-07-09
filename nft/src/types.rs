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

use frame_support::pallet_prelude::*;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use scale_info::TypeInfo;

/// NFT Class ID
pub type ClassId = u128;

/// NFT Instance ID
pub type InstanceId = u128;

#[derive(Encode, Decode, Eq, PartialEq, Clone, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct ClassInfo<ClassType, BoundedVec> {
    /// A class type that implies permissions, e.g. for transfer and other operations
    pub class_type: ClassType,
    /// Arbitrary data about a class, e.g. IPFS hash
    pub metadata: BoundedVec,
}

#[derive(Encode, Decode, Eq, Copy, PartialEq, Clone, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct InstanceInfo<BoundedVec> {
    pub metadata: BoundedVec,
}

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum ClassType {
    Marketplace = 0_isize,
    LiquidityMining = 1_isize,
    Redeemable = 2_isize,
    Auction = 3_isize,
    HydraHeads = 4_isize,
}

impl Default for ClassType {
    fn default() -> Self {
        ClassType::Marketplace
    }
}

pub trait NftPermission<InnerClassType> {
    fn can_create(class_type: &InnerClassType) -> bool;
    fn can_mint(class_type: &InnerClassType) -> bool;
    fn can_transfer(class_type: &InnerClassType) -> bool;
    fn can_burn(class_type: &InnerClassType) -> bool;
    fn can_destroy(class_type: &InnerClassType) -> bool;
    fn has_deposit(class_type: &InnerClassType) -> bool;
}

#[derive(Encode, Decode, Eq, Copy, PartialEq, Clone, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct NftPermissions;

impl NftPermission<ClassType> for NftPermissions {
    fn can_create(class_type: &ClassType) -> bool {
        matches!(*class_type, ClassType::Marketplace)
    }

    fn can_mint(class_type: &ClassType) -> bool {
        matches!(*class_type, ClassType::Marketplace)
    }

    fn can_transfer(class_type: &ClassType) -> bool {
        matches!(*class_type, ClassType::Marketplace | ClassType::LiquidityMining)
    }

    fn can_burn(class_type: &ClassType) -> bool {
        matches!(*class_type, ClassType::Marketplace)
    }

    fn can_destroy(class_type: &ClassType) -> bool {
        matches!(*class_type, ClassType::Marketplace)
    }

    fn has_deposit(class_type: &ClassType) -> bool {
        matches!(*class_type, ClassType::Marketplace)
    }
}
