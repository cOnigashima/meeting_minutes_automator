/**
 * Background Service Worker (Manifest V3)
 * Walking Skeleton (MVP0) - Empty Implementation
 */

console.log('Meeting Minutes Automator - Background Service Worker started');

/**
 * Handle extension installation
 */
chrome.runtime.onInstalled.addListener(() => {
  console.log('Extension installed');
});

/**
 * Handle messages from content scripts
 * Service Worker acts as a message relay only (temporary, can be killed after 30s)
 */
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  console.log('Message received in background:', message);
  // To be implemented in subsequent tasks
  return true; // Keep channel open for async response
});
