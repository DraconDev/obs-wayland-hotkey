# Using OBS-Hotkey on Linux

This guide provides detailed instructions for using OBS-Hotkey specifically on Linux systems.

## Initial Setup

1. Make the run script executable (if not already):
   ```bash
   chmod +x run.sh
   ```

2. Create your Python virtual environment:
   ```bash
   python3 -m venv venv
   source venv/bin/activate
   pip install -r requirements.txt
   ```

3. Configure OBS Studio WebSocket:
   - Open OBS Studio
   - Go to Tools → WebSocket Server Settings
   - Check "Enable WebSocket server"
   - Leave the default port (4455)
   - Disable authentication (or configure it and update the script if needed)

## Running OBS-Hotkey

Basic usage:
```bash
./run.sh
```

This will start the script with sudo permissions, which are required for global keyboard monitoring on Linux.

## Autostarting with OBS

To automatically start OBS-Hotkey when you launch OBS:

### Method 1: Desktop entry

1. Create a desktop entry file:
   ```bash
   nano ~/.local/share/applications/obs-hotkey.desktop
   ```

2. Add the following content:
   ```
   [Desktop Entry]
   Type=Application
   Name=OBS with Hotkeys
   Comment=Launch OBS with global hotkeys
   Exec=bash -c "cd /home/dracon/_Dev/obs-hotkey && ./run.sh"
   Icon=obs
   Terminal=true
   Categories=AudioVideo;Recorder;
   ```

3. Make it executable:
   ```bash
   chmod +x ~/.local/share/applications/obs-hotkey.desktop
   ```

4. Use this launcher instead of your regular OBS launcher

### Method 2: Systemd user service

1. Create a systemd service file:
   ```bash
   mkdir -p ~/.config/systemd/user/
   nano ~/.config/systemd/user/obs-hotkey.service
   ```

2. Add the following content:
   ```
   [Unit]
   Description=OBS Studio Hotkeys
   After=graphical-session.target
   PartOf=graphical-session.target

   [Service]
   ExecStart=/home/dracon/_Dev/obs-hotkey/run.sh
   Restart=on-failure

   [Install]
   WantedBy=graphical-session.target
   ```

3. Enable and start the service:
   ```bash
   systemctl --user daemon-reload
   systemctl --user enable obs-hotkey.service
   ```

4. To start it manually:
   ```bash
   systemctl --user start obs-hotkey.service
   ```

## Common Linux Issues

### 1. Sudo password prompt

You might see a password prompt each time you run the script. To avoid this, you can configure sudo to allow running the script without a password:

1. Open the sudoers file:
   ```bash
   sudo visudo -f /etc/sudoers.d/obs-hotkey
   ```

2. Add the following line:
   ```
   your_username ALL=(root) NOPASSWD: /home/dracon/_Dev/obs-hotkey/venv/bin/python /home/dracon/_Dev/obs-hotkey/main.py
   ```

3. Save and close the file.

### 2. Key detection issues

Some keyboard layouts or desktop environments might have issues with certain keys:

- If a hotkey doesn't work, try mapping it to a different key in `hotkeys.py`
- Media keys and some function keys might be captured by your desktop environment first

### 3. Wayland compatibility

If you're using Wayland instead of X11, you might experience issues with keyboard capture. In this case:

1. Try using X11 instead of Wayland for your session
2. Or check if your distribution has any specific packages that enable global keyboard shortcuts under Wayland

## Viewing Logs

If you're running into issues, you can redirect the output to a log file:

```bash
./run.sh > obs-hotkey.log 2>&1
```

Then check the log file for any error messages:

```bash
cat obs-hotkey.log
```

## Stopping the Script

To stop the script, press Ctrl+C in the terminal where it's running, or kill the process:

```bash
pkill -f "python.*main.py"
```
