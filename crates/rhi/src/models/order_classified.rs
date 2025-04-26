use serde::{Deserialize, Serialize};
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderClassifiedResult {
    pub quantity: OrderClassifiedQuantity,
    pub price: OrderClassifiedPrice,
    pub discounts: Vec<OrderClassifiedDiscount>,
    pub subtotal: OrderClassifiedTotal,
    pub total: OrderClassifiedTotal,
}

#[typeshare]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderClassifiedQuantity {
    pub amount: f64,
    pub unit: String,
    pub label: String,
}

#[typeshare]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderClassifiedPrice {
    pub amount: f64,
    pub currency: String,
    pub quantity_amount: f64,
    pub quantity_unit: String,
}

#[typeshare]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderClassifiedDiscount {
    pub discount_type: String,
    pub threshold: Option<f64>,
    pub threshold_unit: Option<String>,
    pub discount_per_unit: Option<f64>,
    pub discount_unit: Option<String>,
    pub discount_percent: Option<f64>,
    pub discount_amount: f64,
    pub currency: String,
}

#[typeshare]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderClassifiedTotal {
    pub price_amount: f64,
    pub price_currency: String,
    pub quantity_amount: f64,
    pub quantity_unit: String,
}
