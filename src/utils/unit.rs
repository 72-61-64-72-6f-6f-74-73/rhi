use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MassUnitError {
    #[error("Invalid mass unit: {0}")]
    InvalidUnit(String),

    #[error("Invalid mass amount: {0}")]
    InvalidAmount(f64),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MassUnit {
    G,
    Kg,
    Oz,
    Lb,
}

impl MassUnit {
    pub fn to_grams(&self) -> f64 {
        match self {
            MassUnit::G => 1.0,
            MassUnit::Kg => 1000.0,
            MassUnit::Oz => 28.3495,
            MassUnit::Lb => 453.592,
        }
    }

    pub fn amount_in_grams(&self, amount: f64) -> Result<f64, MassUnitError> {
        if !amount.is_finite() {
            return Err(MassUnitError::InvalidAmount(amount));
        }

        let factor = match self {
            MassUnit::G => 1.0,
            MassUnit::Kg => 1000.0,
            MassUnit::Oz => 28.3495,
            MassUnit::Lb => 453.592,
        };

        Ok(amount * factor)
    }
}

impl fmt::Display for MassUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let unit_str = match self {
            MassUnit::G => "g",
            MassUnit::Kg => "kg",
            MassUnit::Oz => "oz",
            MassUnit::Lb => "lb",
        };
        write!(f, "{unit_str}")
    }
}

impl FromStr for MassUnit {
    type Err = MassUnitError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "g" => Ok(MassUnit::G),
            "kg" => Ok(MassUnit::Kg),
            "oz" => Ok(MassUnit::Oz),
            "lb" => Ok(MassUnit::Lb),
            other => Err(MassUnitError::InvalidUnit(other.to_string())),
        }
    }
}

pub fn convert_mass(amount: f64, from_unit: &MassUnit, to_unit: &MassUnit) -> f64 {
    let amount_g = amount * from_unit.to_grams();
    amount_g / to_unit.to_grams()
}
