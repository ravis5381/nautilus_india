use rust_decimal::Decimal;

use crate::model::{Position, Side};

/// Apply one fill to a net position.
///
/// Quantity is signed: long is positive, short is negative. Adding to a
/// position recalculates the average. Reducing it books realized PnL against
/// that average. A fill that crosses zero closes the old side and opens the
/// remainder at the fill price.
pub fn apply_fill(position: &mut Position, side: Side, quantity: i64, price: Decimal) {
    assert!(quantity > 0, "fill quantity must be positive");
    let signed = side.sign() * quantity;
    let old_qty = position.quantity;

    if old_qty == 0 || old_qty.signum() == signed.signum() {
        let old_abs = old_qty.unsigned_abs() as i64;
        let new_abs = old_abs + quantity;
        let notional =
            position.avg_price * Decimal::from(old_abs) + price * Decimal::from(quantity);
        position.avg_price = (notional / Decimal::from(new_abs)).round_dp(4);
        position.quantity = old_qty + signed;
        return;
    }

    let closing = quantity.min(old_qty.abs());
    let direction = Decimal::from(old_qty.signum());
    let pnl = (price - position.avg_price) * Decimal::from(closing) * direction;
    position.realized_pnl = (position.realized_pnl + pnl).round_dp(4);

    let remainder = quantity - closing;
    if remainder == 0 {
        position.quantity = old_qty + signed;
        if position.quantity == 0 {
            position.avg_price = Decimal::ZERO;
        }
    } else {
        position.quantity = signed.signum() * remainder;
        position.avg_price = price.round_dp(4);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::dec;

    fn book(fills: &[(Side, i64, Decimal)]) -> Position {
        let mut position = Position::new("core", "RELIANCE.NSE");
        for (side, quantity, price) in fills {
            apply_fill(&mut position, *side, *quantity, *price);
        }
        position
    }

    #[test]
    fn averages_adds_and_realizes_reductions() {
        let position = book(&[
            (Side::Buy, 10, dec!(100)),
            (Side::Buy, 10, dec!(120)),
            (Side::Sell, 5, dec!(130)),
        ]);

        assert_eq!(position.quantity, 15);
        assert_eq!(position.avg_price, dec!(110));
        assert_eq!(position.realized_pnl, dec!(100));
    }

    #[test]
    fn flips_through_flat_and_covers_a_short() {
        let mut position = book(&[(Side::Buy, 10, dec!(100))]);
        apply_fill(&mut position, Side::Sell, 15, dec!(90));
        assert_eq!(position.quantity, -5);
        assert_eq!(position.avg_price, dec!(90));
        assert_eq!(position.realized_pnl, dec!(-100));

        apply_fill(&mut position, Side::Buy, 5, dec!(80));
        assert_eq!(position.quantity, 0);
        assert_eq!(position.avg_price, Decimal::ZERO);
        assert_eq!(position.realized_pnl, dec!(-50));
    }
}
