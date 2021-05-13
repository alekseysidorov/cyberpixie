UART ?= /dev/ttyUSB0

firmware_cmd = -p cyberpixie-firmware --release
target_arch = riscv32imac-unknown-none-elf

all: hex

hex:
	cargo objcopy --target ${target_arch} $(firmware_cmd) -- -O ihex target/firmware.hex

bin:
	cargo objcopy --target ${target_arch} $(firmware_cmd) -- -O binary target/firmware.bin

splash_bin:
	cargo objcopy --target ${target_arch} $(firmware_cmd) --example splash -- -O binary target/splash.bin

softap_bin:
	cargo objcopy --target ${target_arch} $(firmware_cmd) --example softap -- -O binary target/softap.bin

flash: hex
	stm32flash -w target/firmware.hex -v -g 0x0 $(UART)

run: flash
	serial-monitor --enter crlf

run_retransmitter: 
	cargo objcopy --target ${target_arch} $(firmware_cmd) --example retransmitter -- -O ihex target/retransmitter.hex
	stm32flash -w target/retransmitter.hex -v -g 0x0 $(UART)
	serial-monitor --enter crlf

run_softap: 
	cargo objcopy --target ${target_arch} $(firmware_cmd) --example softap -- -O ihex target/softap.hex
	stm32flash -w target/softap.hex -v -g 0x0 $(UART)
	serial-monitor --enter crlf	

run_splash: 
	cargo objcopy --target ${target_arch} $(firmware_cmd) --example splash -- -O ihex target/splash.hex
	stm32flash -w target/splash.hex -v -g 0x0 $(UART)
	serial-monitor --enter crlf	

clean:
	rm -rf target
