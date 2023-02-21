use frame_support::sp_runtime::DispatchResult;

pub trait SpotPriceProvider<AssetId> {
    type Price;

    fn pair_exists(asset_a: AssetId, asset_b: AssetId) -> bool;

    /// Return spot price for given asset pair
    ///
    /// Returns None if such pair does not exist
    fn spot_price(asset_a: AssetId, asset_b: AssetId) -> Option<Self::Price>;
}

/// Manage list of non-dustable accounts
pub trait DustRemovalAccountWhitelist<AccountId> {
    type Error;

    /// Add account to the list.
    fn add_account(account: &AccountId) -> Result<(), Self::Error>;

    /// Remove an account from the list.
    fn remove_account(account: &AccountId) -> Result<(), Self::Error>;
}

/// AMM trader to define trading functionalities
pub trait AMMTrader<Origin, AssetId, Balance> {
    fn sell(
        origin: Origin,
        asset_in: AssetId,
        asset_out: AssetId,
        amount: Balance,
        min_buy_amount: Balance,
    ) -> DispatchResult;

    fn buy(
        origin: Origin,
        asset_in: AssetId,
        asset_out: AssetId,
        amount: Balance,
        max_sell_amount: Balance,
    ) -> DispatchResult;
}

pub trait PriceProvider<AssetId> {
    type Price;

    /// Return spot price for given asset pair
    ///
    /// Returns None if such pair does not exist
    fn spot_price(asset_a: AssetId, asset_b: AssetId) -> Option<Self::Price>;
}
