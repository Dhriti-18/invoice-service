use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Cents(pub i64);

impl Cents {
    pub const ZERO: Cents = Cents(0);

    pub fn checked_add(self, other: Cents) -> Option<Cents> {
        self.0.checked_add(other.0).map(Cents)
    }

    pub fn checked_mul_qty(self, quantity: i64) -> Option<Cents> {
        self.0.checked_mul(quantity).map(Cents)
    }

    pub fn is_negative(self) -> bool {
        self.0 < 0
    }
}

impl fmt::Display for Cents {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{:02}", self.0 / 100, (self.0 % 100).abs())
    }
}
