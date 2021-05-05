all: hex

hex:
	cargo objcopy -p aurora-led-firmware --release -- -O ihex target/firmware.hex

bin:
	cargo objcopy -p aurora-led-firmware --release -- -O binary target/firmware.bin

flash: hex
	stm32flash -w target/firmware.hex -v -g 0x0 /dev/ttyUSB0

run: flash
	serial-monitor -b 115200 --enter crlf

run_retransmitter: 
	cargo objcopy -p aurora-led-firmware --release --example retransmitter -- -O ihex target/retransmitter.hex
	stm32flash -w target/retransmitter.hex -v -g 0x0 /dev/ttyUSB0
	serial-monitor -b 115200 --enter crlf

clean:
	rm -rf target
