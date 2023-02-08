pub trait Volume<Balance> {
    fn amount_in(&self) -> Balance;
    fn amount_out(&self) -> Balance;
}

pub trait VolumeProvider<AssetId, Balance, Period> {
    type Volume: Volume<Balance>;

    fn asset_volume(asset_id: AssetId, period: Period) -> Option<Self::Volume>;

    fn asset_liquidity(asset_id: AssetId, period: Period) -> Option<Balance>;
}
