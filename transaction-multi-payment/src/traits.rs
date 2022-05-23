use frame_support::sp_runtime::{DispatchError, DispatchResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaymentInfo<Balance, AssetId, Price> {
    Native(Balance),
    NonNative(Balance, AssetId, Price),
}

pub trait TransactionMultiPaymentDataProvider<AccountId, AssetId, Price> {
    fn get_currency_and_price(who: &AccountId) -> Result<(AssetId, Option<Price>), DispatchError>;

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

pub trait DepositFee<AccountId, AssetId, Balance> {
    fn deposit_fee(who: &AccountId, amounts: impl Iterator<Item = (AssetId, Balance)>) -> DispatchResult;
}
