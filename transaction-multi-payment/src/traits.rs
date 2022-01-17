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
