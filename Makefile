all: hex

hex:
	cargo objcopy -p cyberpixie-firmware --release -- -O ihex target/firmware.hex

bin:
	cargo objcopy -p cyberpixie-firmware --release -- -O binary target/firmware.bin

flash: hex
	stm32flash -w target/firmware.hex -v -g 0x0 /dev/ttyUSB0

run: flash
	serial-monitor

run_retransmitter: 
	cargo objcopy -p cyberpixie-firmware --release --example retransmitter -- -O ihex target/retransmitter.hex
	stm32flash -w target/retransmitter.hex -v -g 0x0 /dev/ttyUSB0
	serial-monitor

run_softap: 
	cargo objcopy -p cyberpixie-firmware --release --example softap -- -O ihex target/softap.hex
	stm32flash -w target/softap.hex -v -g 0x0 /dev/ttyUSB0
	serial-monitor	

clean:
	rm -rf target
