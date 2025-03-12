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

def connect_to_obs():
    global ws, obs_connected
    try:
        ws = websocket.create_connection(ws_url)
        print("Connected to OBS Websocket!")
        obs_connected = True

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


def get_record_status():
    global obs_connected
    if not obs_connected:
        print("Not connected to OBS. Reconnecting...")
        if not connect_to_obs():
            return None  # Indicate failure to get status

    try:
        # Proper message format for OBS WebSocket v5
        get_status_request = {
            "op": 6,  # RequestBatchOperation
            "d": {
                "requests": [{
                    "requestType": "GetRecordStatus",
                    "requestId": "get_record_status"
                }]
            }
        }
        
        ws.send(json.dumps(get_status_request))
        response = json.loads(ws.recv())
        
        # Debug the response
        print(f"Debug - Response received: {response}")
        
        if "d" in response and "responseData" in response["d"]:
            return response["d"]["responseData"]
        else:
            print(f"Unexpected response format: {response}")
            return None
    except Exception as e:
        print(f"Error during GetRecordStatus request: {e}")
        obs_connected = False  # Assume connection issue
        return None

def toggle_recording():
    global obs_connected
    status = get_record_status()
    if status:
        if status.get("outputActive", False):
            print("Stopping Recording...")
            action_request = {
                "op": 6,  # RequestBatchOperation
                "d": {
                    "requests": [{
                        "requestType": "StopRecording",
                        "requestId": "stop_recording"
                    }]
                }
            }
        else:
            print("Starting Recording...")
            action_request = {
                "op": 6,  # RequestBatchOperation
                "d": {
                    "requests": [{
                        "requestType": "StartRecording",
                        "requestId": "start_recording"
                    }]
                }
            }

        if obs_connected:  # Double check connection before sending action
            try:
                ws.send(json.dumps(action_request))
                action_response = json.loads(ws.recv())
                if "error" in action_response.get("d", {}):
                    print(f"Error toggling recording: {action_response['d']['error']}")
            except Exception as e:
                print(f"Error sending recording action: {e}")
                obs_connected = False
        else:
            print("OBS connection lost, cannot toggle recording.")

def toggle_pause():
    global obs_connected
    status = get_record_status()
    if status and status.get("outputActive", False):  # Only toggle pause if recording is active
        if status.get("outputPaused", False):
            print("Resuming Recording...")
            action_request = {
                "op": 6,  # RequestBatchOperation
                "d": {
                    "requests": [{
                        "requestType": "ResumeRecording",
                        "requestId": "resume_recording"
                    }]
                }
            }
        else:
            print("Pausing Recording...")
            action_request = {
                "op": 6,  # RequestBatchOperation
                "d": {
                    "requests": [{
                        "requestType": "PauseRecording",
                        "requestId": "pause_recording"
                    }]
                }
            }

        if obs_connected:  # Double check connection before sending action
            try:
                ws.send(json.dumps(action_request))
                action_response = json.loads(ws.recv())
                if "error" in action_response.get("d", {}):
                    print(f"Error toggling pause: {action_response['d']['error']}")
            except Exception as e:
                print(f"Error sending pause action: {e}")
                obs_connected = False
        else:
            print("OBS connection lost, cannot toggle pause.")
    elif status and not status.get("outputActive", False):
        print("Recording is not active, cannot toggle pause.")
    else:
        print("Could not get recording status, cannot toggle pause.")


# Hotkey assignments
keyboard.add_hotkey(toggle_recording_hotkey, toggle_recording)
keyboard.add_hotkey(toggle_pause_hotkey, toggle_pause)

print(f"OBS Hotkey Script Started:")
print(f"- {toggle_recording_hotkey}: Toggle Recording")
print(f"- {toggle_pause_hotkey}: Toggle Pause/Resume Recording")
print(f"Connecting to OBS Websocket at {ws_url} (no auth)...")

connect_to_obs() # Initial connection attempt

if obs_connected:
    print("Listening for hotkeys...")
    keyboard.wait() # Keep the script running and listening for hotkeys
else:
    print("Failed to connect to OBS on startup. Please ensure OBS is running and Websocket Server is enabled with correct settings.")
    print("Script will exit.")