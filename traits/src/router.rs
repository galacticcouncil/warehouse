use codec::{Decode, Encode};
use scale_info::TypeInfo;

#[derive(Encode, Decode, Clone, Copy, Debug, Eq, PartialEq, TypeInfo)]
pub enum PoolType<AssetId> {
    XYK,
    Stableswap(AssetId),
    Omnipool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ExecutorError<E> {
    NotSupported,
    Error(E),
}

pub trait TradeExecution<Origin, AccountId, AssetId, Balance> {
    type Error;

    fn calculate_sell(
        pool_type: PoolType<AssetId>,
        asset_in: AssetId,
        asset_out: AssetId,
        amount_in: Balance,
    ) -> Result<Balance, ExecutorError<Self::Error>>;

    fn calculate_buy(
        pool_type: PoolType<AssetId>,
        asset_in: AssetId,
        asset_out: AssetId,
        amount_out: Balance,
    ) -> Result<Balance, ExecutorError<Self::Error>>;

    fn execute_sell(
        who: &Origin,
        pool_type: PoolType<AssetId>,
        asset_in: AssetId,
        asset_out: AssetId,
        amount_in: Balance,
    ) -> Result<(), ExecutorError<Self::Error>>;

    fn execute_buy(
        who: &Origin,
        pool_type: PoolType<AssetId>,
        asset_in: AssetId,
        asset_out: AssetId,
        amount_out: Balance,
    ) -> Result<(), ExecutorError<Self::Error>>;
}

#[impl_trait_for_tuples::impl_for_tuples(1, 5)]
impl<E: PartialEq, Origin, AccountId, AssetId: Copy, Balance: Copy> TradeExecution<Origin, AccountId, AssetId, Balance>
    for Tuple
{
    for_tuples!( where #(Tuple: TradeExecution<Origin,AccountId, AssetId, Balance, Error=E>)*);
    type Error = E;

    fn calculate_sell(
        pool_type: PoolType<AssetId>,
        asset_in: AssetId,
        asset_out: AssetId,
        amount_in: Balance,
    ) -> Result<Balance, ExecutorError<Self::Error>> {
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
        amount_out: Balance,
    ) -> Result<Balance, ExecutorError<Self::Error>> {
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
        who: &Origin,
        pool_type: PoolType<AssetId>,
        asset_in: AssetId,
        asset_out: AssetId,
        amount_in: Balance,
    ) -> Result<(), ExecutorError<Self::Error>> {
        for_tuples!(
            #(
                let value = match Tuple::execute_sell(who,pool_type, asset_in, asset_out, amount_in) {
                    Ok(result) => return Ok(result),
                    Err(v) if v == ExecutorError::NotSupported => v,
                    Err(v) => return Err(v),
                };
            )*
        );
        Err(value)
    }

    fn execute_buy(
        who: &Origin,
        pool_type: PoolType<AssetId>,
        asset_in: AssetId,
        asset_out: AssetId,
        amount_out: Balance,
    ) -> Result<(), ExecutorError<Self::Error>> {
        for_tuples!(
            #(
                let value = match Tuple::execute_buy(who, pool_type,asset_in, asset_out, amount_out) {
                    Ok(result) => return Ok(result),
                    Err(v) if v == ExecutorError::NotSupported => v,
                    Err(v) => return Err(v),
                };
            )*
        );
        Err(value)
    }
}
