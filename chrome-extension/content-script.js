// Content Script: WebSocket Client for Meeting Minutes Automator
// Walking Skeleton (MVP0) - Connects to Tauri WebSocket server

class WebSocketClient {
  constructor() {
    this.ws = null;
    this.connected = false;
    this.reconnectAttempts = 0;
    this.maxReconnectAttempts = 5;
    this.reconnectDelays = [1000, 2000, 4000, 8000, 16000]; // Exponential backoff
    this.portRange = { start: 9001, end: 9100 };
    this.currentPort = this.portRange.start;
  }

  /**
   * Start WebSocket connection with port scanning
   */
  async connect() {
    console.log('[Meeting Minutes] Starting WebSocket connection...');
    
    // Try to restore last successful port from storage
    const stored = await chrome.storage.local.get(['lastPort']);
    if (stored.lastPort) {
      this.currentPort = stored.lastPort;
      console.log(`[Meeting Minutes] Trying last successful port: ${this.currentPort}`);
    }

    await this.tryConnect(this.currentPort);
  }

  /**
   * Try to connect to a specific port
   */
  async tryConnect(port) {
    const url = `ws://127.0.0.1:${port}`;
    console.log(`[Meeting Minutes] Attempting connection to ${url}...`);

    try {
      this.ws = new WebSocket(url);

      // Connection timeout (1 second)
      const timeout = setTimeout(() => {
        if (this.ws.readyState !== WebSocket.OPEN) {
          console.log(`[Meeting Minutes] Connection timeout on port ${port}`);
          this.ws.close();
          this.tryNextPort();
        }
      }, 1000);

      this.ws.onopen = () => {
        clearTimeout(timeout);
        this.connected = true;
        this.reconnectAttempts = 0;
        console.log(`[Meeting Minutes] ✅ Connected to WebSocket server on port ${port}`);

        // Save connection state (AC-007.5)
        chrome.storage.local.set({
          lastPort: port,
          connectionStatus: 'connected',
          lastAttempt: Date.now(),
          lastError: null
        }, () => {
          // Log saved state for verification
          chrome.storage.local.get(null, (items) => {
            console.log('[Meeting Minutes] 📦 Storage saved:', items);
          });
        });
      };

      this.ws.onmessage = (event) => {
        this.handleMessage(event.data);
      };

      this.ws.onerror = (error) => {
        clearTimeout(timeout);
        console.error(`[Meeting Minutes] WebSocket error on port ${port}:`, error);

        // Save error state (AC-007.5)
        chrome.storage.local.set({
          connectionStatus: 'error',
          lastAttempt: Date.now(),
          lastError: error.type || 'WebSocket error'
        }, () => {
          // Log saved state for verification
          chrome.storage.local.get(null, (items) => {
            console.log('[Meeting Minutes] 📦 Storage saved (error):', items);
          });
        });
      };

      this.ws.onclose = () => {
        clearTimeout(timeout);
        this.connected = false;
        console.log(`[Meeting Minutes] WebSocket connection closed`);

        // Save disconnected state (AC-007.5)
        chrome.storage.local.set({
          connectionStatus: 'disconnected',
          lastAttempt: Date.now()
        }, () => {
          // Log saved state for verification
          chrome.storage.local.get(null, (items) => {
            console.log('[Meeting Minutes] 📦 Storage saved (disconnected):', items);
          });
        });

        // Attempt reconnection
        this.reconnect();
      };

    } catch (error) {
      console.error(`[Meeting Minutes] Failed to connect to port ${port}:`, error);
      this.tryNextPort();
    }
  }

  /**
   * Try next port in range
   */
  tryNextPort() {
    this.currentPort++;
    
    if (this.currentPort <= this.portRange.end) {
      this.tryConnect(this.currentPort);
    } else {
      console.error('[Meeting Minutes] ❌ No WebSocket server found in port range 9001-9100');
      this.reconnect();
    }
  }

  /**
   * Reconnection logic with exponential backoff
   */
  reconnect() {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.error('[Meeting Minutes] ❌ Max reconnection attempts reached. Giving up.');
      return;
    }

    const delay = this.reconnectDelays[this.reconnectAttempts] || 16000;
    console.log(`[Meeting Minutes] Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts + 1}/${this.maxReconnectAttempts})...`);

    setTimeout(() => {
      this.reconnectAttempts++;
      this.currentPort = this.portRange.start; // Reset port scan
      this.connect();
    }, delay);
  }

  /**
   * Handle incoming WebSocket message
   */
  handleMessage(data) {
    try {
      const message = JSON.parse(data);
      
      console.log('[Meeting Minutes] Received message:', message);

      // Handle different message types
      switch (message.type) {
        case 'connected':
          // Fields are in camelCase from Rust WebSocket server
          console.log(`[Meeting Minutes] ✅ Connection established - Session: ${message.sessionId}`);
          break;

        case 'transcription':
          console.log(`[Meeting Minutes] 📝 Transcription: ${message.text}`);
          // TODO: Display in UI (MVP1+)
          break;

        case 'error':
          console.error(`[Meeting Minutes] ❌ Error: ${message.message}`);
          break;

        default:
          console.warn(`[Meeting Minutes] ⚠️ Unknown message type: ${message.type}`);
      }

    } catch (error) {
      console.error('[Meeting Minutes] Failed to parse message:', error, 'Data:', data);
    }
  }

  /**
   * Send message to WebSocket server
   */
  send(message) {
    if (this.connected && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    } else {
      console.error('[Meeting Minutes] Cannot send message - WebSocket not connected');
    }
  }

  /**
   * Close WebSocket connection
   */
  close() {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
      this.connected = false;
    }
  }
}

// Initialize WebSocket client when content script loads
const wsClient = new WebSocketClient();

// Start connection when on Google Meet
if (window.location.hostname === 'meet.google.com') {
  console.log('[Meeting Minutes] Content script loaded on Google Meet');
  wsClient.connect();
}

// Listen for messages from service worker (if needed in future)
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  console.log('[Meeting Minutes] Message from service worker:', message);
  
  if (message.action === 'reconnect') {
    wsClient.connect();
  }
  
  sendResponse({ status: 'ok' });
});
