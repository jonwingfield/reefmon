ssh -t pi@pi3b 'sudo systemctl stop aquamon.service'
scp -r ../aquamon_server/static/ target/armv7-unknown-linux-gnueabihf/debug/aquamon pi@pi3b:services/
ssh -t pi@pi3b 'sudo systemctl start aquamon.service'

