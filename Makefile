all: hex

hex:
	cargo objcopy --release -- -O ihex target/firmware.hex

flash: hex
	stm32flash -w target/firmware.hex -v -g 0x0 /dev/ttyUSB0

run: flash
	serial-monitor -b 115200 --enter crlf

clean:
	rm -rf target
