use criterion::{black_box, criterion_group, criterion_main, Criterion};

use sp_arithmetic::traits::One;
use sp_arithmetic::FixedPointNumber;

use pallet_price_oracle::balance_ema;
use pallet_price_oracle::price_ema;
use pallet_price_oracle::Period;
use pallet_price_oracle::Price;
use pallet_price_oracle::PriceEntry;

fn from_period(period: Period) -> (Price, Price) {
    let alpha = Price::saturating_from_rational(2u32, period.saturating_add(1));
    debug_assert!(alpha <= Price::one());
    (alpha, Price::one() - alpha)
}

fn criterion_benchmark(c: &mut Criterion) {
    const PERIOD: u32 = 7_200;
    let (start_price, start_volume, start_liquidity) = (
        Price::saturating_from_integer(1_000_000_000_000_000u64),
        1_000_000_000_000_000u64.into(),
        1_000_000_000_000_000u64.into(),
    );
    let start_oracle = PriceEntry {
        price: start_price,
        volume: start_volume,
        liquidity: start_liquidity,
        timestamp: 1,
    };
    let (next_price, next_volume, next_liquidity) = (
        Price::saturating_from_integer(1_000_000_000_000_000_000u64),
        1_000_000_000_000_000_000u64.into(),
        1_000_000_000_000_000_000u64.into(),
    );
    let next_value = PriceEntry {
        price: next_price,
        volume: next_volume,
        liquidity: next_liquidity,
        timestamp: 1_000_000,
    };

    let mut next_oracle = None;
    c.bench_function("calculate_new_ema_entry", |b| {
        b.iter(|| {
            next_oracle = black_box(next_value).calculate_new_ema_entry(PERIOD, &black_box(start_oracle));
        })
    });

    // assert_eq!(next_oracle.unwrap().price, next_value.price);

    let mut next_volume = None;
    let (alpha, complement) = from_period(PERIOD);
    c.bench_function("balance_ema", |b| {
        b.iter(|| {
            next_volume = balance_ema(
                black_box(start_oracle.volume),
                black_box(complement),
                black_box(next_value.volume),
                black_box(alpha),
            );
        })
    });

    assert!(next_volume.is_some());

    let mut next_price = None;
    let (alpha, complement) = from_period(PERIOD);
    c.bench_function("price_ema", |b| {
        b.iter(|| {
            next_price = price_ema(
                black_box(start_oracle.price),
                black_box(complement),
                black_box(next_value.price),
                black_box(alpha),
            );
        })
    });

    assert!(next_price.is_some());
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
