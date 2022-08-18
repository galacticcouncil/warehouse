use codec::{Decode, Encode};
use hydradx_traits::router::PoolType;
use scale_info::TypeInfo;

#[derive(Encode, Decode, Debug, Eq, PartialEq, Copy, Clone, TypeInfo)]
pub struct Trade<AssetId> {
    pub pool: PoolType<AssetId>,
    pub asset_in: AssetId,
    pub asset_out: AssetId,
}
