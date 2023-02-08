use crate::Fee;
use crate::tests::mock::*;

#[test]
pub fn asset_fee_should_be_update_correctly_when_volume_is_increasing() {
    ExtBuilder::default().build().execute_with(|| {
        crate::AssetFee::<Test>::insert(HDX, (Fee::from_float(0.03), Fee::from_float(0.03), 0));
        System::set_block_number(1);

        for block in (1..=200).step_by(1) {
            let _fee = retrieve_fee_entry(HDX);
            //dbg!(fee);
            System::set_block_number(block);
            BLOCK.with(|v| *v.borrow_mut() = block as usize);
        }
    })
}
