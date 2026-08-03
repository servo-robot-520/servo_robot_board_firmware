//! HUSB238A initialization helper for the charge subsystem.

/// Initialize HUSB238A USB PD sink controller.
///
/// Unmasks interrupt sources so the device can report attach/detach and faults.
pub fn init_husb238a<I2C, E>(i2c: &mut I2C)
where
    I2C: embedded_hal::i2c::I2c<Error = E>,
{
    use embedded_husb238a::Husb238a;
    let mut husb = Husb238a::new(i2c);
    match husb.init() {
        Ok(()) => defmt::info!("HUSB238A initialized"),
        Err(_e) => defmt::warn!("HUSB238A init failed"),
    }
}
