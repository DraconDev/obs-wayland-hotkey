package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"log"
	"os"
	"os/signal"
	"path/filepath"
	"regexp"
	"strings"
	"sync"
	"sync/atomic"
	"syscall"
	"time"

	"github.com/gorilla/websocket"
	evdev "github.com/gvalkov/golang-evdev"
)

const (
	defaultWSURL    = "ws://localhost:4455"
	maxRetries       = 10
	retryDelay       = 30 * time.Second
	configDirName    = "obs-hotkey"
	configFileName   = "hotkeys.json"
)

func getConfigPath(configFlag string) string {
	if configFlag != "" {
		return configFlag
	}

	if xdg := os.Getenv("XDG_CONFIG_HOME"); xdg != "" {
		return filepath.Join(xdg, configDirName, configFileName)
	}

	homeDir := getRealHome()
	return filepath.Join(homeDir, ".config", configDirName, configFileName)
}

func getRealHome() string {
	if sudoUser := os.Getenv("SUDO_USER"); sudoUser != "" {
		if passwd, err := os.ReadFile("/etc/passwd"); err == nil {
			for _, line := range strings.Split(string(passwd), "\n") {
				parts := strings.Split(line, ":")
				if len(parts) >= 6 && parts[0] == sudoUser {
					return parts[5]
				}
			}
		}
	}
	homeDir, _ := os.UserHomeDir()
	return homeDir
}

func defaultConfig() AppConfig {
	return AppConfig{
		OBSHost: defaultWSURL,
		Hotkeys: HotkeyConfig{
			ToggleRecording: "scroll lock",
			TogglePause:     "pause",
			ToggleStreaming: "",
			Screenshot:      "",
			ToggleMuteMic:   "",
			ToggleStudioMode: "",
			ToggleReplayBuf: "",
			SaveReplay:      "",
		},
		ScreenshotDir: "~/Pictures",
		MicName:      "",
	}
}

func expandHome(path string) string {
	if len(path) > 0 && path[0] == '~' {
		homeDir := getRealHome()
		return filepath.Join(homeDir, path[1:])
	}
	return path
}

func sanitizeOBSHost(host string) string {
	if host != "" && !strings.HasPrefix(host, "ws://") && !strings.HasPrefix(host, "wss://") {
		return "ws://" + host
	}
	return host
}

func loadConfig(path string) (AppConfig, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return AppConfig{}, fmt.Errorf("config file not found")
		}
		return AppConfig{}, fmt.Errorf("failed to read config: %w", err)
	}

	var cfg AppConfig
	if err := json.Unmarshal(data, &cfg); err != nil {
		return AppConfig{}, fmt.Errorf("failed to parse config: %w", err)
	}

	cfg.OBSHost = sanitizeOBSHost(cfg.OBSHost)
	cfg.ScreenshotDir = expandHome(cfg.ScreenshotDir)

	return cfg, nil
}

func ensureConfig(dirPath, filePath string) error {
	if _, err := os.Stat(filePath); err == nil {
		return nil
	}

	if err := os.MkdirAll(dirPath, 0755); err != nil {
		return fmt.Errorf("failed to create config directory: %w", err)
	}

	cfg := defaultConfig()
	data, err := json.MarshalIndent(cfg, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal default config: %w", err)
	}

	if err := os.WriteFile(filePath, data, 0644); err != nil {
		return fmt.Errorf("failed to write default config: %w", err)
	}

	log.Printf("Created default config at: %s", filePath)
	return nil
}

func getKeyCode(keyName string) uint16 {
	if code, ok := keyNameToCode[keyName]; ok {
		return code
	}
	return 0
}

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
		RequestType string                 `json:"requestType"`
		RequestID   string                 `json:"requestId"`
		RequestData map[string]interface{} `json:"requestData,omitempty"`
	} `json:"d"`
}

type ResponseMessage struct {
	Op int `json:"op"`
	D  struct {
		RequestID string `json:"requestId"`
	} `json:"d"`
}

// Config file structures
type HotkeyConfig struct {
	ToggleRecording  string `json:"toggle_recording"`
	TogglePause     string `json:"toggle_pause"`
	ToggleStreaming string `json:"toggle_streaming"`
	Screenshot      string `json:"screenshot"`
	ToggleMuteMic   string `json:"toggle_mute_mic"`
	ToggleStudioMode string `json:"toggle_studio_mode"`
	ToggleReplayBuf string `json:"toggle_replay_buffer"`
	SaveReplay      string `json:"save_replay"`
}

type AppConfig struct {
	OBSHost          string        `json:"obs_host"`
	Hotkeys          HotkeyConfig  `json:"hotkeys"`
	ScreenshotSource string        `json:"screenshot_source"`
	ScreenshotDir    string        `json:"screenshot_dir"`
	MicName          string        `json:"mic_name"`
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
	evdev.KEY_F13:        "f13",
	evdev.KEY_F14:        "f14",
	evdev.KEY_F15:        "f15",
	evdev.KEY_F16:        "f16",
	evdev.KEY_F17:        "f17",
	evdev.KEY_F18:        "f18",
	evdev.KEY_F19:        "f19",
	evdev.KEY_F20:        "f20",
	evdev.KEY_F21:        "f21",
	evdev.KEY_F22:        "f22",
	evdev.KEY_F23:        "f23",
	evdev.KEY_F24:        "f24",
}

var keyNameToCode map[string]uint16

func init() {
	keyNameToCode = make(map[string]uint16, len(keyNames))
	for code, name := range keyNames {
		keyNameToCode[name] = code
	}
}
type OBSClient struct {
	conn            *websocket.Conn
	connected       atomic.Bool
	studioModeEnabled atomic.Bool
	studioModeQueried atomic.Bool
	wsURL           string
	mu              sync.Mutex // protects conn during read/write
}

func NewOBSClient(wsURL string) *OBSClient {
	c := &OBSClient{
		wsURL: wsURL,
	}
	c.connected.Store(false)
	c.studioModeEnabled.Store(false)
	c.studioModeQueried.Store(false)
	return c
}

func (c *OBSClient) Connect() error {
	c.mu.Lock()

	if c.conn != nil {
		c.conn.Close()
		c.conn = nil
	}
	c.connected.Store(false)

	conn, _, err := websocket.DefaultDialer.Dial(c.wsURL, nil)
	if err != nil {
		return fmt.Errorf("failed to connect to OBS: %w", err)
	}

	c.conn = conn

	// Read hello message
	conn.SetReadDeadline(time.Now().Add(10 * time.Second))
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

	// Clear read deadline after handshake; use ping handler for keepalive
	conn.SetReadDeadline(time.Time{})
	conn.SetPingHandler(func(msg string) error {
		return conn.WriteControl(websocket.PongMessage, []byte(msg), time.Now().Add(5*time.Second))
	})

	if response.Op == 2 {
		log.Println("Successfully identified to OBS WebSocket")
		c.connected.Store(true)
		c.mu.Unlock()
		c.QueryStudioMode()
	} else {
		c.mu.Unlock()
		return fmt.Errorf("failed to identify to OBS")
	}

	return nil
}
func (c *OBSClient) QueryStudioMode() {
	c.mu.Lock()
	defer c.mu.Unlock()

	type studioModeResponse struct {
		Op   int `json:"op"`
		D    struct {
			RequestID    string `json:"requestId"`
			RequestStatus struct {
				Result bool   `json:"result"`
				Code   int    `json:"code"`
			} `json:"requestStatus"`
			ResponseData struct {
				StudioModeEnabled bool `json:"studioModeEnabled"`
			} `json:"responseData"`
		} `json:"d"`
	}

	request := RequestMessage{
		Op: 6,
		D: struct {
			RequestType string                 `json:"requestType"`
			RequestID   string                 `json:"requestId"`
			RequestData map[string]interface{} `json:"requestData,omitempty"`
		}{
			RequestType: "GetStudioModeEnabled",
			RequestID:   fmt.Sprintf("GetStudioModeEnabled_%d", time.Now().Unix()),
		},
	}

	if err := c.conn.WriteJSON(request); err != nil {
		log.Printf("Failed to query studio mode: %v", err)
		return
	}

	var response studioModeResponse
	if err := c.conn.ReadJSON(&response); err != nil {
		log.Printf("Failed to read studio mode response: %v", err)
		return
	}

	if response.D.RequestStatus.Result {
		c.studioModeEnabled.Store(response.D.ResponseData.StudioModeEnabled)
	}
	c.studioModeQueried.Store(true)
	log.Printf("Studio mode is currently: %v", c.studioModeEnabled.Load())
}

func (c *OBSClient) SendRequest(requestType string) error {
	return c.SendRequestWithData(requestType, nil)
}

func (c *OBSClient) SendRequestWithData(requestType string, requestData map[string]interface{}) error {
	c.mu.Lock()
	defer c.mu.Unlock()

	if !c.connected.Load() {
		c.mu.Unlock()
		log.Println("Not connected to OBS. Reconnecting...")
		if err := c.Connect(); err != nil {
			return err
		}
		c.mu.Lock()
	}

	request := RequestMessage{
		Op: 6,
		D: struct {
			RequestType string                 `json:"requestType"`
			RequestID   string                 `json:"requestId"`
			RequestData map[string]interface{} `json:"requestData,omitempty"`
		}{
			RequestType: requestType,
			RequestID:   fmt.Sprintf("%s_%d", requestType, time.Now().Unix()),
			RequestData: requestData,
		},
	}

	if err := c.conn.WriteJSON(request); err != nil {
		c.connected.Store(false)
		return fmt.Errorf("failed to send request: %w", err)
	}

	// Read response (but don't block)
	var response map[string]interface{}
	c.conn.SetReadDeadline(time.Now().Add(5 * time.Second))
	if err := c.conn.ReadJSON(&response); err != nil {
		c.connected.Store(false)
		c.conn.SetReadDeadline(time.Time{})
		return fmt.Errorf("failed to read response: %w", err)
	}
	c.conn.SetReadDeadline(time.Time{})

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

func (c *OBSClient) ToggleStreaming() {
	log.Println("Toggling stream...")
	if err := c.SendRequest("ToggleStream"); err != nil {
		log.Printf("Error toggling stream: %v", err)
	}
}

func (c *OBSClient) Screenshot(sourceName, saveDir string) {
	log.Println("Taking screenshot...")
	reqData := map[string]interface{}{
		"imageFormat":    "png",
		"imageFilePath": fmt.Sprintf("%s/obs-screenshot-%d.png", saveDir, time.Now().UnixMilli()),
	}
	if sourceName != "" {
		reqData["sourceName"] = sourceName
	}
	if err := c.SendRequestWithData("SaveSourceScreenshot", reqData); err != nil {
		log.Printf("Error taking screenshot: %v", err)
	} else {
		log.Printf("Screenshot saved to: %s", reqData["imageFilePath"])
	}
}

func (c *OBSClient) ToggleMuteMic(inputName string) {
	if inputName == "" {
		log.Println("Mic input name not configured, skipping mute toggle")
		return
	}
	log.Println("Toggling mic mute...")
	reqData := map[string]interface{}{
		"inputName": inputName,
	}
	if err := c.SendRequestWithData("ToggleInputMute", reqData); err != nil {
		log.Printf("Error toggling mic mute: %v", err)
	}
}

func (c *OBSClient) ToggleStudioMode() {
	log.Println("Toggling studio mode...")
	if !c.studioModeQueried.Load() {
		log.Println("Studio mode state unknown, querying...")
		c.QueryStudioMode()
	}
	newState := !c.studioModeEnabled.Load()
	reqData := map[string]interface{}{
		"studioModeEnabled": newState,
	}
	if err := c.SendRequestWithData("SetStudioModeEnabled", reqData); err != nil {
		log.Printf("Error toggling studio mode: %v", err)
	} else {
		c.studioModeEnabled.Store(newState)
		log.Printf("Studio mode set to: %v", newState)
	}
}

func (c *OBSClient) ToggleReplayBuffer() {
	log.Println("Toggling replay buffer...")
	if err := c.SendRequest("ToggleReplayBuffer"); err != nil {
		log.Printf("Error toggling replay buffer: %v", err)
	}
}

func (c *OBSClient) SaveReplay() {
	log.Println("Saving replay buffer...")
	if err := c.SendRequest("SaveReplayBuffer"); err != nil {
		log.Printf("Error saving replay: %v", err)
	}
}

func (c *OBSClient) Close() {
	if c.conn != nil {
		c.conn.Close()
	}
}

var eventDevicePath = regexp.MustCompile(`^/dev/input/event(\d+)$`)

func findKeyboardDevices() ([]*evdev.InputDevice, []chan evdev.InputEvent, error) {
	keyboards := []*evdev.InputDevice{}
	channels := []chan evdev.InputEvent{}

	entries, err := os.ReadDir("/dev/input")
	if err != nil {
		return nil, nil, fmt.Errorf("failed to read /dev/input: %w", err)
	}

	for _, entry := range entries {
		if !eventDevicePath.MatchString(entry.Name()) {
			continue
		}

		path := filepath.Join("/dev/input", entry.Name())
		device, err := evdev.Open(path)
		if err != nil {
			log.Printf("Warning: could not open %s: %v", path, err)
			continue
		}

		hasKeyboard := false
		for capType := range device.Capabilities {
			if capType.Type == 1 { // EV_KEY
				hasKeyboard = true
				break
			}
		}

		if hasKeyboard {
			ch := make(chan evdev.InputEvent, 10)
			keyboards = append(keyboards, device)
			channels = append(channels, ch)
		} else {
			device.File.Close()
		}
	}

	return keyboards, channels, nil
}

func main() {
	log.Println("OBS Hotkey Controller - Wayland compatible")

	configFlag := flag.String("config", "", "Path to config file (overrides default location)")
	flag.Parse()
	configPath := getConfigPath(*configFlag)
	dirPath := filepath.Dir(configPath)

	if err := ensureConfig(dirPath, configPath); err != nil {
		log.Printf("Warning: could not ensure config file: %v", err)
	}

	cfg, err := loadConfig(configPath)
	if err != nil {
		log.Fatalf("Failed to load config from %s: %v\nSet your hotkeys in the config file.", configPath, err)
	}

	log.Printf("Loaded config from: %s", configPath)

	wsURL := cfg.OBSHost
	if wsURL == "" {
		wsURL = defaultWSURL
	}

	hotkeyActions := make(map[uint16]func())

	client := NewOBSClient(wsURL)
	defer client.Close()

	type hotkeyBinding struct {
		keyName string
		action  func()
		label   string
	}

	bindings := []hotkeyBinding{
		{cfg.Hotkeys.ToggleRecording, client.ToggleRecording, "Toggle Recording"},
		{cfg.Hotkeys.TogglePause, client.TogglePause, "Toggle Pause/Resume"},
		{cfg.Hotkeys.ToggleStreaming, client.ToggleStreaming, "Toggle Streaming"},
		{cfg.Hotkeys.Screenshot, func() { client.Screenshot(cfg.ScreenshotSource, cfg.ScreenshotDir) }, "Screenshot"},
		{cfg.Hotkeys.ToggleMuteMic, func() { client.ToggleMuteMic(cfg.MicName) }, "Toggle Mic Mute"},
		{cfg.Hotkeys.ToggleStudioMode, client.ToggleStudioMode, "Toggle Studio Mode"},
		{cfg.Hotkeys.ToggleReplayBuf, client.ToggleReplayBuffer, "Toggle Replay Buffer"},
		{cfg.Hotkeys.SaveReplay, client.SaveReplay, "Save Replay"},
	}

	for _, b := range bindings {
		if b.keyName == "" {
			continue
		}
		keyCode := getKeyCode(b.keyName)
		if keyCode == 0 {
			log.Printf("Warning: unknown key '%s' for %s", b.keyName, b.label)
			continue
		}
		hotkeyActions[keyCode] = b.action
		log.Printf("- %s: %s", b.keyName, b.label)
	}

	if len(hotkeyActions) == 0 {
		log.Fatal("No valid hotkeys configured")
	}

	var eventChans []chan evdev.InputEvent

	// Find keyboard devices
	log.Println("\nSearching for keyboard devices...")
	devices, eventChans, err := findKeyboardDevices()
	if err != nil {
		log.Fatalf("Error finding keyboard devices: %v", err)
	}

	if len(devices) == 0 {
		log.Fatal("No keyboard devices found! Make sure you're in the input group.")
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

	if !client.connected.Load() {
		log.Printf("Failed to connect to OBS after %d attempts.", maxRetries)
		log.Println("Hotkeys are ready but will only work when OBS is running.")
	}

	log.Println("\nListening for hotkeys... (Press Ctrl+C to exit)")

	// Set up signal handling for graceful shutdown
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, os.Interrupt, syscall.SIGTERM)

	// Start device readers
	deviceClosed := make(chan *evdev.InputDevice, len(devices))
	for i, device := range devices {
		go func(dev *evdev.InputDevice, ch chan evdev.InputEvent) {
			for {
				events, err := dev.Read()
				if err != nil {
					log.Printf("Error reading from %s: %v", dev.Name, err)
					close(ch)
					deviceClosed <- dev
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
				if device != nil {
					device.File.Close()
				}
			}
			return

		case dev := <-deviceClosed:
			// A device was unplugged or errored; mark it closed so we don't try
			// to close it again in the shutdown path. We don't re-scan for new
			// devices — the user must restart the service on hot-plug changes.
			for i, d := range devices {
				if d == dev {
					devices[i] = nil // mark as closed
					close(eventChans[i]) // close the channel
				}
			}

		case <-reconnectTicker.C:
			if !client.connected.Load() {
				log.Println("Attempting to reconnect to OBS...")
				client.Connect()
			}

		default:
			// Check all device channels
			for _, ch := range eventChans {
				select {
				case event, ok := <-ch:
					if !ok {
						continue
					}
					// Only process key press events (value == 1, not 0 for release or 2 for repeat)
					if event.Type == 1 && event.Value == 1 {
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
