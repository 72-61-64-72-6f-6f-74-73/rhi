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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum MassUnit {
    G,
    Kg,
    Lb,
}

impl MassUnit {
    pub fn to_grams(&self, amount: f64) -> Result<f64, MassUnitError> {
        if !amount.is_finite() || amount.is_nan() {
            return Err(MassUnitError::InvalidAmount(amount));
        }

        let grams = match self {
            MassUnit::G => amount,
            MassUnit::Kg => amount * 1000.0,
            MassUnit::Lb => amount * 453.592,
        };

        Ok(grams)
    }
}

impl fmt::Display for MassUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let unit_str = match self {
            MassUnit::G => "g",
            MassUnit::Kg => "kg",
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
            "lb" => Ok(MassUnit::Lb),
            other => Err(MassUnitError::InvalidUnit(other.to_string())),
        }
    }
}
