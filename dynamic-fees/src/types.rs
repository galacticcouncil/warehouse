use frame_support::pallet_prelude::*;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::FixedU128;

use scale_info::TypeInfo;

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct FeeParams<Fee> {
    pub(crate) min_fee: Fee,
    pub(crate) max_fee: Fee,
    pub(crate) decay: FixedU128,
    pub(crate) amplification: FixedU128,
}
