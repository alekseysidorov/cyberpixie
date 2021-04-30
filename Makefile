all: hex

hex:
	cargo objcopy --release -- -O ihex target/firmware.hex

bin:
	cargo objcopy --release -- -O binary target/firmware.bin

flash: hex
	stm32flash -w target/firmware.hex -v -g 0x0 /dev/ttyUSB0

run: flash
	serial-monitor -b 115200 --enter crlf

esprun: 
	cargo objcopy --release -p esp8266-device -- -O ihex target/esp.hex
	stm32flash -w target/esp.hex -v -g 0x0 /dev/ttyUSB0
	serial-monitor -b 115200 --enter crlf

clean:
	rm -rf target
