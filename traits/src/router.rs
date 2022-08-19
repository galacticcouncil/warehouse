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

pub trait Executor<AccountId, AssetId, Balance> {
    type Output;
    type Error;

    fn calculate_sell(
        pool_type: PoolType<AssetId>,
        asset_in: AssetId,
        asset_out: AssetId,
        amount_in: Balance,
    ) -> Result<Self::Output, ExecutorError<Self::Error>>;

    fn calculate_buy(
        pool_type: PoolType<AssetId>,
        asset_in: AssetId,
        asset_out: AssetId,
        amount_out: Balance,
    ) -> Result<Self::Output, ExecutorError<Self::Error>>;

    fn execute_sell(
        pool_type: PoolType<AssetId>,
        who: &AccountId,
        asset_in: AssetId,
        asset_out: AssetId,
        amount_in: Balance,
    ) -> Result<(), ExecutorError<Self::Error>>;

    fn execute_buy(
        pool_type: PoolType<AssetId>,
        who: &AccountId,
        asset_in: AssetId,
        asset_out: AssetId,
        amount_out: Balance,
    ) -> Result<(), ExecutorError<Self::Error>>;
}

#[impl_trait_for_tuples::impl_for_tuples(1, 5)]
impl<O, E: PartialEq, AccountId, AssetId: Copy, Balance: Copy> Executor<AccountId, AssetId, Balance> for Tuple {
    for_tuples!( where #(Tuple: Executor<AccountId, AssetId, Balance, Output=O, Error=E>)*);
    type Output = O;
    type Error = E;

    fn calculate_sell(
        pool_type: PoolType<AssetId>,
        asset_in: AssetId,
        asset_out: AssetId,
        amount_in: Balance,
    ) -> Result<Self::Output, ExecutorError<Self::Error>> {
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
    ) -> Result<Self::Output, ExecutorError<Self::Error>> {
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
        amount_in: Balance,
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
        amount_out: Balance,
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

#[cfg(test)]
mod tests {
    use crate::router::{Executor, ExecutorError, PoolType};

    type Accountid = u64;
    type AssetId = u32;
    type Balance = u128;

    struct XYK;

    impl Executor<Accountid, AssetId, Balance> for XYK {
        type Output = u8;
        type Error = ();

        fn calculate_sell(
            pool_type: PoolType<AssetId>,
            asset_in: AssetId,
            asset_out: AssetId,
            amount_in: Balance,
        ) -> Result<Self::Output, ExecutorError<Self::Error>> {
            if pool_type != PoolType::XYK {
                return Err(ExecutorError::NotSupported);
            }

            if amount_in != 100u128 {
                return Err(ExecutorError::Error(()));
            }

            Ok(5u8)
        }

        fn calculate_buy(
            pool_type: PoolType<AssetId>,
            asset_in: AssetId,
            asset_out: AssetId,
            amount_out: Balance,
        ) -> Result<Self::Output, ExecutorError<Self::Error>> {
            todo!()
        }

        fn execute_sell(
            pool_type: PoolType<AssetId>,
            who: &Accountid,
            asset_in: AssetId,
            asset_out: AssetId,
            amount_in: Balance,
        ) -> Result<(), ExecutorError<Self::Error>> {
            todo!()
        }

        fn execute_buy(
            pool_type: PoolType<AssetId>,
            who: &Accountid,
            asset_in: AssetId,
            asset_out: AssetId,
            amount_out: Balance,
        ) -> Result<(), ExecutorError<Self::Error>> {
            todo!()
        }
    }

    struct Stableswap;

    impl Executor<Accountid, AssetId, Balance> for Stableswap {
        type Output = u8;
        type Error = ();

        fn calculate_sell(
            pool_type: PoolType<AssetId>,
            asset_in: AssetId,
            asset_out: AssetId,
            amount_in: Balance,
        ) -> Result<Self::Output, ExecutorError<Self::Error>> {
            if !matches!(pool_type, PoolType::Stableswap(_)) {
                return Err(ExecutorError::NotSupported);
            }
            Ok(1u8)
        }

        fn calculate_buy(
            pool_type: PoolType<AssetId>,
            asset_in: AssetId,
            asset_out: AssetId,
            amount_out: Balance,
        ) -> Result<Self::Output, ExecutorError<Self::Error>> {
            todo!()
        }

        fn execute_sell(
            pool_type: PoolType<AssetId>,
            who: &Accountid,
            asset_in: AssetId,
            asset_out: AssetId,
            amount_in: Balance,
        ) -> Result<(), ExecutorError<Self::Error>> {
            todo!()
        }

        fn execute_buy(
            pool_type: PoolType<AssetId>,
            who: &Accountid,
            asset_in: AssetId,
            asset_out: AssetId,
            amount_out: Balance,
        ) -> Result<(), ExecutorError<Self::Error>> {
            todo!()
        }
    }

    type Pools = (XYK, Stableswap);

    #[test]
    fn test_it() {
        assert_eq!(Pools::calculate_sell(PoolType::XYK, 0u32, 1u32, 100u128).unwrap(), 5u8);
        assert_eq!(
            Pools::calculate_sell(PoolType::XYK, 0u32, 1u32, 10u128),
            Err(ExecutorError::Error(()))
        );

        assert_eq!(
            Pools::calculate_sell(PoolType::Stableswap(10u32), 0u32, 1u32, 100u128).unwrap(),
            1u8
        );
        assert!(Pools::calculate_sell(PoolType::Omnipool, 0u32, 1u32, 100u128).is_err());
    }
}
