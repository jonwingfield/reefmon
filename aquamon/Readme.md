##Installation

1. Run deploy.sh
2. copy aquamon.service to /etc/systemd/system/
3. Run `sudo systemctl enable aquamon.service`
4. Run deploy.sh again to make sure everything worked
5. Web server should be running on port 80

##Notes:

Had to lower the i2c baudrate to 10000 on the raspberry pi due to clock stretching issues. Added a file to /etc/modprobe.d/i2c.conf with "options i2c_bcm2708 baudrate=10000" in it.

## Email Setup

See https://stackoverflow.com/questions/37375712/cross-compile-rust-openssl-for-raspberry-pi-2 for info on cross compiling openssl (needed for email).

On pi:

1. `sudo apt-get install ssmtp mailutils mpack`
2. Copy ssmtp.conf to the pi at `/etc/ssmtp/ssmtp.conf`

