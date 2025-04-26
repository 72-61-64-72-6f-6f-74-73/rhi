use anyhow::Result;
use nostr::{EventId, event::Event};
use serde::{Deserialize, Serialize};

use crate::{
    handlers::job_request_order::{JobRequestOrderDataOrder, JobRequestOrderError},
    utils::{
        nostr::{
            nostr_tag_match_geohash, nostr_tag_match_l, nostr_tag_match_location,
            nostr_tag_match_summary, nostr_tag_match_title, nostr_tags_match,
        },
        unit::{MassUnit, convert_mass},
    },
};

use super::order_classified::{
    OrderClassifiedDiscount, OrderClassifiedPrice, OrderClassifiedQuantity, OrderClassifiedResult,
    OrderClassifiedTotal,
};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct EventClassifiedGeolocation {
    pub geohash: Option<String>,
    pub lat: f64,
    pub lng: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct EventClassifiedLocation {
    pub address: String,
    pub region: String,
    pub country: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum EventClassifiedDiscount {
    #[serde(rename = "subtotal")]
    Subtotal {
        threshold: f64,
        currency: String,
        value: f64,
        is_percent: bool,
    },
    #[serde(rename = "mass")]
    Mass {
        discount_unit: String,
        threshold: f64,
        threshold_unit: String,
        discount_per_unit: f64,
        currency: String,
    },
    #[serde(rename = "quantity")]
    Quantity {
        product_key: String,
        min_count: u32,
        discount_per_unit: f64,
        currency: String,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EventClassifiedQuantity {
    pub amount: f64,
    pub unit: MassUnit,
    pub label: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EventClassifiedPrice {
    pub amount: f64,
    pub currency: String,
    pub quantity_amount: f64,
    pub quantity_unit: MassUnit,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct EventClassifiedListing {
    pub key: String,
    pub category: String,
    pub process: Option<String>,
    pub lot: Option<String>,
    pub profile: Option<String>,
    pub year: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct EventClassifiedBasis {
    pub title: String,
    pub summary: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EventClassified {
    pub id: EventId,
    pub basis: EventClassifiedBasis,
    pub listing: EventClassifiedListing,
    pub prices: Vec<EventClassifiedPrice>,
    pub quantities: Vec<EventClassifiedQuantity>,
    pub discounts: Vec<EventClassifiedDiscount>,
    pub location: Option<EventClassifiedLocation>,
    pub geolocation: Option<EventClassifiedGeolocation>,
}

impl EventClassified {
    pub fn from_event(event: &Event) -> Result<Self> {
        let mut prices = Vec::new();
        let mut quantities = Vec::new();
        let mut basis = EventClassifiedBasis::default();
        let mut listing = EventClassifiedListing::default();

        let mut address: Option<String> = None;
        let mut region: Option<String> = None;
        let mut country: Option<String> = None;
        let mut lat: Option<f64> = None;
        let mut lng: Option<f64> = None;
        let mut geohash: Option<String> = None;
        let mut discounts: Vec<EventClassifiedDiscount> = Vec::new();

        for tag in event.tags.iter() {
            if let Some((key, values)) = nostr_tags_match(tag) {
                match key {
                    "quantity" if values.len() >= 3 => {
                        let amount_str = &values[0];
                        let unit_str = &values[1];
                        let label = &values[2];

                        if let (Ok(amount), Ok(unit)) =
                            (amount_str.parse::<f64>(), unit_str.parse::<MassUnit>())
                        {
                            quantities.push(EventClassifiedQuantity {
                                amount,
                                unit,
                                label: label.clone(),
                            });
                        }
                    }
                    "price" if values.len() >= 4 => {
                        let amount_str = &values[0];
                        let currency = &values[1];
                        let quantity_amount_str = &values[2];
                        let quantity_unit_str = &values[3];

                        if let (Ok(amount), Ok(quantity_amount), Ok(quantity_unit)) = (
                            amount_str.parse::<f64>(),
                            quantity_amount_str.parse::<f64>(),
                            quantity_unit_str.to_lowercase().parse::<MassUnit>(),
                        ) {
                            prices.push(EventClassifiedPrice {
                                amount,
                                currency: currency.clone(),
                                quantity_amount,
                                quantity_unit,
                            });
                        }
                    }
                    "key" if !values.is_empty() => listing.key = values[0].clone(),
                    "category" if !values.is_empty() => listing.category = values[0].clone(),
                    "process" if !values.is_empty() => listing.process = Some(values[0].clone()),
                    "lot" if !values.is_empty() => listing.lot = Some(values[0].clone()),
                    "profile" if !values.is_empty() => listing.profile = Some(values[0].clone()),
                    "year" if !values.is_empty() => listing.year = Some(values[0].clone()),
                    "price-discount-subtotal" if values.len() >= 4 => {
                        let threshold = values[0].parse().unwrap_or(0.0);
                        let currency = values[1].clone();
                        let value = values[2].parse().unwrap_or(0.0);
                        let is_percent = values[3] == "%";
                        discounts.push(EventClassifiedDiscount::Subtotal {
                            threshold,
                            currency,
                            value,
                            is_percent,
                        });
                    }
                    "price-discount-mass" if values.len() >= 5 => {
                        let discount_unit = values[0].clone();
                        let threshold = values[1].parse().unwrap_or(0.0);
                        let threshold_unit = values[2].clone();
                        let discount_per_unit = values[3].parse().unwrap_or(0.0);
                        let currency = values[4].clone();
                        discounts.push(EventClassifiedDiscount::Mass {
                            discount_unit,
                            threshold,
                            threshold_unit,
                            discount_per_unit,
                            currency,
                        });
                    }
                    "price-discount-quantity" if values.len() >= 4 => {
                        let product_key = values[0].clone();
                        let min_count = values[1].parse().unwrap_or(0);
                        let discount_per_unit = values[2].parse().unwrap_or(0.0);
                        let currency = values[3].clone();
                        discounts.push(EventClassifiedDiscount::Quantity {
                            product_key,
                            min_count,
                            discount_per_unit,
                            currency,
                        });
                    }
                    _ => {}
                }
            }

            if let Some((kind, value)) = nostr_tag_match_l(tag) {
                let precision = value.to_string().split('.').nth(1).map_or(0, |s| s.len());

                match kind {
                    "dd.lat" => {
                        let current_precision = lat
                            .map(|v| v.to_string().split('.').nth(1).map_or(0, |s| s.len()))
                            .unwrap_or(0);
                        if precision > current_precision {
                            lat = Some(value);
                        }
                    }
                    "dd.lon" => {
                        let current_precision = lng
                            .map(|v| v.to_string().split('.').nth(1).map_or(0, |s| s.len()))
                            .unwrap_or(0);
                        if precision > current_precision {
                            lng = Some(value);
                        }
                    }
                    _ => {}
                }
            }

            if let Some((addr, reg, coun)) = nostr_tag_match_location(tag) {
                address = Some(addr.to_string());
                region = Some(reg.to_string());
                country = Some(coun.to_string());
            }

            if let Some(g) = nostr_tag_match_geohash(tag) {
                if geohash
                    .as_ref()
                    .map_or(true, |current| g.len() > current.len())
                {
                    geohash = Some(g);
                }
            }

            if let Some(title) = nostr_tag_match_title(tag) {
                basis.title = title;
            }

            if let Some(summary) = nostr_tag_match_summary(tag) {
                basis.summary = summary;
            }
        }

        let location = if address.is_some() || region.is_some() || country.is_some() {
            Some(EventClassifiedLocation {
                address: address.unwrap_or_default(),
                region: region.unwrap_or_default(),
                country: country.unwrap_or_default(),
            })
        } else {
            None
        };

        let geolocation = if let (Some(lat), Some(lng)) = (lat, lng) {
            Some(EventClassifiedGeolocation { geohash, lat, lng })
        } else {
            None
        };

        Ok(Self {
            id: event.id,
            basis,
            listing,
            prices,
            quantities,
            discounts,
            location,
            geolocation,
        })
    }

    pub fn calculate_order(
        &self,
        order: &JobRequestOrderDataOrder,
    ) -> Result<OrderClassifiedResult, JobRequestOrderError> {
        let quantity = &order.quantity;
        let price = &order.price;

        let qty_unit = quantity
            .unit
            .parse::<MassUnit>()
            .map_err(|_| JobRequestOrderError::Unsatisfiable("invalid quantity unit".into()))?;
        let price_unit = price.quantity_unit.parse::<MassUnit>().map_err(|_| {
            JobRequestOrderError::Unsatisfiable("invalid price quantity unit".into())
        })?;

        let total_qty = quantity.amount * quantity.count as f64;

        let matched_packaging = self
            .quantities
            .iter()
            .any(|q| q.unit == qty_unit && (q.amount - quantity.amount).abs() < f64::EPSILON);

        if !matched_packaging {
            return Err(JobRequestOrderError::Unsatisfiable(format!(
                "requested packaging {} {} not available",
                quantity.amount, quantity.unit
            )));
        }

        let matched_tier = self.prices.iter().find(|p| {
            p.quantity_unit == price_unit
                && (p.quantity_amount - price.quantity_amount).abs() < f64::EPSILON
                && p.currency.to_lowercase() == price.currency.to_lowercase()
        });

        let tier = matched_tier.ok_or_else(|| {
            JobRequestOrderError::Unsatisfiable(format!(
                "no matching price tier {} {} found",
                price.quantity_amount, price.quantity_unit
            ))
        })?;

        if (tier.amount - price.amount).abs() > f64::EPSILON {
            return Err(JobRequestOrderError::Unsatisfiable(format!(
                "price mismatch: expected {}, got {}",
                tier.amount, price.amount
            )));
        }

        let converted_qty = convert_mass(total_qty, &qty_unit, &price_unit);
        let unit_price = tier.amount / tier.quantity_amount;
        let subtotal = (unit_price * converted_qty * 100.0).round() / 100.0;

        let mut discounts: Vec<OrderClassifiedDiscount> = Vec::new();
        let package_key = format!(
            "{}-{}-{}",
            quantity.amount,
            quantity.unit.to_lowercase(),
            quantity.label
        );

        for d in &self.discounts {
            match d {
                EventClassifiedDiscount::Subtotal {
                    threshold,
                    currency,
                    value,
                    is_percent,
                } => {
                    if subtotal < *threshold {
                        continue;
                    }
                    let amt = if *is_percent {
                        (subtotal * value / 100.0 * 100.0).round() / 100.0
                    } else {
                        (*value * 100.0).round() / 100.0
                    };
                    discounts.push(OrderClassifiedDiscount {
                        discount_type: "subtotal".into(),
                        threshold: Some(*threshold),
                        threshold_unit: None,
                        discount_per_unit: None,
                        discount_unit: None,
                        discount_percent: if *is_percent { Some(*value) } else { None },
                        discount_amount: amt,
                        currency: currency.clone(),
                    });
                }
                EventClassifiedDiscount::Mass {
                    discount_unit,
                    threshold,
                    threshold_unit,
                    discount_per_unit,
                    currency,
                } => {
                    let th_unit = threshold_unit.parse::<MassUnit>().map_err(|_| {
                        JobRequestOrderError::Unsatisfiable("invalid threshold unit".into())
                    })?;
                    let dis_unit = discount_unit.parse::<MassUnit>().map_err(|_| {
                        JobRequestOrderError::Unsatisfiable("invalid discount unit".into())
                    })?;

                    let qty_in_th = convert_mass(total_qty, &qty_unit, &th_unit);
                    if qty_in_th < *threshold {
                        continue;
                    }

                    let qty_in_dis = convert_mass(total_qty, &qty_unit, &dis_unit);
                    let amt = (qty_in_dis * discount_per_unit * 100.0).round() / 100.0;

                    discounts.push(OrderClassifiedDiscount {
                        discount_type: "mass".into(),
                        threshold: Some(*threshold),
                        threshold_unit: Some(threshold_unit.clone()),
                        discount_per_unit: Some(*discount_per_unit),
                        discount_unit: Some(discount_unit.clone()),
                        discount_percent: None,
                        discount_amount: amt,
                        currency: currency.clone(),
                    });
                }
                EventClassifiedDiscount::Quantity {
                    product_key,
                    min_count,
                    discount_per_unit,
                    currency,
                } => {
                    if product_key != &package_key || quantity.count < *min_count {
                        continue;
                    }

                    let amt = (*discount_per_unit * quantity.count as f64 * 100.0).round() / 100.0;

                    discounts.push(OrderClassifiedDiscount {
                        discount_type: "quantity".into(),
                        threshold: Some(*min_count as f64),
                        threshold_unit: None,
                        discount_per_unit: Some(*discount_per_unit),
                        discount_unit: None,
                        discount_percent: None,
                        discount_amount: amt,
                        currency: currency.clone(),
                    });
                }
            }
        }

        let total_discount: f64 = discounts.iter().map(|d| d.discount_amount).sum();
        let total = ((subtotal - total_discount) * 100.0).round() / 100.0;

        Ok(OrderClassifiedResult {
            quantity: OrderClassifiedQuantity {
                amount: quantity.amount,
                unit: quantity.unit.clone(),
                label: quantity.label.clone(),
            },
            price: OrderClassifiedPrice {
                amount: tier.amount,
                currency: tier.currency.clone(),
                quantity_amount: tier.quantity_amount,
                quantity_unit: price.quantity_unit.clone(),
            },
            discounts,
            subtotal: OrderClassifiedTotal {
                price_amount: subtotal,
                price_currency: tier.currency.clone(),
                quantity_amount: total_qty,
                quantity_unit: quantity.unit.clone(),
            },
            total: OrderClassifiedTotal {
                price_amount: total,
                price_currency: tier.currency.clone(),
                quantity_amount: total_qty,
                quantity_unit: quantity.unit.clone(),
            },
        })
    }
}
