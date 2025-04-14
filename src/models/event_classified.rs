use anyhow::Result;
use nostr::{EventId, event::Event};
use serde::{Deserialize, Serialize};

use crate::utils::{
    nostr::{
        nostr_tag_match_geohash, nostr_tag_match_l, nostr_tag_match_location,
        nostr_tag_match_summary, nostr_tag_match_title, nostr_tags_match,
    },
    unit::MassUnit,
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
            location,
            geolocation,
        })
    }
}
