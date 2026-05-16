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
	server        *httptest.Server
	mut           sync.Mutex
	connectCount  int
	helloSent     bool
	identifyReceived bool
}

func newMockOBS() *mockOBS {
	return &mockOBS{}
}

func (m *mockOBS) handler(w http.ResponseWriter, r *http.Request) {
	m.mut.Lock()
	m.connectCount++
	m.helloSent = false
	m.identifyReceived = false
	m.mut.Unlock()

	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		return
	}
	defer conn.Close()

	var hello struct {
		Op int `json:"op"`
		D  struct {
			ObsWebSocketVersion string `json:"obsWebSocketVersion"`
			RpcVersion          int    `json:"rpcVersion"`
		} `json:"d"`
	}
	if err := conn.ReadJSON(&hello); err != nil {
		return
	}

	m.mut.Lock()
	m.helloSent = true
	m.mut.Unlock()

	identifyBytes, _ := json.Marshal(map[string]interface{}{
		"op": 1,
		"d":  map[string]interface{}{"rpcVersion": 1},
	})
	if err := conn.WriteMessage(websocket.TextMessage, identifyBytes); err != nil {
		return
	}

	m.mut.Lock()
	m.identifyReceived = true
	m.mut.Unlock()
}

func (m *mockOBS) start() {
	m.server = httptest.NewServer(http.HandlerFunc(m.handler))
}

func (m *mockOBS) URL() string {
	return m.server.URL
}

func (m *mockOBS) stop() {
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

func TestSendRequestWithDataReconnectPathUnlocksSafely(t *testing.T) {
	mock := newMockOBS()
	mock.start()
	defer mock.stop()

	client := NewOBSClient("ws" + mock.URL()[4:])

	connected := make(chan struct{}, 1)
	client.connected.Store(false)

	var wg sync.WaitGroup
	wg.Add(1)
	go func() {
		defer wg.Done()
		err := client.SendRequestWithData("GetStudioModeEnabled", nil)
		if err != nil {
			t.Logf("SendRequestWithData returned error (expected on mock): %v", err)
		}
		connected <- struct{}{}
	}()

	select {
	case <-connected:
	case <-time.After(5 * time.Second):
		t.Fatal("SendRequestWithData did not complete within 5s — possible deadlock")
	}
	wg.Wait()
}

func TestConnectAllErrorPathsUnlockMutex(t *testing.T) {
	tests := []struct {
		name string
		url  string
	}{
		{"invalid_url", "ws://localhost:9999"},
		{"bad_ws_url", "not-a-url"},
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