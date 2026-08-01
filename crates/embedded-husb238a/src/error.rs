//! Error types for the HUSB238A driver.

/// Driver error
#[derive(Debug)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
pub enum Error<I2cError> {
    /// I2C communication error
    I2c(I2cError),
    /// GO command timeout
    GoTimeout,
    /// The controller reported that the requested PDO could not be selected.
    GoFailed,
    /// No charger attached
    NotAttached,
}
