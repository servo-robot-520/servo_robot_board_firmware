//! Startup sound presentation.
//!
//! This module describes the product-facing boot melody. It deliberately does
//! not know about GPIO pins or timer registers; those details stay in
//! [`crate::platform::buzzer`].

use crate::platform::buzzer::Buzzer;

/// Play the board's R2-D2-style startup melody.
pub fn play(buzzer: &mut Buzzer) {
    // Quick rising tone.
    let mut frequency_hz = 800;
    while frequency_hz <= 3_000 {
        buzzer.tone(frequency_hz, 12);
        frequency_hz += 200;
    }

    // Three short high beeps.
    buzzer.tone(3_500, 30);
    buzzer.tone(0, 15);
    buzzer.tone(4_000, 30);
    buzzer.tone(0, 15);
    buzzer.tone(3_200, 30);
    buzzer.tone(0, 20);

    // Descending sweep.
    let mut frequency_hz = 2_500;
    while frequency_hz >= 600 {
        buzzer.tone(frequency_hz, 10);
        frequency_hz = frequency_hz.saturating_sub(150);
    }

    // Two cheerful rising phrases.
    buzzer.tone(0, 30);
    let mut frequency_hz = 1_000;
    while frequency_hz <= 2_800 {
        buzzer.tone(frequency_hz, 15);
        frequency_hz += 300;
    }
    buzzer.tone(0, 20);

    let mut frequency_hz = 1_200;
    while frequency_hz <= 3_500 {
        buzzer.tone(frequency_hz, 15);
        frequency_hz += 300;
    }

    // Closing tone.
    buzzer.tone(0, 20);
    buzzer.tone(2_500, 60);
    buzzer.tone(3_000, 80);
    buzzer.stop();
}
