/**
 * Spike Loader for DevTools Console Execution
 *
 * Purpose: Load oauth-docs-spike.ts in a popup context (window exists)
 * and expose runSpike() to DevTools Console for manual testing.
 *
 * Usage:
 * 1. Load extension in Chrome
 * 2. Click extension icon to open popup
 * 3. Right-click popup → Inspect → Open DevTools Console
 * 4. Run: runSpike()
 */

import { runSpike } from '../spike/oauth-docs-spike';

// Expose to DevTools Console
(window as any).runSpike = runSpike;

console.log('✅ Spike loaded. Run: runSpike()');
console.log('Prerequisites:');
console.log('1. Set GOOGLE_CLIENT_ID in oauth-docs-spike.ts');
console.log('2. Create Google Cloud Console project (Desktop app type)');
console.log('3. Add redirect URI: chrome.identity.getRedirectURL()');
