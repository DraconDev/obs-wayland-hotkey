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
    if not obs_connected:
        print("Not connected to OBS. Reconnecting...")
        if not connect_to_obs():
            return None  # Indicate failure to get status

    get_status_request = {
        "requestType": "GetRecordStatus",
        "requestId": "get_record_status"
    }
    try:
        ws.send(json.dumps(get_status_request))
        status_response = json.loads(ws.recv())
        if status_response["status"] == "ok":
            return status_response["responseData"]
        else:
            print(f"Error getting record status: {status_response['error']}")
            return None
    except Exception as e:
        print(f"Error during GetRecordStatus request: {e}")
        obs_connected = False # Assume connection issue
        return None

def toggle_recording():
    status = get_record_status()
    if status:
        if status["outputActive"]:
            print("Stopping Recording...")
            action_request = {"requestType": "StopRecording", "requestId": "stop_recording"}
        else:
            print("Starting Recording...")
            action_request = {"requestType": "StartRecording", "requestId": "start_recording"}

        if obs_connected: # Double check connection before sending action
            try:
                ws.send(json.dumps(action_request))
                action_response = json.loads(ws.recv())
                if action_response["status"] != "ok":
                    print(f"Error toggling recording: {action_response['error']}")
            except Exception as e:
                print(f"Error sending recording action: {e}")
                obs_connected = False
        else:
            print("OBS connection lost, cannot toggle recording.")


def toggle_pause():
    status = get_record_status()
    if status and status["outputActive"]: # Only toggle pause if recording is active
        if status["outputPaused"]:
            print("Resuming Recording...")
            action_request = {"requestType": "ResumeRecording", "requestId": "resume_recording"}
        else:
            print("Pausing Recording...")
            action_request = {"requestType": "PauseRecording", "requestId": "pause_recording"}

        if obs_connected: # Double check connection before sending action
            try:
                ws.send(json.dumps(action_request))
                action_response = json.loads(ws.recv())
                if action_response["status"] != "ok":
                    print(f"Error toggling pause: {action_response['error']}")
            except Exception as e:
                print(f"Error sending pause action: {e}")
                obs_connected = False
        else:
            print("OBS connection lost, cannot toggle pause.")
    elif status and not status["outputActive"]:
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