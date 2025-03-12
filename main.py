try:
    import websocket
    import json
    import keyboard
    import time
    from hotkeys import HOTKEYS, ACTION_DESCRIPTIONS
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
MAX_RETRIES = 10  # Maximum number of connection attempts
RETRY_DELAY = 30  # Seconds between retries

obs_connected = False
ws = None

# Available actions
def toggle_recording():
    print("Toggling recording...")
    send_request("ToggleRecord")

def toggle_pause():
    print("Toggling record pause...")
    send_request("ToggleRecordPause")

# Action registry - maps action names to functions
ACTIONS = {
    'toggle_recording': toggle_recording,
    'toggle_pause': toggle_pause,
    # Add more actions here as needed
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

# Register all configured hotkeys
print("OBS Hotkey Script Started:")
for action_name, hotkey in HOTKEYS.items():
    if hotkey and action_name in ACTIONS:  # Only register non-empty hotkeys
        keyboard.add_hotkey(hotkey, ACTIONS[action_name])
        description = ACTION_DESCRIPTIONS.get(action_name, action_name.replace('_', ' ').title())
        print(f"- {hotkey}: {description}")
    elif hotkey:  # Hotkey defined but action not available
        print(f"Warning: Action '{action_name}' not found, hotkey '{hotkey}' will not work")

print(f"Connecting to OBS Websocket at {ws_url} (no auth)...")

# Try initial connection
connect_to_obs()

# If connection failed, keep trying until MAX_RETRIES is reached
retries = 0
while not obs_connected and retries < MAX_RETRIES:
    retries += 1
    print(f"Connection attempt {retries}/{MAX_RETRIES} failed. Waiting {RETRY_DELAY} seconds before retrying...")
    time.sleep(RETRY_DELAY)
    print("Trying to connect to OBS again...")
    connect_to_obs()

if obs_connected:
    print("Listening for hotkeys...")
    keyboard.wait()  # Keep the script running and listening for hotkeys
else:
    print(f"Failed to connect to OBS after {MAX_RETRIES} attempts.")
    print("Hotkeys are ready but will only work when OBS is running.")
    print("This script will keep running and try to connect when possible.")
    
    # Instead of exiting, keep running and periodically try to reconnect
    while True:
        time.sleep(60)  # Check every minute
        print("Attempting to connect to OBS...")
        if connect_to_obs():
            print("Successfully connected! Hotkeys are now working.")
            keyboard.wait()  # If connection breaks, this will end and loop continues