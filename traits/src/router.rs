use codec::{Decode, Encode};
use scale_info::TypeInfo;

#[derive(Encode, Decode, Clone, Copy, Debug, Eq, PartialEq, TypeInfo)]
pub enum PoolType<AssetId> {
    XYK,
    Stableswap(AssetId),
    Omnipool,
}

#[derive(Debug, PartialEq)]
pub enum ExecutorError<E> {
    NotSupported,
    Error(E),
}

#[derive(Encode, Decode, Clone, Copy, Debug, Eq, PartialEq, TypeInfo)]
pub struct AmountWithFee<Balance> {
    pub amount: Balance,
    pub fee: Balance,
}

impl<Balance: Default> AmountWithFee<Balance> {
    pub fn new(amount: Balance, fee: Balance) -> Self {
        AmountWithFee { amount, fee }
    }

    pub fn new_without_fee(amount: Balance) -> Self {
        AmountWithFee {
            amount,
            fee: Balance::default(),
        }
    }
}

pub trait TradeExecution<AccountId, AssetId, Balance> {
    type TradeCalculationResult;
    type Error;

    fn calculate_sell(
        pool_type: PoolType<AssetId>,
        asset_in: AssetId,
        asset_out: AssetId,
        amount_in: Self::TradeCalculationResult,
    ) -> Result<Self::TradeCalculationResult, ExecutorError<Self::Error>>;

    fn calculate_buy(
        pool_type: PoolType<AssetId>,
        asset_in: AssetId,
        asset_out: AssetId,
        amount_out: Self::TradeCalculationResult,
    ) -> Result<Self::TradeCalculationResult, ExecutorError<Self::Error>>;

    fn execute_sell(
        pool_type: PoolType<AssetId>,
        who: &AccountId,
        asset_in: AssetId,
        asset_out: AssetId,
        amount_in: Self::TradeCalculationResult,
    ) -> Result<(), ExecutorError<Self::Error>>;

    fn execute_buy(
        pool_type: PoolType<AssetId>,
        who: &AccountId,
        asset_in: AssetId,
        asset_out: AssetId,
        amount_out: Self::TradeCalculationResult,
    ) -> Result<(), ExecutorError<Self::Error>>;
}

#[impl_trait_for_tuples::impl_for_tuples(1, 5)]
impl<R: Copy, E: PartialEq, AccountId, AssetId: Copy, Balance: Copy> TradeExecution<AccountId, AssetId, Balance>
    for Tuple
{
    for_tuples!( where #(Tuple: TradeExecution<AccountId, AssetId, Balance, TradeCalculationResult=R, Error=E>)*);
    type TradeCalculationResult = R;
    type Error = E;

    fn calculate_sell(
        pool_type: PoolType<AssetId>,
        asset_in: AssetId,
        asset_out: AssetId,
        amount_in: Self::TradeCalculationResult,
    ) -> Result<Self::TradeCalculationResult, ExecutorError<Self::Error>> {
        for_tuples!(
            #(
                let value = match Tuple::calculate_sell(pool_type, asset_in,asset_out,amount_in) {
                    Ok(result) => return Ok(result),
                    Err(v) if v == ExecutorError::NotSupported => v,
                    Err(v) => return Err(v),
                };
            )*
        );
        Err(value)
    }

    fn calculate_buy(
        pool_type: PoolType<AssetId>,
        asset_in: AssetId,
        asset_out: AssetId,
        amount_out: Self::TradeCalculationResult,
    ) -> Result<Self::TradeCalculationResult, ExecutorError<Self::Error>> {
        for_tuples!(
            #(
                let value = match Tuple::calculate_buy(pool_type, asset_in,asset_out,amount_out) {
                    Ok(result) => return Ok(result),
                    Err(v) if v == ExecutorError::NotSupported => v,
                    Err(v) => return Err(v),
                };
            )*
        );
        Err(value)
    }

    fn execute_sell(
        pool_type: PoolType<AssetId>,
        who: &AccountId,
        asset_in: AssetId,
        asset_out: AssetId,
        amount_in: Self::TradeCalculationResult,
    ) -> Result<(), ExecutorError<Self::Error>> {
        for_tuples!(
            #(
                let value = match Tuple::execute_sell(pool_type, who, asset_in, asset_out, amount_in) {
                    Ok(result) => return Ok(result),
                    Err(v) if v == ExecutorError::NotSupported => v,
                    Err(v) => return Err(v),
                };
            )*
        );
        Err(value)
    }

    fn execute_buy(
        pool_type: PoolType<AssetId>,
        who: &AccountId,
        asset_in: AssetId,
        asset_out: AssetId,
        amount_out: Self::TradeCalculationResult,
    ) -> Result<(), ExecutorError<Self::Error>> {
        for_tuples!(
            #(
                let value = match Tuple::execute_buy(pool_type, who,asset_in, asset_out, amount_out) {
                    Ok(result) => return Ok(result),
                    Err(v) if v == ExecutorError::NotSupported => v,
                    Err(v) => return Err(v),
                };
            )*
        );
        Err(value)
    }
}
