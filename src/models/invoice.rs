use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvoiceState {
    Draft,
    Open,
    Paid,
    Void,
    Uncollectible,
}

impl InvoiceState {
    pub fn as_str(self) -> &'static str {
        match self {
            InvoiceState::Draft => "draft",
            InvoiceState::Open => "open",
            InvoiceState::Paid => "paid",
            InvoiceState::Void => "void",
            InvoiceState::Uncollectible => "uncollectible",
        }
    }
}

impl fmt::Display for InvoiceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for InvoiceState {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "draft" => Ok(InvoiceState::Draft),
            "open" => Ok(InvoiceState::Open),
            "paid" => Ok(InvoiceState::Paid),
            "void" => Ok(InvoiceState::Void),
            "uncollectible" => Ok(InvoiceState::Uncollectible),
            other => Err(format!("unknown invoice state '{other}'")),
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct Invoice {
    pub id: Uuid,
    pub business_id: Uuid,
    pub customer_id: Uuid,
    pub total_amount_cents: i64,
    pub currency: String,
    pub state: String,
    pub due_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct InvoiceLineItem {
    pub id: Uuid,
    pub invoice_id: Uuid,
    pub position: i32,
    pub description: String,
    pub quantity: i32,
    pub unit_amount_cents: i64,
    pub amount_cents: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateInvoiceLineItemRequest {
    pub description: String,
    pub quantity: i64,
    pub unit_amount_cents: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateInvoiceRequest {
    pub customer_id: Uuid,
    pub due_date: Option<NaiveDate>,
    pub line_items: Vec<CreateInvoiceLineItemRequest>,
}
