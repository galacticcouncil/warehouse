use crate::math::NetVolumeDirection::*;
use crate::Balance;
use sp_runtime::traits::{Saturating, Zero};
use sp_runtime::{FixedPointNumber, FixedPointOperand, FixedU128, PerThing};

pub struct AssetVolume {
    pub amount_in: Balance,
    pub amount_out: Balance,
    pub liquidity: Balance,
}

impl AssetVolume {
    fn net_volume(&self, direction: NetVolumeDirection) -> (Balance, bool) {
        match direction {
            OutIn => (
                self.amount_out.abs_diff(self.amount_in),
                self.amount_out < self.amount_in,
            ),
            InOut => (
                self.amount_out.abs_diff(self.amount_in),
                self.amount_out > self.amount_in,
            ),
        }
    }
}

pub struct FeeParams<Fee> {
    pub(crate) max_fee: Fee,
    pub(crate) min_fee: Fee,
    pub(crate) decay: FixedU128,
    pub(crate) amplification: FixedU128,
}

#[derive(PartialEq, Eq, Debug)]
enum NetVolumeDirection {
    OutIn,
    InOut,
}

fn recalculate_fee<Fee: PerThing>(
    volume: AssetVolume,
    previous_fee: Fee,
    last_block_diff: u128,
    params: FeeParams<Fee>,
    direction: NetVolumeDirection,
) -> Fee
where
    <Fee as PerThing>::Inner: FixedPointOperand,
{
    // Adjust previous fee which may not have been calculated in previous block
    let previous_fee = if last_block_diff > 1 {
        let decaying = params
            .decay
            .saturating_mul(FixedU128::from(last_block_diff.saturating_sub(1)));
        let fee = FixedU128::from(previous_fee);
        Fee::from_rational(fee.saturating_sub(decaying).into_inner(), FixedU128::DIV).max(params.min_fee)
    } else {
        previous_fee
    };

    // x = (V0 - Vi) / L
    let (x, x_neg) = if volume.liquidity != Balance::zero() {
        let (diff, neg) = volume.net_volume(direction);
        (FixedU128::from_rational(diff, volume.liquidity), neg)
    } else {
        (FixedU128::zero(), false)
    };

    let a_x = params.amplification.saturating_mul(x);

    // Work out fee adjustment taking into account possible negative result
    let (delta_f, neg) = if x_neg {
        (a_x.saturating_add(params.decay), true)
    } else if a_x > params.decay {
        (a_x.saturating_sub(params.decay), false)
    } else {
        (params.decay.saturating_sub(a_x), true)
    };

    let fee_plus = if neg {
        FixedU128::from(previous_fee)
            .saturating_sub(delta_f)
            .clamp(FixedU128::from(params.min_fee), FixedU128::from(params.max_fee))
    } else {
        FixedU128::from(previous_fee)
            .saturating_add(delta_f)
            .clamp(FixedU128::from(params.min_fee), FixedU128::from(params.max_fee))
    };

    Fee::from_rational(fee_plus.into_inner(), FixedU128::DIV)
}

pub fn recalculate_asset_fee<Fee: PerThing>(
    volume: AssetVolume,
    previous_fee: Fee,
    last_block_diff: u128,
    params: FeeParams<Fee>,
) -> Fee
where
    <Fee as PerThing>::Inner: FixedPointOperand,
{
    recalculate_fee(volume, previous_fee, last_block_diff, params, OutIn)
}

pub fn recalculate_protocol_fee<Fee: PerThing>(
    volume: AssetVolume,
    previous_fee: Fee,
    last_block_diff: u128,
    params: FeeParams<Fee>,
) -> Fee
where
    <Fee as PerThing>::Inner: FixedPointOperand,
{
    recalculate_fee(volume, previous_fee, last_block_diff, params, InOut)
}
