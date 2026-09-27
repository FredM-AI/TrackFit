use std::iter::Sum;
use std::ops::{Add, Sub};

use serde::{Deserialize, Serialize};

/// Montant en euros stocke en centimes (R-MONEY : jamais de `f64` pour l'argent).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct Money(i64);

impl Money {
    pub const ZERO: Money = Money(0);

    #[must_use]
    pub fn from_cents(cents: i64) -> Self {
        Self(cents)
    }

    #[must_use]
    pub fn cents(self) -> i64 {
        self.0
    }
}

impl Add for Money {
    type Output = Money;
    fn add(self, rhs: Self) -> Self::Output {
        Money(self.0 + rhs.0)
    }
}

impl Sub for Money {
    type Output = Money;
    fn sub(self, rhs: Self) -> Self::Output {
        Money(self.0 - rhs.0)
    }
}

impl Sum for Money {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Money::ZERO, Add::add)
    }
}

/// Quantite de jetons de tournoi (jamais de l'argent reel).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct Chips(i64);

impl Chips {
    pub const ZERO: Chips = Chips(0);

    #[must_use]
    pub fn from_i64(amount: i64) -> Self {
        Self(amount)
    }

    #[must_use]
    pub fn amount(self) -> i64 {
        self.0
    }
}

impl Add for Chips {
    type Output = Chips;
    fn add(self, rhs: Self) -> Self::Output {
        Chips(self.0 + rhs.0)
    }
}

impl Sub for Chips {
    type Output = Chips;
    fn sub(self, rhs: Self) -> Self::Output {
        Chips(self.0 - rhs.0)
    }
}

impl Sum for Chips {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Chips::ZERO, Add::add)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_adds_and_subtracts_in_cents() {
        let a = Money::from_cents(180);
        let b = Money::from_cents(20);
        assert_eq!((a + b).cents(), 200);
        assert_eq!((a - b).cents(), 160);
    }

    #[test]
    fn money_sums_an_iterator() {
        let total: Money = [
            Money::from_cents(80),
            Money::from_cents(100),
            Money::from_cents(20),
        ]
        .into_iter()
        .sum();
        assert_eq!(total.cents(), 200);
    }

    #[test]
    fn chips_add_and_subtract() {
        let a = Chips::from_i64(20_000);
        let b = Chips::from_i64(5_000);
        assert_eq!((a + b).amount(), 25_000);
        assert_eq!((a - b).amount(), 15_000);
    }
}
