##Installation

1. Run deploy.sh
2. copy aquamon.service to /etc/systemd/system/
3. Run `sudo systemcrl enable aquamon.service`
4. Run deploy.sh again to make sure everything worked
5. Web server should be running on port 80

##Notes:

Had to lower the i2c baudrate to 10000 on the raspberry pi due to clock stretching issues. Added a file to /etc/modeprod.d/i2c.conf with "options i2c_bcm2708 baudrate=10000" in it.

Need to add a CRC to the transmissions in case stuff goes haywire.
