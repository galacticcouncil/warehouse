use frame_support::sp_runtime::DispatchError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaymentInfo<Balance, AssetId, Price> {
    Native(Balance),
    NonNative(Balance, AssetId, Price),
}

pub trait TransactionMultiPaymentDataProvider<AccountId, AssetId, Price> {
    fn get_currency_and_price(who: &AccountId) -> Result<(AssetId, Option<Price>), DispatchError>;

    fn get_fallback_account() -> Option<AccountId>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaymentWithdrawResult {
    Native,
    Transferred,
}

pub trait CurrencyWithdraw<AccountId, Balance> {
    fn withdraw(
        who: &AccountId,
        fee: Balance,
    ) -> Result<PaymentWithdrawResult, frame_support::sp_runtime::DispatchError>;
}