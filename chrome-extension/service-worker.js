// Service Worker: Lightweight message relay
// Walking Skeleton (MVP0) - Manifest V3 compliant

console.log('[Meeting Minutes] Service worker loaded');

// Service worker is intentionally lightweight due to Manifest V3 30-second idle timeout
// All WebSocket connection logic is in content-script.js for persistence

// Handle extension installation
chrome.runtime.onInstalled.addListener((details) => {
  console.log('[Meeting Minutes] Extension installed:', details.reason);
  
  if (details.reason === 'install') {
    console.log('[Meeting Minutes] First install - ready to use on Google Meet');
  }
});

// Handle messages from content scripts (future use)
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  console.log('[Meeting Minutes] Message received in service worker:', message);
  
  // Future: Handle commands from popup/content scripts
  switch (message.action) {
    case 'ping':
      sendResponse({ status: 'pong' });
      break;
      
    default:
      console.log('[Meeting Minutes] Unknown action:', message.action);
      sendResponse({ status: 'unknown' });
  }
  
  return true; // Keep channel open for async response
});

// Keep service worker description minimal to avoid unnecessary wake-ups
console.log('[Meeting Minutes] Service worker initialization complete');
