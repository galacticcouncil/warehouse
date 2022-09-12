use codec::{Decode, Encode};
use hydradx_traits::router::PoolType;
use scale_info::TypeInfo;

///A single trade for buy/sell, describing the asset pair and the pool type in which the trade is executed
#[derive(Encode, Decode, Debug, Eq, PartialEq, Copy, Clone, TypeInfo)]
pub struct Trade<AssetId> {
    pub pool: PoolType<AssetId>,
    pub asset_in: AssetId,
    pub asset_out: AssetId,
}
