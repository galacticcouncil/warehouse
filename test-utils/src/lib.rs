use frame_system::Config;
use pretty_assertions::assert_eq;

pub fn expect_events<TEvent: std::fmt::Debug + PartialEq, TRuntime: Config>(e: Vec<TEvent>) where Vec<TEvent>: FromIterator<<TRuntime as Config>::Event>{
    let last_events: Vec<TEvent> = last_events::<TEvent, TRuntime>(e.len());
    assert_eq!(last_events, e);
}

pub fn last_events<TEvent: std::fmt::Debug, TRuntime>(n: usize) -> Vec<TEvent>  where  TRuntime: Config, Vec<TEvent>: FromIterator<<TRuntime as Config>::Event>{
    frame_system::Pallet::<TRuntime>::events()
        .into_iter()
        .rev()
        .take(n)
        .rev()
        .map(|e| e.event)
        .collect()
}