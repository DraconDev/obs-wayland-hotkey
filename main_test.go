package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestEventDevicePathRegex(t *testing.T) {
	tests := []struct {
		input   string
		matches bool
	}{
		{"event0", true},
		{"event1", true},
		{"event12", true},
		{"event999", true},
		{"/dev/input/event0", false},
		{"event", false},
		{"event0x", false},
		{"Event0", false},
		{"event-1", false},
		{"mouse0", false},
		{"js0", false},
		{"by-id", false},
		{"", false},
		{" ", false},
		{"event0\n", false},
	}

	for _, tt := range tests {
		t.Run(tt.input, func(t *testing.T) {
			got := eventDevicePath.MatchString(tt.input)
			if got != tt.matches {
				t.Errorf("eventDevicePath.MatchString(%q) = %v, want %v", tt.input, got, tt.matches)
			}
		})
	}
}

func TestEventDevicePathRegexExtractsNumber(t *testing.T) {
	matches := eventDevicePath.FindStringSubmatch("event42")
	if len(matches) != 2 {
		t.Fatalf("expected 2 submatches, got %d", len(matches))
	}
	if matches[1] != "42" {
		t.Errorf("expected submatch '42', got %q", matches[1])
	}
}

func TestSanitizeOBSHost(t *testing.T) {
	tests := []struct {
		input string
		want  string
	}{
		{"localhost:4455", "ws://localhost:4455"},
		{"ws://localhost:4455", "ws://localhost:4455"},
		{"wss://localhost:4455", "wss://localhost:4455"},
		{"", ""},
		{"192.168.1.1:4455", "ws://192.168.1.1:4455"},
	}

	for _, tt := range tests {
		t.Run(tt.input, func(t *testing.T) {
			got := sanitizeOBSHost(tt.input)
			if got != tt.want {
				t.Errorf("sanitizeOBSHost(%q) = %q, want %q", tt.input, got, tt.want)
			}
		})
	}
}

func TestExpandHome(t *testing.T) {
	homeDir, err := os.UserHomeDir()
	if err != nil {
		t.Fatal(err)
	}

	tests := []struct {
		input string
		want  string
	}{
		{"~/Pictures", filepath.Join(homeDir, "Pictures")},
		{"/tmp/abs", "/tmp/abs"},
		{"relative", "relative"},
		{"", ""},
	}

	for _, tt := range tests {
		t.Run(tt.input, func(t *testing.T) {
			got := expandHome(tt.input)
			if got != tt.want {
				t.Errorf("expandHome(%q) = %q, want %q", tt.input, got, tt.want)
			}
		})
	}
}

func TestGetKeyCode(t *testing.T) {
	if code := getKeyCode("scroll lock"); code == 0 {
		t.Error("getKeyCode('scroll lock') returned 0, expected non-zero")
	}
	if code := getKeyCode("pause"); code == 0 {
		t.Error("getKeyCode('pause') returned 0, expected non-zero")
	}
	if code := getKeyCode("f1"); code == 0 {
		t.Error("getKeyCode('f1') returned 0, expected non-zero")
	}
	if code := getKeyCode("nonexistent_key"); code != 0 {
		t.Errorf("getKeyCode('nonexistent_key') = %d, expected 0", code)
	}
}

func TestGetConfigPath(t *testing.T) {
	t.Run("explicit flag overrides default", func(t *testing.T) {
		got := getConfigPath("/custom/path/config.json")
		if got != "/custom/path/config.json" {
			t.Errorf("getConfigPath('/custom/path/config.json') = %q, want '/custom/path/config.json'", got)
		}
	})

	t.Run("empty flag uses default location", func(t *testing.T) {
		got := getConfigPath("")
		if got == "" {
			t.Error("getConfigPath('') returned empty string")
		}
		expectedSuffix := filepath.Join(".config", "obs-hotkey", "hotkeys.json")
		if len(got) < len(expectedSuffix) || got[len(got)-len(expectedSuffix):] != expectedSuffix {
			t.Errorf("getConfigPath('') = %q, expected to end with %q", got, expectedSuffix)
		}
	})

	t.Run("XDG_CONFIG_HOME is respected", func(t *testing.T) {
		origXDG := os.Getenv("XDG_CONFIG_HOME")
		os.Setenv("XDG_CONFIG_HOME", "/xdg/config")
		defer os.Setenv("XDG_CONFIG_HOME", origXDG)

		got := getConfigPath("")
		want := filepath.Join("/xdg/config", "obs-hotkey", "hotkeys.json")
		if got != want {
			t.Errorf("getConfigPath('') with XDG_CONFIG_HOME = %q, want %q", got, want)
		}
	})
}

func TestDefaultConfig(t *testing.T) {
	cfg := defaultConfig()
	if cfg.OBSHost != defaultWSURL {
		t.Errorf("default OBSHost = %q, want %q", cfg.OBSHost, defaultWSURL)
	}
	if cfg.Hotkeys.ToggleRecording != "scroll lock" {
		t.Errorf("default ToggleRecording = %q, want 'scroll lock'", cfg.Hotkeys.ToggleRecording)
	}
	if cfg.Hotkeys.TogglePause != "pause" {
		t.Errorf("default TogglePause = %q, want 'pause'", cfg.Hotkeys.TogglePause)
	}
}

func TestLoadConfig(t *testing.T) {
	t.Run("missing file returns error", func(t *testing.T) {
		_, err := loadConfig("/nonexistent/path/config.json")
		if err == nil {
			t.Error("expected error for missing config file")
		}
	})

	t.Run("invalid json returns error", func(t *testing.T) {
		dir := t.TempDir()
		path := filepath.Join(dir, "bad.json")
		os.WriteFile(path, []byte("not json"), 0644)
		_, err := loadConfig(path)
		if err == nil {
			t.Error("expected error for invalid JSON")
		}
	})

	t.Run("valid json loads correctly", func(t *testing.T) {
		dir := t.TempDir()
		path := filepath.Join(dir, "hotkeys.json")
		content := `{"obs_host":"ws://localhost:4455","hotkeys":{"toggle_recording":"f1","toggle_pause":"f2"}}`
		os.WriteFile(path, []byte(content), 0644)

		cfg, err := loadConfig(path)
		if err != nil {
			t.Fatalf("loadConfig() error: %v", err)
		}
		if cfg.OBSHost != "ws://localhost:4455" {
			t.Errorf("OBSHost = %q, want 'ws://localhost:4455'", cfg.OBSHost)
		}
		if cfg.Hotkeys.ToggleRecording != "f1" {
			t.Errorf("ToggleRecording = %q, want 'f1'", cfg.Hotkeys.ToggleRecording)
		}
		if cfg.Hotkeys.TogglePause != "f2" {
			t.Errorf("TogglePause = %q, want 'f2'", cfg.Hotkeys.TogglePause)
		}
	})

	t.Run("bare host gets ws:// prefix", func(t *testing.T) {
		dir := t.TempDir()
		path := filepath.Join(dir, "hotkeys.json")
		content := `{"obs_host":"localhost:4455"}`
		os.WriteFile(path, []byte(content), 0644)

		cfg, err := loadConfig(path)
		if err != nil {
			t.Fatalf("loadConfig() error: %v", err)
		}
		if cfg.OBSHost != "ws://localhost:4455" {
			t.Errorf("OBSHost = %q, want 'ws://localhost:4455'", cfg.OBSHost)
		}
	})

	t.Run("tilde in screenshot_dir is expanded", func(t *testing.T) {
		dir := t.TempDir()
		path := filepath.Join(dir, "hotkeys.json")
		content := `{"screenshot_dir":"~/Pictures"}`
		os.WriteFile(path, []byte(content), 0644)

		cfg, err := loadConfig(path)
		if err != nil {
			t.Fatalf("loadConfig() error: %v", err)
		}
		if cfg.ScreenshotDir == "~/Pictures" {
			t.Error("screenshot_dir tilde was not expanded")
		}
	})
}

func TestEnsureConfig(t *testing.T) {
	t.Run("creates default config if missing", func(t *testing.T) {
		dir := t.TempDir()
		path := filepath.Join(dir, "config", "hotkeys.json")

		err := ensureConfig(filepath.Dir(path), path)
		if err != nil {
			t.Fatalf("ensureConfig() error: %v", err)
		}

		if _, statErr := os.Stat(path); statErr != nil {
			t.Errorf("config file not created: %v", statErr)
		}
	})

	t.Run("does not overwrite existing config", func(t *testing.T) {
		dir := t.TempDir()
		path := filepath.Join(dir, "hotkeys.json")
		original := `{"obs_host":"ws://custom:1234"}`
		os.WriteFile(path, []byte(original), 0644)

		err := ensureConfig(dir, path)
		if err != nil {
			t.Fatalf("ensureConfig() error: %v", err)
		}

		data, readErr := os.ReadFile(path)
		if readErr != nil {
			t.Fatalf("failed to read config: %v", readErr)
		}
		if string(data) != original {
			t.Errorf("existing config was overwritten: got %q, want %q", string(data), original)
		}
	})
}

func TestWriteServiceFile(t *testing.T) {
	dir := t.TempDir()
	oldHome := os.Getenv("HOME")
	os.Setenv("HOME", dir)
	defer os.Setenv("HOME", oldHome)

	exePath := "/usr/bin/obs-hotkey"
	cfgDir := filepath.Join(dir, ".config", "obs-hotkey")

	err := writeServiceFile(exePath, cfgDir)
	if err != nil {
		t.Fatalf("writeServiceFile() error: %v", err)
	}

	unitPath := filepath.Join(dir, ".config", "systemd", "user", "obs-hotkey.service")
	data, err := os.ReadFile(unitPath)
	if err != nil {
		t.Fatalf("failed to read service file: %v", err)
	}

	if !strings.Contains(string(data), "ExecStart="+exePath+" --config "+cfgDir+"/hotkeys.json") {
		t.Errorf("service file does not contain correct ExecStart line")
	}
	if !strings.Contains(string(data), "WantedBy=graphical-session.target") {
		t.Errorf("service file does not contain WantedBy")
	}
}

func TestIsAutostartEnabled(t *testing.T) {
	enabled := isAutostartEnabled()
	_ = enabled
}

func TestInInputGroup(t *testing.T) {
	result := inInputGroup()
	if os.Getenv("USER") == "" {
		t.Skip("USER env not set")
	}
	if result {
		t.Log("Current user is in input group")
	} else {
		t.Log("Current user is NOT in input group")
	}
}

func TestRunningUnderSystemd(t *testing.T) {
	result := runningUnderSystemd()
	t.Logf("runningUnderSystemd() = %v", result)
}

func TestServiceUnitPath(t *testing.T) {
	oldHome := os.Getenv("HOME")
	os.Setenv("HOME", "/home/testuser")
	defer os.Setenv("HOME", oldHome)

	path := serviceUnitPath()
	want := "/home/testuser/.config/systemd/user/obs-hotkey.service"
	if path != want {
		t.Errorf("serviceUnitPath() = %q, want %q", path, want)
	}
}
