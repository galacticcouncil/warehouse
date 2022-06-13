use frame_support::sp_runtime::{DispatchError, DispatchResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaymentInfo<Balance, AssetId, Price> {
    Native(Balance),
    NonNative(Balance, AssetId, Price),
}

/// Helper method for providing some data that are needed in OnChargeTransaction
pub trait TransactionMultiPaymentDataProvider<AccountId, AssetId, Price> {
    /// Get a fee currency set by an account and its price
    fn get_currency_and_price(who: &AccountId) -> Result<(AssetId, Option<Price>), DispatchError>;

    /// Returns the account where fees are deposited
    fn get_fee_receiver() -> AccountId;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaymentWithdrawResult {
    Native,
    Transferred,
}

pub trait CurrencyWithdraw<AccountId, Balance> {
    fn withdraw(who: &AccountId, fee: Balance) -> Result<PaymentWithdrawResult, DispatchError>;
}

/// Handler for dealing with fees
pub trait DepositFee<AccountId, AssetId, Balance> {
    fn deposit_fee(who: &AccountId, currency: AssetId, amount: Balance) -> DispatchResult;
}
