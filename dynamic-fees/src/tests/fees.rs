use crate::tests::mock::*;
use crate::UpdateAndRetrieveAssetFee;
use orml_traits::GetByKey;
use sp_runtime::Permill;

#[test]
pub fn asset_fee_should_be_update_correctly_when_volume_is_increasing() {
    ExtBuilder::default().build().execute_with(|| {
        PAIRS.with(|v| {
            v.borrow_mut().push((HDX, 1));
        });

        dbg!(MinimumFee::get());
        dbg!(MaximumFee::get());

        crate::AssetFee::<Test>::insert(HDX, (Permill::from_float(0.03), 0));
        System::set_block_number(1);

        for block in (1..=200).step_by(1) {
            let fee = <UpdateAndRetrieveAssetFee<Test> as GetByKey<(AssetId, AssetId), Permill>>::get(&(HDX, LRNA));
            dbg!(fee);
            System::set_block_number(block);
            BLOCK.with(|v| *v.borrow_mut() = block as usize);
        }
    })
}
