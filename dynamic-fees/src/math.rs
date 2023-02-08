use crate::{Balance, Fee};
use sp_runtime::traits::{Saturating, Zero};
use sp_runtime::{FixedPointNumber, FixedU128};

pub struct AssetVolume {
    pub amount_in: Balance,
    pub amount_out: Balance,
    pub liquidity: Balance,
}

pub struct FeeParams {
    pub(crate) max_fee: Fee,
    pub(crate) min_fee: Fee,
    pub(crate) decay: FixedU128,
    pub(crate) amplification: FixedU128,
}

pub fn recalculate_asset_fee(volume: AssetVolume, previous_fee: Fee, last_block_diff: u128, params: FeeParams) -> Fee {
    let decaying = params
        .decay
        .saturating_mul(FixedU128::from(last_block_diff.saturating_sub(1)));
    let fee = FixedU128::from(previous_fee);
    let s = Fee::from_rational(fee.saturating_sub(decaying).into_inner(), FixedU128::DIV);
    let previous_fee = s.max(params.min_fee);

    let v_o = volume.amount_out;
    let v_i = volume.amount_in;
    let liquidity = volume.liquidity;
    // x = (V0 - Vi) / L
    let (x, x_neg) = if liquidity != Balance::zero() {
        (FixedU128::from_rational(v_o.abs_diff(v_i), liquidity), v_o < v_i)
    } else {
        (FixedU128::zero(), false)
    };

    let a_x = params.amplification.saturating_mul(x);

    let (delta_f, neg) = if x_neg {
        (a_x.saturating_add(params.decay), true)
    } else if a_x > params.decay {
        (a_x.saturating_sub(params.decay), false)
    } else {
        (params.decay.saturating_sub(a_x), true)
    };

    let left = if neg {
        FixedU128::from(previous_fee)
            .saturating_sub(delta_f)
            .max(FixedU128::from(params.min_fee))
    } else {
        FixedU128::from(previous_fee)
            .saturating_add(delta_f)
            .max(FixedU128::from(params.min_fee))
    };

    let f_plus = left.min(FixedU128::from(params.max_fee));

    Fee::from_rational(f_plus.into_inner(), FixedU128::DIV)
}

pub fn recalculate_protocol_fee(
    volume: AssetVolume,
    previous_fee: Fee,
    last_block_diff: u128,
    params: FeeParams,
) -> Fee {
    let decaying = params
        .decay
        .saturating_mul(FixedU128::from(last_block_diff.saturating_sub(1)));
    let fee = FixedU128::from(previous_fee);
    let s = Fee::from_rational(fee.saturating_sub(decaying).into_inner(), FixedU128::DIV);
    let previous_fee = s.max(params.min_fee);

    let v_o = volume.amount_out;
    let v_i = volume.amount_in;
    let liquidity = volume.liquidity;
    // x = (V0 - Vi) / L
    let (x, x_neg) = if liquidity != Balance::zero() {
        (FixedU128::from_rational(v_o.abs_diff(v_i), liquidity), v_o > v_i)
    } else {
        (FixedU128::zero(), false)
    };

    let a_x = params.amplification.saturating_mul(x);

    let (delta_f, neg) = if x_neg {
        (a_x.saturating_add(params.decay), true)
    } else if a_x > params.decay {
        (a_x.saturating_sub(params.decay), false)
    } else {
        (params.decay.saturating_sub(a_x), true)
    };

    let left = if neg {
        FixedU128::from(previous_fee)
            .saturating_sub(delta_f)
            .max(FixedU128::from(params.min_fee))
    } else {
        FixedU128::from(previous_fee)
            .saturating_add(delta_f)
            .max(FixedU128::from(params.min_fee))
    };

    let f_plus = left.min(FixedU128::from(params.max_fee));

    Fee::from_rational(f_plus.into_inner(), FixedU128::DIV)
}
