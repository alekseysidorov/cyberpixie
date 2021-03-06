CYBERPIXIE_UART ?= /dev/ttyUSB0
CYBERPIXIE_FEATURES ?= ""

firmware_cmd = -p cyberpixie-firmware --release --features $(CYBERPIXIE_FEATURES)
target_arch = riscv32imac-unknown-none-elf
serial_monitor = serial-monitor --enter crlf -b 921600 # 460800 921600
stm32flash = stm32flash -b 115200

all: hex

hex:
	cargo objcopy --target ${target_arch} $(firmware_cmd) -- -O ihex target/firmware.hex

bin:
	cargo objcopy --target ${target_arch} $(firmware_cmd) -- -O binary target/firmware.bin

splash_bin:
	cargo objcopy --target ${target_arch} $(firmware_cmd) --example splash -- -O binary target/splash.bin

softap_bin:
	cargo objcopy --target ${target_arch} $(firmware_cmd) --example softap -- -O binary target/softap.bin

run: 
	cargo objcopy --target ${target_arch} $(firmware_cmd) -- -O ihex target/firmware.hex
	$(stm32flash) -w target/firmware.hex -v -g 0x0 $(CYBERPIXIE_UART)
	$(serial_monitor)

run_retransmitter: 
	cargo objcopy --target ${target_arch} $(firmware_cmd) --example retransmitter -- -O ihex target/retransmitter.hex
	$(stm32flash) -w target/retransmitter.hex -v -g 0x0 $(CYBERPIXIE_UART)
	$(serial_monitor)

run_slave: 
	cargo objcopy --target ${target_arch} $(firmware_cmd) --example slave -- -O ihex target/slave.hex
	$(stm32flash) -w target/slave.hex -v -g 0x0 $(CYBERPIXIE_UART)
	$(serial_monitor)

run_splash: 
	cargo objcopy --target ${target_arch} $(firmware_cmd) --example splash -- -O ihex target/splash.hex
	$(stm32flash) -w target/splash.hex -v -g 0x0 $(CYBERPIXIE_UART)
	$(serial_monitor)

clean:
	rm -rf target
