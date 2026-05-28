# deploy/

Canonical copies of the production systemd unit files for this project.
The live system unit at `/etc/systemd/system/<name>.service` should be a
symlink to the file here so the repo is the source of truth.

## One-time setup

```bash
# As root:
sudo systemctl stop <name>.service
sudo rm /etc/systemd/system/<name>.service
sudo ln -s "/home/el/Documents/El-Projects/<repo>/deploy/<name>.service" \
           /etc/systemd/system/<name>.service
sudo systemctl daemon-reload
sudo systemctl enable --now <name>.service
sudo systemctl status <name>.service
```

After this, editing the unit file in the repo + `sudo systemctl daemon-reload`
+ `sudo systemctl restart <name>` is all you need.
