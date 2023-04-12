use frame_support::pallet_prelude::*;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::FixedU128;

use hydra_dx_math::dynamic_fees::types::FeeParams as MathFeeParams;

use scale_info::TypeInfo;

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct FeeParams<Fee> {
    pub(crate) min_fee: Fee,
    pub(crate) max_fee: Fee,
    pub(crate) decay: FixedU128,
    pub(crate) amplification: FixedU128,
}

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct FeeEntry<Fee, Block> {
    pub asset_fee: Fee,
    pub protocol_fee: Fee,
    pub timestamp: Block,
}

impl<Fee> From<FeeParams<Fee>> for MathFeeParams<Fee> {
    fn from(value: FeeParams<Fee>) -> Self {
        MathFeeParams {
            min_fee: value.min_fee,
            max_fee: value.max_fee,
            decay: value.decay,
            amplification: value.amplification,
        }
    }
}
