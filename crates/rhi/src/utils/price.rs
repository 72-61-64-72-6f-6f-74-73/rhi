use super::unit::{MassUnit, convert_mass};

pub fn calculate_total_price(
    quantity_amount: f64,
    quantity_unit: &MassUnit,
    quantity_count: u32,
    price_amount: f64,
    price_quantity_amount: f64,
    price_quantity_unit: &MassUnit,
) -> f64 {
    let total_mass = quantity_amount * quantity_count as f64;
    let total_mass_in_price_unit = convert_mass(total_mass, quantity_unit, price_quantity_unit);
    let price_per_quantity_unit = price_amount / price_quantity_amount;
    price_per_quantity_unit * total_mass_in_price_unit
}
