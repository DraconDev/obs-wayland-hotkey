#!/usr/bin/env python3
"""
OBS Hotkey Controller for Wayland
Uses evdev for direct keyboard input capture, which works on Wayland
"""

try:
    import websocket
    import json
    import time
    import sys
    from evdev import InputDevice, categorize, ecodes, list_devices
    from select import select
    from hotkeys import HOTKEYS, ACTION_DESCRIPTIONS
except ModuleNotFoundError as e:
    print(f"Error: {e}")
    print("\nMissing required dependencies. Please install them with:")
    print("pip install -r requirements.txt")
    print("\nOr install individual packages:")
    print("pip install websocket-client evdev")
    exit(1)

# OBS Websocket connection details
ws_url = "ws://localhost:4455"
ws_password = None
MAX_RETRIES = 10
RETRY_DELAY = 30

obs_connected = False
ws = None

# Key mapping from evdev key codes to readable names
KEY_NAMES = {
    ecodes.KEY_SCROLLLOCK: 'scroll lock',
    ecodes.KEY_PAUSE: 'pause',
    ecodes.KEY_HOME: 'home',
    ecodes.KEY_PAGEUP: 'page up',
    ecodes.KEY_PAGEDOWN: 'page down',
    ecodes.KEY_END: 'end',
    ecodes.KEY_INSERT: 'insert',
    ecodes.KEY_DELETE: 'delete',
    ecodes.KEY_F1: 'f1',
    ecodes.KEY_F2: 'f2',
    ecodes.KEY_F3: 'f3',
    ecodes.KEY_F4: 'f4',
    ecodes.KEY_F5: 'f5',
    ecodes.KEY_F6: 'f6',
    ecodes.KEY_F7: 'f7',
    ecodes.KEY_F8: 'f8',
    ecodes.KEY_F9: 'f9',
    ecodes.KEY_F10: 'f10',
    ecodes.KEY_F11: 'f11',
    ecodes.KEY_F12: 'f12',
}

# Available actions
def toggle_recording():
    print("Toggling recording...")
    send_request("ToggleRecord")

def toggle_pause():
    print("Toggling record pause...")
    send_request("ToggleRecordPause")

# Action registry
ACTIONS = {
    'toggle_recording': toggle_recording,
    'toggle_pause': toggle_pause,
}

# Connection and communication functions
def connect_to_obs():
    global ws, obs_connected
    try:
        ws = websocket.create_connection(ws_url)
        
        # Handle the initial hello message from OBS
        hello = json.loads(ws.recv())
        print(f"Connected to OBS WebSocket v{hello['d']['obsWebSocketVersion']}")
        
        # Identify ourselves to OBS (required in v5)
        identify_message = {
            "op": 1,
            "d": {
                "rpcVersion": 1
            }
        }
        ws.send(json.dumps(identify_message))
        
        # Get the identify response
        identify_response = json.loads(ws.recv())
        if identify_response["op"] == 2:
            print("Successfully identified to OBS WebSocket")
            obs_connected = True
        else:
            print(f"Failed to identify to OBS: {identify_response}")
            obs_connected = False
            
    except ConnectionRefusedError:
        print("Connection to OBS Websocket refused. Is OBS Studio running and Websocket Server enabled (port 4455, no auth)?")
        obs_connected = False
    except websocket.WebSocketException as e:
        print(f"WebSocket error: {e}")
        obs_connected = False
    except Exception as e:
        print(f"An error occurred during connection: {e}")
        obs_connected = False
    return obs_connected

def send_request(request_type, data=None):
    global ws, obs_connected
    if not obs_connected:
        print("Not connected to OBS. Reconnecting...")
        if not connect_to_obs():
            return None
    
    try:
        request = {
            "op": 6,
            "d": {
                "requestType": request_type,
                "requestId": f"{request_type}_{time.time()}",
            }
        }
        
        if data:
            request["d"].update(data)
            
        ws.send(json.dumps(request))
        response = json.loads(ws.recv())
        
        if "error" in response.get("d", {}):
            print(f"Error in {request_type} request: {response['d']['error']}")
            return None
        return response
    except Exception as e:
        print(f"Error sending {request_type} request: {e}")
        obs_connected = False
        return None

def find_keyboard_devices():
    """Find all keyboard input devices"""
    devices = []
    for path in list_devices():
        device = InputDevice(path)
        # Check if device has key capabilities
        capabilities = device.capabilities()
        if ecodes.EV_KEY in capabilities:
            devices.append(device)
    return devices

def get_key_name(keycode):
    """Convert evdev keycode to readable name"""
    return KEY_NAMES.get(keycode, None)

def main():
    print("OBS Hotkey Script Started (Wayland-compatible):")
    
    # Build hotkey map
    hotkey_map = {}
    for action_name, hotkey in HOTKEYS.items():
        if hotkey and action_name in ACTIONS:
            # Find the keycode for this hotkey
            for keycode, key_name in KEY_NAMES.items():
                if key_name == hotkey.lower():
                    hotkey_map[keycode] = ACTIONS[action_name]
                    description = ACTION_DESCRIPTIONS.get(action_name, action_name.replace('_', ' ').title())
                    print(f"- {hotkey}: {description}")
                    break
            else:
                print(f"Warning: Key '{hotkey}' not mapped in KEY_NAMES, hotkey will not work")
        elif hotkey:
            print(f"Warning: Action '{action_name}' not found, hotkey '{hotkey}' will not work")
    
    if not hotkey_map:
        print("No valid hotkeys configured. Please check hotkeys.py")
        sys.exit(1)
    
    # Find keyboard devices
    print("\nSearching for keyboard devices...")
    devices = find_keyboard_devices()
    
    if not devices:
        print("Error: No keyboard devices found!")
        print("Make sure you're running this script with sudo/root privileges.")
        sys.exit(1)
    
    print(f"Found {len(devices)} keyboard device(s):")
    for device in devices:
        print(f"  - {device.name} ({device.path})")
    
    print(f"\nConnecting to OBS Websocket at {ws_url} (no auth)...")
    
    # Try initial connection
    connect_to_obs()
    
    # If connection failed, keep trying
    retries = 0
    while not obs_connected and retries < MAX_RETRIES:
        retries += 1
        print(f"Connection attempt {retries}/{MAX_RETRIES} failed. Waiting {RETRY_DELAY} seconds before retrying...")
        time.sleep(RETRY_DELAY)
        print("Trying to connect to OBS again...")
        connect_to_obs()
    
    if not obs_connected:
        print(f"Failed to connect to OBS after {MAX_RETRIES} attempts.")
        print("Hotkeys are ready but will only work when OBS is running.")
    
    print("\nListening for hotkeys... (Press Ctrl+C to exit)")
    
    # Main event loop
    try:
        while True:
            # Wait for input from any keyboard device
            r, w, x = select(devices, [], [], 1.0)
            
            for device in r:
                try:
                    for event in device.read():
                        # Only process key press events (not release)
                        if event.type == ecodes.EV_KEY and event.value == 1:
                            if event.code in hotkey_map:
                                # Execute the associated action
                                hotkey_map[event.code]()
                except OSError as e:
                    # Device might have been disconnected
                    print(f"Error reading from {device.name}: {e}")
                    # Re-scan for devices
                    devices = find_keyboard_devices()
                    if not devices:
                        print("No keyboard devices available!")
                        break
            
            # Periodically try to reconnect to OBS if disconnected
            if not obs_connected:
                connect_to_obs()
                
    except KeyboardInterrupt:
        print("\nShutting down...")
    except Exception as e:
        print(f"Unexpected error: {e}")
    finally:
        if ws:
            ws.close()

if __name__ == "__main__":
    main()
