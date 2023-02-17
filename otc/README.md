# OTC pallet
## General description
This pallet provides basic over-the-counter (OTC) trading functionality.

It allows anyone to `place_order` by specifying a pair of assets (in and out), their respective amounts, and whether the order is partially fillable. The order price is static and calculated as `amount_out / amount_in`.

Users can `fill_order` by specifying the order_id, the asset they are filling and the amount. 

The owner can `cancel_order` at any time.

## Notes
The pallet implements a minimum order size as an alternative to storage fees. The amounts of an open order cannot be lower than the existential deposit for the respective asset, multiplied by `ExistentialDepositMultiplier`. This is validated at `place_order` but also at `fill_order` - meaning that a user cannot leave dust amounts below the defined threshold after filling an order (instead they should fill the order completely).

## Dispatachable functions
* `place_order` -  create a new OTC order.
* `fill_order` - fill an OTC order (partially or completely) by providing some amount of order.asset_in.
* `cancel_order` - cancel an open OTC order.
