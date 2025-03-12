# Running OBS-Hotkey in the Background

There are several ways to run OBS-Hotkey in the background on Linux. Here are the most common methods:

## Method 1: Using Systemd User Service (Recommended)

This method allows obs-hotkey to run in the background and start automatically with your user session.

1. Create a systemd user service file:

```bash
mkdir -p ~/.config/systemd/user/
nano ~/.config/systemd/user/obs-hotkey.service
```

2. Add the following content (adjust paths if you installed to a different location):

```
[Unit]
Description=OBS Studio Hotkey Controller
After=graphical-session.target
PartOf=graphical-session.target

[Service]
Type=simple
ExecStart=/home/dracon/.local/bin/obs-hotkey/run.sh
Restart=on-failure
RestartSec=10s

[Install]
WantedBy=graphical-session.target
```

3. Enable and start the service:

```bash
systemctl --user daemon-reload
systemctl --user enable obs-hotkey.service
systemctl --user start obs-hotkey.service
```

4. Check status:

```bash
systemctl --user status obs-hotkey.service
```

5. To stop the service:

```bash
systemctl --user stop obs-hotkey.service
```

## Method 2: Using nohup

For a quick way to run the script in the background:

```bash
cd ~/.local/bin/obs-hotkey
nohup ./run.sh > obs-hotkey.log 2>&1 &
```

This will:

- Run obs-hotkey in the background
- Save output to obs-hotkey.log
- Return control to your terminal

To check if it's running:

```bash
ps aux | grep obs-hotkey
```

To stop it:

```bash
pkill -f "obs-hotkey/run.sh"
```

## Method 3: Using Screen or Tmux

If you want to be able to check on the program later:

```bash
# Install screen if not already installed
sudo apt install screen  # For Debian/Ubuntu
# or
sudo dnf install screen  # For Fedora

# Start a new screen session
screen -S obs-hotkey

# Now run the program
cd ~/.local/bin/obs-hotkey
./run.sh

# Detach from the screen session with Ctrl+A, then press D
```

To reattach to the screen session later:

```bash
screen -r obs-hotkey
```

## Method 4: Autostart Entry

To start obs-hotkey automatically when you log in:

```bash
mkdir -p ~/.config/autostart
nano ~/.config/autostart/obs-hotkey.desktop
```

Add this content:

```
[Desktop Entry]
Type=Application
Name=OBS Hotkeys
Comment=Control OBS Studio with global hotkeys
Exec=bash -c "sleep 10 && /home/dracon/.local/bin/obs-hotkey/run.sh"
Terminal=false
Hidden=false
```

The `sleep 10` gives your desktop environment time to fully start before launching obs-hotkey.

## Handling Sudo Password

Since the script requires sudo permissions, you'll need to handle the password prompt:

### Option 1: Configure passwordless sudo for the script

1. Open the sudoers configuration:

```bash
sudo visudo -f /etc/sudoers.d/obs-hotkey
```

2. Add the following line, replacing YOUR_USERNAME with your actual username:

```
YOUR_USERNAME ALL=(root) NOPASSWD: /home/dracon/.local/bin/obs-hotkey/venv/bin/python /home/dracon/.local/bin/obs-hotkey/main.py
```

3. Save and exit.

### Option 2: Use a password manager like KeePassXC

You can set up a KeePassXC entry with auto-type to enter the password when the sudo prompt appears.

## Troubleshooting

If obs-hotkey isn't working in the background:

1. Check the output logs:

   ```bash
   # For systemd
   journalctl --user -u obs-hotkey.service

   # For nohup
   cat ~/.local/bin/obs-hotkey/obs-hotkey.log
   ```

2. Verify OBS is running before starting obs-hotkey
3. Check that sudo permissions are correctly configured
