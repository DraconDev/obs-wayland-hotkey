try:
    import websocket
    import json
    import keyboard
    import time
except ModuleNotFoundError as e:
    print(f"Error: {e}")
    print("\nMissing required dependencies. Please install them with:")
    print("pip install -r requirements.txt")
    print("\nOr install individual packages:")
    print("pip install websocket-client keyboard")
    exit(1)

# OBS Websocket connection details (adjust if needed)
ws_url = "ws://localhost:4455"  # Port 4455, no authentication
ws_password = None  # No password

# Hotkey definitions (you can change these)
toggle_recording_hotkey = "insert"
toggle_pause_hotkey = "pause"

obs_connected = False
ws = None

def connect_to_obs():
    global ws, obs_connected
    try:
        ws = websocket.create_connection(ws_url)
        
        # Handle the initial hello message from OBS
        hello = json.loads(ws.recv())
        print(f"Connected to OBS WebSocket v{hello['d']['obsWebSocketVersion']}")
        
        # Identify ourselves to OBS (required in v5)
        identify_message = {
            "op": 1,  # Identify operation
            "d": {
                "rpcVersion": 1
                # No authentication for now
            }
        }
        ws.send(json.dumps(identify_message))
        
        # Get the identify response
        identify_response = json.loads(ws.recv())
        if identify_response["op"] == 2:  # Identified
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
            "op": 6,  # RequestBatchOperation
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

def toggle_recording():
    print("Toggling recording...")
    send_request("ToggleRecord")

def toggle_pause():
    print("Toggling record pause...")
    send_request("ToggleRecordPause")

# Hotkey assignments
keyboard.add_hotkey(toggle_recording_hotkey, toggle_recording)
keyboard.add_hotkey(toggle_pause_hotkey, toggle_pause)

print(f"OBS Hotkey Script Started:")
print(f"- {toggle_recording_hotkey}: Toggle Recording")
print(f"- {toggle_pause_hotkey}: Toggle Pause/Resume Recording")
print(f"Connecting to OBS Websocket at {ws_url} (no auth)...")

connect_to_obs()  # Initial connection attempt

if obs_connected:
    print("Listening for hotkeys...")
    keyboard.wait()  # Keep the script running and listening for hotkeys
else:
    print("Failed to connect to OBS on startup. Please ensure OBS is running and Websocket Server is enabled with correct settings.")
    print("Script will exit.")