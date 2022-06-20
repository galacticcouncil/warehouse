use super::*;

use crate::mock::{
    set_block_number, CollatorRewards, ExtBuilder, Test, Tokens, ALICE, BOB, CHARLIE, COLLATOR_REWARD, DAVE, GC_COLL_1,
    GC_COLL_2, GC_COLL_3, NATIVE_TOKEN,
};

use sp_runtime::testing::UintAuthorityId;

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| set_block_number(1));
    ext
}

#[test]
fn reward_collator_on_new_session_should_work() {
    new_test_ext().execute_with(|| {
        //collators which should be rewarded
        assert_eq!(Tokens::free_balance(NATIVE_TOKEN, &ALICE), 0);
        assert_eq!(Tokens::free_balance(NATIVE_TOKEN, &CHARLIE), 0);
        assert_eq!(Tokens::free_balance(NATIVE_TOKEN, &BOB), 0);
        assert_eq!(Tokens::free_balance(NATIVE_TOKEN, &DAVE), 0);

        let collator_set = vec![
            (&ALICE, UintAuthorityId(ALICE).to_public_key()),
            (&BOB, UintAuthorityId(BOB).to_public_key()),
            (&GC_COLL_1, UintAuthorityId(GC_COLL_1).to_public_key()),
            (&CHARLIE, UintAuthorityId(CHARLIE).to_public_key()),
            (&GC_COLL_2, UintAuthorityId(GC_COLL_2).to_public_key()),
            (&DAVE, UintAuthorityId(DAVE).to_public_key()),
            (&GC_COLL_3, UintAuthorityId(GC_COLL_3).to_public_key()),
        ];

        CollatorRewards::on_new_session(true, collator_set.into_iter(), vec![].into_iter());

        //excluded collators and should not be rewarded
        assert_eq!(Tokens::free_balance(NATIVE_TOKEN, &GC_COLL_1), 0);
        assert_eq!(Tokens::free_balance(NATIVE_TOKEN, &GC_COLL_2), 0);
        assert_eq!(Tokens::free_balance(NATIVE_TOKEN, &GC_COLL_3), 0);

        //this collators should be rewarded
        assert_eq!(Tokens::free_balance(NATIVE_TOKEN, &ALICE), COLLATOR_REWARD);
        assert_eq!(Tokens::free_balance(NATIVE_TOKEN, &CHARLIE), COLLATOR_REWARD);
        assert_eq!(Tokens::free_balance(NATIVE_TOKEN, &BOB), COLLATOR_REWARD);
        assert_eq!(Tokens::free_balance(NATIVE_TOKEN, &DAVE), COLLATOR_REWARD);

        frame_system::Pallet::<Test>::assert_has_event(mock::Event::CollatorRewards(Event::CollatorRewarded {
            who: ALICE,
            amount: COLLATOR_REWARD,
            currency: NATIVE_TOKEN,
        }));
        frame_system::Pallet::<Test>::assert_has_event(mock::Event::CollatorRewards(Event::CollatorRewarded {
            who: BOB,
            amount: COLLATOR_REWARD,
            currency: NATIVE_TOKEN,
        }));
        frame_system::Pallet::<Test>::assert_has_event(mock::Event::CollatorRewards(Event::CollatorRewarded {
            who: CHARLIE,
            amount: COLLATOR_REWARD,
            currency: NATIVE_TOKEN,
        }));
        frame_system::Pallet::<Test>::assert_has_event(mock::Event::CollatorRewards(Event::CollatorRewarded {
            who: DAVE,
            amount: COLLATOR_REWARD,
            currency: NATIVE_TOKEN,
        }));
    });
}

#[test]
fn reward_collator_on_end_session_should_work() {
    new_test_ext().execute_with(|| {
        //collators which should be rewarded
        assert_eq!(Tokens::free_balance(NATIVE_TOKEN, &ALICE), 0);
        assert_eq!(Tokens::free_balance(NATIVE_TOKEN, &CHARLIE), 0);
        assert_eq!(Tokens::free_balance(NATIVE_TOKEN, &BOB), 0);
        assert_eq!(Tokens::free_balance(NATIVE_TOKEN, &DAVE), 0);

        // We run a stripped down version of `fn rotate_session`
        // https://github.com/paritytech/substrate/blob/6001b59f9f64a133d55fc13a495acc76eb4b532f/frame/session/src/lib.rs#L636-L715
        let trigger_next_session = |index| {
            assert!(index > 0);
            CollatorRewards::end_session(index - 1);
            CollatorRewards::start_session(index);
            CollatorRewards::new_session(index + 1);
        };
        // We run it three times in order for the collators returned in `new_session(2)` to be rewarded
        // in `end_session(2)`.
        trigger_next_session(1);
        trigger_next_session(2);
        trigger_next_session(3);

        //excluded collators and should not be rewarded
        assert_eq!(Tokens::free_balance(NATIVE_TOKEN, &GC_COLL_1), 0);
        assert_eq!(Tokens::free_balance(NATIVE_TOKEN, &GC_COLL_2), 0);
        assert_eq!(Tokens::free_balance(NATIVE_TOKEN, &GC_COLL_3), 0);

        //this collators should be rewarded
        assert_eq!(Tokens::free_balance(NATIVE_TOKEN, &ALICE), COLLATOR_REWARD);
        assert_eq!(Tokens::free_balance(NATIVE_TOKEN, &CHARLIE), COLLATOR_REWARD);
        assert_eq!(Tokens::free_balance(NATIVE_TOKEN, &BOB), COLLATOR_REWARD);
        assert_eq!(Tokens::free_balance(NATIVE_TOKEN, &DAVE), COLLATOR_REWARD);

        frame_system::Pallet::<Test>::assert_has_event(mock::Event::CollatorRewards(Event::CollatorRewarded {
            who: ALICE,
            amount: COLLATOR_REWARD,
            currency: NATIVE_TOKEN,
        }));
        frame_system::Pallet::<Test>::assert_has_event(mock::Event::CollatorRewards(Event::CollatorRewarded {
            who: BOB,
            amount: COLLATOR_REWARD,
            currency: NATIVE_TOKEN,
        }));
        frame_system::Pallet::<Test>::assert_has_event(mock::Event::CollatorRewards(Event::CollatorRewarded {
            who: CHARLIE,
            amount: COLLATOR_REWARD,
            currency: NATIVE_TOKEN,
        }));
        frame_system::Pallet::<Test>::assert_has_event(mock::Event::CollatorRewards(Event::CollatorRewarded {
            who: DAVE,
            amount: COLLATOR_REWARD,
            currency: NATIVE_TOKEN,
        }));
    });
}
