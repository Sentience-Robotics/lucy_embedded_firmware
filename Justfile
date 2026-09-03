# Build AVR firmware (nightly, build-std)
build-arduino-mega:
    cd firmwares/arduino_mega && cargo build --release -Z build-std=core --target avr-specs/atmega2560.json

# Build RP2040 firmware (stable)
build-rp2040:
    cd firmwares/rp2040 && cargo build --release --target thumbv6m-none-eabi

# Flash AVR
#flash-avr: build-avr
#    ravedude uno -cb 115200 target/avr-atmega2560/release/firmware-arduino-mega.elf

# Flash RP2040
#flash-rp2040: build-rp2040
#    elf2uf2-rs target/thumbv6m-none-eabi/release/firmware-rp2040