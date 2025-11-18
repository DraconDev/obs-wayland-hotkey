package main

import (
	"fmt"
	"log"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/gorilla/websocket"
	"github.com/gvalkov/golang-evdev"
)

const (
	wsURL        = "ws://localhost:4455"
	maxRetries   = 10
	retryDelay   = 30 * time.Second
)

// OBS WebSocket message structures
type HelloMessage struct {
	Op int `json:"op"`
	D  struct {
		ObsWebSocketVersion string `json:"obsWebSocketVersion"`
		RpcVersion          int    `json:"rpcVersion"`
	} `json:"d"`
}

type IdentifyMessage struct {
	Op int `json:"op"`
	D  struct {
		RpcVersion int `json:"rpcVersion"`
	} `json:"d"`
}

type RequestMessage struct {
	Op int `json:"op"`
	D  struct {
		RequestType string `json:"requestType"`
		RequestID   string `json:"requestId"`
	} `json:"d"`
}

type ResponseMessage struct {
	Op int `json:"op"`
	D  struct {
		RequestID string `json:"requestId"`
	} `json:"d"`
}

// Hotkey configuration
type HotkeyConfig struct {
	ToggleRecording string
	TogglePause     string
}

var config = HotkeyConfig{
	ToggleRecording: "scroll lock",
	TogglePause:     "pause",
}

// Key code mappings
var keyNames = map[uint16]string{
	evdev.KEY_SCROLLLOCK: "scroll lock",
	evdev.KEY_PAUSE:      "pause",
	evdev.KEY_HOME:       "home",
	evdev.KEY_PAGEUP:     "page up",
	evdev.KEY_PAGEDOWN:   "page down",
	evdev.KEY_END:        "end",
	evdev.KEY_INSERT:     "insert",
	evdev.KEY_DELETE:     "delete",
	evdev.KEY_F1:         "f1",
	evdev.KEY_F2:         "f2",
	evdev.KEY_F3:         "f3",
	evdev.KEY_F4:         "f4",
	evdev.KEY_F5:         "f5",
	evdev.KEY_F6:         "f6",
	evdev.KEY_F7:         "f7",
	evdev.KEY_F8:         "f8",
	evdev.KEY_F9:         "f9",
	evdev.KEY_F10:        "f10",
	evdev.KEY_F11:        "f11",
	evdev.KEY_F12:        "f12",
}

type OBSClient struct {
	conn      *websocket.Conn
	connected bool
}

func NewOBSClient() *OBSClient {
	return &OBSClient{
		connected: false,
	}
}

func (c *OBSClient) Connect() error {
	conn, _, err := websocket.DefaultDialer.Dial(wsURL, nil)
	if err != nil {
		return fmt.Errorf("failed to connect to OBS: %w", err)
	}

	c.conn = conn

	// Read hello message
	var hello HelloMessage
	if err := conn.ReadJSON(&hello); err != nil {
		return fmt.Errorf("failed to read hello message: %w", err)
	}

	log.Printf("Connected to OBS WebSocket v%s", hello.D.ObsWebSocketVersion)

	// Send identify message
	identify := IdentifyMessage{
		Op: 1,
		D: struct {
			RpcVersion int `json:"rpcVersion"`
		}{
			RpcVersion: 1,
		},
	}

	if err := conn.WriteJSON(identify); err != nil {
		return fmt.Errorf("failed to send identify message: %w", err)
	}

	// Read identify response
	var response ResponseMessage
	if err := conn.ReadJSON(&response); err != nil {
		return fmt.Errorf("failed to read identify response: %w", err)
	}

	if response.Op == 2 {
		log.Println("Successfully identified to OBS WebSocket")
		c.connected = true
	} else {
		return fmt.Errorf("failed to identify to OBS")
	}

	return nil
}

func (c *OBSClient) SendRequest(requestType string) error {
	if !c.connected {
		log.Println("Not connected to OBS. Reconnecting...")
		if err := c.Connect(); err != nil {
			return err
		}
	}

	request := RequestMessage{
		Op: 6,
		D: struct {
			RequestType string `json:"requestType"`
			RequestID   string `json:"requestId"`
		}{
			RequestType: requestType,
			RequestID:   fmt.Sprintf("%s_%d", requestType, time.Now().Unix()),
		},
	}

	if err := c.conn.WriteJSON(request); err != nil {
		c.connected = false
		return fmt.Errorf("failed to send request: %w", err)
	}

	// Read response (but don't block)
	var response map[string]interface{}
	if err := c.conn.ReadJSON(&response); err != nil {
		c.connected = false
		return fmt.Errorf("failed to read response: %w", err)
	}

	return nil
}

func (c *OBSClient) ToggleRecording() {
	log.Println("Toggling recording...")
	if err := c.SendRequest("ToggleRecord"); err != nil {
		log.Printf("Error toggling recording: %v", err)
	}
}

func (c *OBSClient) TogglePause() {
	log.Println("Toggling record pause...")
	if err := c.SendRequest("ToggleRecordPause"); err != nil {
		log.Printf("Error toggling pause: %v", err)
	}
}

func (c *OBSClient) Close() {
	if c.conn != nil {
		c.conn.Close()
	}
}

func findKeyboardDevices() ([]*evdev.InputDevice, error) {
	// Use the built-in ListInputDevices function
	devices, err := evdev.ListInputDevices("/dev/input/event*")
	if err != nil {
		return nil, fmt.Errorf("failed to list input devices: %w", err)
	}

	// Filter to only keyboard devices
	keyboards := []*evdev.InputDevice{}
	for _, device := range devices {
		// Check if device has key capabilities
		caps := device.Capabilities
		// EV_KEY is type 1 - check if this device has key event capabilities
		if codes, hasKeys := caps[evdev.CapabilityType{Type: 1}]; hasKeys && len(codes) > 0 {
			keyboards = append(keyboards, device)
		} else {
			device.File.Close()
		}
	}

	return keyboards, nil
}

func main() {
	log.Println("OBS Hotkey Controller (Go version - Wayland compatible)")

	// Check if running as root
	if os.Geteuid() != 0 {
		log.Fatal("This program must be run as root (sudo) to access keyboard devices")
	}

	// Build hotkey action map
	hotkeyActions := make(map[uint16]func())
	
	client := NewOBSClient()
	defer client.Close()

	for keyCode, keyName := range keyNames {
		if keyName == config.ToggleRecording {
			hotkeyActions[keyCode] = client.ToggleRecording
			log.Printf("- %s: Toggle Recording", keyName)
		} else if keyName == config.TogglePause {
			hotkeyActions[keyCode] = client.TogglePause
			log.Printf("- %s: Toggle Pause/Resume Recording", keyName)
		}
	}

	if len(hotkeyActions) == 0 {
		log.Fatal("No valid hotkeys configured")
	}

	// Find keyboard devices
	log.Println("\nSearching for keyboard devices...")
	devices, err := findKeyboardDevices()
	if err != nil {
		log.Fatalf("Error finding keyboard devices: %v", err)
	}

	if len(devices) == 0 {
		log.Fatal("No keyboard devices found! Make sure you're running with sudo.")
	}

	log.Printf("Found %d keyboard device(s):", len(devices))
	for _, device := range devices {
		log.Printf("  - %s (%s)", device.Name, device.Fn)
	}

	// Connect to OBS
	log.Printf("\nConnecting to OBS WebSocket at %s...", wsURL)
	retries := 0
	for retries < maxRetries {
		if err := client.Connect(); err != nil {
			retries++
			log.Printf("Connection attempt %d/%d failed: %v", retries, maxRetries, err)
			if retries < maxRetries {
				log.Printf("Waiting %v before retrying...", retryDelay)
				time.Sleep(retryDelay)
			}
		} else {
			break
		}
	}

	if !client.connected {
		log.Printf("Failed to connect to OBS after %d attempts.", maxRetries)
		log.Println("Hotkeys are ready but will only work when OBS is running.")
	}

	log.Println("\nListening for hotkeys... (Press Ctrl+C to exit)")

	// Set up signal handling for graceful shutdown
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, os.Interrupt, syscall.SIGTERM)

	// Create event channels for each device
	eventChans := make([]chan evdev.InputEvent, len(devices))
	for i, device := range devices {
		eventChans[i] = make(chan evdev.InputEvent, 10)
		go func(dev *evdev.InputDevice, ch chan evdev.InputEvent) {
			for {
				events, err := dev.Read()
				if err != nil {
					log.Printf("Error reading from %s: %v", dev.Name, err)
					return
				}
				for _, event := range events {
					ch <- event
				}
			}
		}(device, eventChans[i])
	}

	// Main event loop
	reconnectTicker := time.NewTicker(60 * time.Second)
	defer reconnectTicker.Stop()

	for {
		select {
		case <-sigChan:
			log.Println("\nShutting down...")
			for _, device := range devices {
				device.File.Close()
			}
			return

		case <-reconnectTicker.C:
			if !client.connected {
				log.Println("Attempting to reconnect to OBS...")
				client.Connect()
			}

		default:
			// Check all device channels
			for _, ch := range eventChans {
				select {
				case event := <-ch:
					// Only process key press events (value == 1)
					if event.Type == evdev.EV_KEY && event.Value == 1 {
						if action, ok := hotkeyActions[event.Code]; ok {
							action()
						}
					}
				default:
					// No event, continue
				}
			}
			time.Sleep(10 * time.Millisecond)
		}
	}
}
