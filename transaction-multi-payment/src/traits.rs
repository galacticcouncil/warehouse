#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaymentSwapResult {
    Native,
    Swapped,
    Transferred,
}

pub trait CurrencySwap<AccountId, Balance> {
    fn swap(who: &AccountId, fee: Balance) -> Result<PaymentSwapResult, frame_support::sp_runtime::DispatchError>;
}

pub trait SpotPriceProvider<AssetId> {
    type Price;

    fn pair_exists(asset_a: AssetId, asset_b: AssetId) -> bool;

    /// Return spot price for given asset pair
    ///
    /// Returns None if such pair does not exist
    fn spot_price(asset_a: AssetId, asset_b: AssetId) -> Option<Self::Price>;
}
