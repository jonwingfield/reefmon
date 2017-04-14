##Notes:

Had to lower the i2c baudrate to 10000 on the raspberry pi due to clock stretching issues. Added a file to /etc/modeprod.d/i2c.conf with "options i2c_bcm2708 baudrate=10000" in it.

Need to add a CRC to the transmissions in case stuff goes haywire.
