package main

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"sync"
	"testing"
	"time"

	"github.com/gorilla/websocket"
)

var upgrader = websocket.Upgrader{
	CheckOrigin: func(r *http.Request) bool { return true },
}

type mockOBS struct {
	server           *httptest.Server
	handlerConn      *websocket.Conn
	mut              sync.Mutex
	helloSent        bool
	identifyReceived bool
	identifyReady    chan struct{}
}

func newMockOBS() *mockOBS {
	return &mockOBS{
		identifyReady: make(chan struct{}, 1),
	}
}

func (m *mockOBS) handler(w http.ResponseWriter, r *http.Request) {
	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		return
	}
	defer conn.Close()

	m.mut.Lock()
	m.helloSent = false
	m.identifyReceived = false
	m.handlerConn = conn
	m.mut.Unlock()

	helloBytes, _ := json.Marshal(map[string]interface{}{
		"op": 0,
		"d": map[string]interface{}{
			"obsWebSocketVersion": "5.0.0",
			"rpcVersion":          1,
		},
	})
	if err := conn.WriteMessage(websocket.TextMessage, helloBytes); err != nil {
		return
	}
	m.mut.Lock()
	m.helloSent = true
	m.mut.Unlock()

	var identify map[string]interface{}
	if err := conn.ReadJSON(&identify); err != nil {
		return
	}

	m.mut.Lock()
	m.identifyReceived = true
	m.mut.Unlock()
	close(m.identifyReady)

	identifiedBytes, _ := json.Marshal(map[string]interface{}{
		"op": 2,
		"d": map[string]interface{}{
			"status": "ok",
		},
	})
	conn.WriteMessage(websocket.TextMessage, identifiedBytes)

	for {
		conn.SetReadDeadline(time.Now().Add(5 * time.Second))
		var req map[string]interface{}
		if err := conn.ReadJSON(&req); err != nil {
			return
		}
		reqId := ""
		if d, ok := req["d"].(map[string]interface{}); ok {
			if rid, ok := d["requestId"].(string); ok {
				reqId = rid
			}
		}
		respBytes, _ := json.Marshal(map[string]interface{}{
			"op": 7,
			"d": map[string]interface{}{
				"requestId":     reqId,
				"requestStatus": map[string]interface{}{"result": true, "code": 200},
				"responseData":  map[string]interface{}{"studioModeEnabled": false},
			},
		})
		conn.WriteMessage(websocket.TextMessage, respBytes)
	}
}

func (m *mockOBS) start() {
	m.server = httptest.NewServer(http.HandlerFunc(m.handler))
}

func (m *mockOBS) URL() string {
	return m.server.URL
}

func (m *mockOBS) stop() {
	m.server.CloseClientConnections()
	m.server.Close()
}

func TestConnectEstablishesConnection(t *testing.T) {
	mock := newMockOBS()
	mock.start()
	defer mock.stop()

	client := NewOBSClient("ws" + mock.URL()[4:])
	err := client.Connect()
	if err != nil {
		t.Fatalf("Connect() failed: %v", err)
	}
	if !client.connected.Load() {
		t.Error("connected should be true after successful Connect()")
	}
	client.Close()
}

func TestConnectReleasesMutexOnDialFailure(t *testing.T) {
	client := NewOBSClient("ws://localhost:9999")
	start := time.Now()
	err := client.Connect()
	elapsed := time.Since(start)

	if err == nil {
		t.Fatal("expected Connect() to fail on unreachable address")
	}
	if elapsed > 500*time.Millisecond {
		t.Errorf("Connect() took too long (%v), may indicate mutex not released on dial failure", elapsed)
	}
}

func TestConnectAllErrorPathsUnlockMutex(t *testing.T) {
	tests := []struct {
		name string
		url  string
	}{
		{"invalid_host", "ws://localhost:9999"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			client := NewOBSClient(tt.url)
			err := client.Connect()
			if err == nil {
				t.Errorf("Connect(%s) = nil, expected error", tt.name)
			}
		})
	}
}

func connectedClient(t *testing.T, mock *mockOBS) *OBSClient {
	t.Helper()
	client := NewOBSClient("ws" + mock.URL()[4:])
	if err := client.Connect(); err != nil {
		t.Fatalf("failed to connect: %v", err)
	}
	return client
}

func TestSendRequestSendsSuccessfully(t *testing.T) {
	mock := newMockOBS()
	mock.start()
	defer mock.stop()

	client := connectedClient(t, mock)
	defer client.Close()

	if err := client.SendRequest("ToggleRecord"); err != nil {
		t.Errorf("SendRequest(ToggleRecord) error: %v", err)
	}
}

func TestSendRequestWithDataSendsSuccessfully(t *testing.T) {
	mock := newMockOBS()
	mock.start()
	defer mock.stop()

	client := connectedClient(t, mock)
	defer client.Close()

	err := client.SendRequestWithData("ToggleInputMute", map[string]interface{}{"inputName": "mic"})
	if err != nil {
		t.Errorf("SendRequestWithData error: %v", err)
	}
}

func TestToggleRecordingSendsRequest(t *testing.T) {
	mock := newMockOBS()
	mock.start()
	defer mock.stop()

	client := connectedClient(t, mock)
	defer client.Close()

	client.ToggleRecording()
}

func TestTogglePauseSendsRequest(t *testing.T) {
	mock := newMockOBS()
	mock.start()
	defer mock.stop()

	client := connectedClient(t, mock)
	defer client.Close()

	client.TogglePause()
}

func TestToggleStreamingSendsRequest(t *testing.T) {
	mock := newMockOBS()
	mock.start()
	defer mock.stop()

	client := connectedClient(t, mock)
	defer client.Close()

	client.ToggleStreaming()
}

func TestToggleReplayBufferSendsRequest(t *testing.T) {
	mock := newMockOBS()
	mock.start()
	defer mock.stop()

	client := connectedClient(t, mock)
	defer client.Close()

	client.ToggleReplayBuffer()
}

func TestSaveReplaySendsRequest(t *testing.T) {
	mock := newMockOBS()
	mock.start()
	defer mock.stop()

	client := connectedClient(t, mock)
	defer client.Close()

	client.SaveReplay()
}

func TestToggleMuteMicSkipsWhenEmpty(t *testing.T) {
	mock := newMockOBS()
	mock.start()
	defer mock.stop()

	client := connectedClient(t, mock)
	defer client.Close()

	client.ToggleMuteMic("")
}

func TestToggleMuteMicSendsRequest(t *testing.T) {
	mock := newMockOBS()
	mock.start()
	defer mock.stop()

	client := connectedClient(t, mock)
	defer client.Close()

	client.ToggleMuteMic("Microphone")
}

func TestToggleStudioModeSendsRequest(t *testing.T) {
	mock := newMockOBS()
	mock.start()
	defer mock.stop()

	client := connectedClient(t, mock)
	defer client.Close()

	client.ToggleStudioMode()
	if !client.studioModeEnabled.Load() {
		t.Error("expected studioModeEnabled to be true after toggle")
	}
}

func TestScreenshotSendsRequest(t *testing.T) {
	mock := newMockOBS()
	mock.start()
	defer mock.stop()

	client := connectedClient(t, mock)
	defer client.Close()

	client.Screenshot("", t.TempDir())
}

func TestSendRequestWithDataReconnectPathNoDeadlock(t *testing.T) {
	mock := newMockOBS()
	mock.start()
	defer mock.stop()

	client := NewOBSClient("ws" + mock.URL()[4:])
	client.connected.Store(false)

	connected := make(chan struct{})
	var errOut error

	var wg sync.WaitGroup
	wg.Add(1)
	go func() {
		defer wg.Done()
		errOut = client.SendRequestWithData("GetStudioModeEnabled", nil)
		if errOut != nil {
			t.Logf("SendRequestWithData error: %v", errOut)
		}
		close(connected)
	}()

	select {
	case <-connected:
	case <-time.After(5 * time.Second):
		t.Fatal("SendRequestWithData did not complete within 5s — possible deadlock")
	}
	wg.Wait()
}
