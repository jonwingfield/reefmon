ssh -t pi@192.168.1.243 'sudo systemctl stop aquamon.service'
scp -r ../aquamon_server/static/ target/arm-unknown-linux-gnueabihf/release/aquamon pi@192.168.1.243:services/
ssh -t pi@192.168.1.243 'sudo systemctl start aquamon.service'

