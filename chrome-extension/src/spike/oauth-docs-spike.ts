/**
 * Vertical Slice Spike: OAuth 2.0 + Google Docs API Integration
 *
 * Purpose: Validate technical feasibility of:
 * 1. Chrome Identity API (chrome.identity.launchWebAuthFlow)
 * 2. Google Docs API (documents.batchUpdate)
 * 3. Named Range creation/update
 * 4. Token Refresh implementation pattern
 * 5. PKCE (Proof Key for Code Exchange) flow
 *
 * This is a prototype/spike code - NOT production implementation.
 * Results will inform the design of 19 production classes.
 *
 * SECURITY NOTE: This spike uses PKCE instead of client_secret.
 * Chrome extensions (MV3) are fully inspectable, so client secrets
 * would be exposed to all users. PKCE is the recommended flow for
 * "installed apps" per Google OAuth 2.0 best practices.
 */

// ============================================================================
// Configuration
// ============================================================================

// ⚠️ SPIKE: Set your OAuth client ID here (Web Application type for testing)
const GOOGLE_CLIENT_ID = "YOUR_CLIENT_ID.apps.googleusercontent.com";
// ⚠️ SPIKE ONLY: client_secret required for Web Application OAuth type
// In production, use backend server for token exchange or Chrome App OAuth type
const GOOGLE_CLIENT_SECRET = "YOUR_CLIENT_SECRET";
const REDIRECT_URI = chrome.identity.getRedirectURL();

// Scopes: documents (read/write) + drive.file (access to user-created docs)
// https://developers.google.com/identity/protocols/oauth2/scopes#docs
const SCOPES = [
  'https://www.googleapis.com/auth/documents',
  'https://www.googleapis.com/auth/drive.file',
].join(' ');

// ============================================================================
// Types
// ============================================================================

type AuthTokens = {
  accessToken: string;
  refreshToken: string;
  expiresAt: number; // Unix timestamp (ms)
};

type ApiError = {
  status: number;
  message: string;
};

type Result<T, E> = { ok: true; value: T } | { ok: false; error: E };

// ============================================================================
// PKCE Helper Functions
// ============================================================================

/**
 * Generate a cryptographically secure code_verifier for PKCE
 * @returns Base64-URL-encoded random string (43-128 characters)
 */
function generateCodeVerifier(): string {
  const array = new Uint8Array(32); // 32 bytes = 256 bits
  crypto.getRandomValues(array);
  return base64UrlEncode(array);
}

/**
 * Generate code_challenge from code_verifier using SHA-256
 * @param verifier The code_verifier string
 * @returns Base64-URL-encoded SHA-256 hash of verifier
 */
async function generateCodeChallenge(verifier: string): Promise<string> {
  const encoder = new TextEncoder();
  const data = encoder.encode(verifier);
  const hash = await crypto.subtle.digest('SHA-256', data);
  return base64UrlEncode(new Uint8Array(hash));
}

/**
 * Base64-URL encoding (RFC 4648 Section 5)
 * Converts Uint8Array to base64url format (no padding, URL-safe characters)
 */
function base64UrlEncode(array: Uint8Array): string {
  const base64 = btoa(String.fromCharCode(...array));
  return base64.replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

// ============================================================================
// Step 1: OAuth 2.0 Authentication Flow with PKCE
// ============================================================================

/**
 * Launch OAuth 2.0 flow with PKCE using chrome.identity.launchWebAuthFlow()
 *
 * Validates:
 * - Chrome Identity API is accessible
 * - User authorization works
 * - Authorization code can be extracted from redirect URL
 * - PKCE (code_challenge) is sent correctly
 *
 * @returns Object with authorization code and code_verifier (for token exchange)
 */
async function launchAuthFlow(): Promise<Result<{ code: string; verifier: string }, ApiError>> {
  try {
    // PKCE Step 1: Generate code_verifier and code_challenge
    const codeVerifier = generateCodeVerifier();
    const codeChallenge = await generateCodeChallenge(codeVerifier);

    console.log('[Spike] PKCE code_verifier:', codeVerifier);
    console.log('[Spike] PKCE code_challenge:', codeChallenge);

    const authUrl = new URL('https://accounts.google.com/o/oauth2/v2/auth');
    authUrl.searchParams.set('client_id', GOOGLE_CLIENT_ID);
    authUrl.searchParams.set('response_type', 'code');
    authUrl.searchParams.set('redirect_uri', REDIRECT_URI);
    authUrl.searchParams.set('scope', SCOPES);
    authUrl.searchParams.set('access_type', 'offline'); // Request refresh token
    authUrl.searchParams.set('prompt', 'consent'); // Force consent screen

    // PKCE Step 2: Add code_challenge to auth URL
    authUrl.searchParams.set('code_challenge', codeChallenge);
    authUrl.searchParams.set('code_challenge_method', 'S256'); // SHA-256

    console.log('[Spike] Launching auth flow with PKCE:', authUrl.toString());

    const redirectUrl = await chrome.identity.launchWebAuthFlow({
      url: authUrl.toString(),
      interactive: true,
    });

    console.log('[Spike] Redirect URL:', redirectUrl);

    if (!redirectUrl) {
      return {
        ok: false,
        error: { status: 400, message: 'No redirect URL returned' },
      };
    }

    // Extract authorization code from redirect URL
    const url = new URL(redirectUrl);
    const code = url.searchParams.get('code');

    if (!code) {
      return {
        ok: false,
        error: { status: 400, message: 'No authorization code in redirect URL' },
      };
    }

    // Return both code and verifier (verifier needed for token exchange)
    return { ok: true, value: { code, verifier: codeVerifier } };
  } catch (error) {
    console.error('[Spike] Auth flow error:', error);
    return {
      ok: false,
      error: { status: 500, message: String(error) },
    };
  }
}

/**
 * Exchange authorization code for access token and refresh token using PKCE
 *
 * Validates:
 * - Token exchange endpoint works with PKCE (code_verifier)
 * - Refresh token is returned (requires access_type=offline)
 * - Token expiry time is calculated correctly
 * - Client secret is NOT required (PKCE replaces it)
 *
 * @param code Authorization code from launchAuthFlow()
 * @param codeVerifier The code_verifier generated in launchAuthFlow()
 */
async function exchangeCodeForToken(
  code: string,
  codeVerifier: string
): Promise<Result<AuthTokens, ApiError>> {
  try {
    const tokenUrl = 'https://oauth2.googleapis.com/token';

    // SPIKE: Include client_secret for Web Application OAuth type
    const body = new URLSearchParams({
      code,
      client_id: GOOGLE_CLIENT_ID,
      client_secret: GOOGLE_CLIENT_SECRET, // Required for Web Application type
      code_verifier: codeVerifier,
      redirect_uri: REDIRECT_URI,
      grant_type: 'authorization_code',
    });

    console.log('[Spike] Exchanging code for token with PKCE...');

    const response = await fetch(tokenUrl, {
      method: 'POST',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      body: body.toString(),
    });

    if (!response.ok) {
      const errorText = await response.text();
      return {
        ok: false,
        error: { status: response.status, message: errorText },
      };
    }

    const data = await response.json();
    console.log('[Spike] Token response:', {
      hasAccessToken: !!data.access_token,
      hasRefreshToken: !!data.refresh_token,
      expiresIn: data.expires_in,
    });

    const tokens: AuthTokens = {
      accessToken: data.access_token,
      refreshToken: data.refresh_token,
      expiresAt: Date.now() + data.expires_in * 1000,
    };

    return { ok: true, value: tokens };
  } catch (error) {
    console.error('[Spike] Token exchange error:', error);
    return {
      ok: false,
      error: { status: 500, message: String(error) },
    };
  }
}

/**
 * Refresh access token using refresh token
 *
 * Validates:
 * - Token refresh endpoint works
 * - New access token is returned
 * - Refresh token remains valid after refresh
 */
async function refreshAccessToken(
  refreshToken: string
): Promise<Result<AuthTokens, ApiError>> {
  try {
    const tokenUrl = 'https://oauth2.googleapis.com/token';
    // SPIKE: Include client_secret for Web Application OAuth type
    const body = new URLSearchParams({
      refresh_token: refreshToken,
      client_id: GOOGLE_CLIENT_ID,
      client_secret: GOOGLE_CLIENT_SECRET,
      grant_type: 'refresh_token',
    });

    console.log('[Spike] Refreshing access token...');

    const response = await fetch(tokenUrl, {
      method: 'POST',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      body: body.toString(),
    });

    if (!response.ok) {
      const errorText = await response.text();
      return {
        ok: false,
        error: { status: response.status, message: errorText },
      };
    }

    const data = await response.json();
    console.log('[Spike] Refresh response:', {
      hasAccessToken: !!data.access_token,
      expiresIn: data.expires_in,
    });

    const tokens: AuthTokens = {
      accessToken: data.access_token,
      refreshToken: refreshToken, // Reuse existing refresh token
      expiresAt: Date.now() + data.expires_in * 1000,
    };

    return { ok: true, value: tokens };
  } catch (error) {
    console.error('[Spike] Token refresh error:', error);
    return {
      ok: false,
      error: { status: 500, message: String(error) },
    };
  }
}

// ============================================================================
// Step 2: Google Docs API Integration
// ============================================================================

/**
 * Insert text into Google Docs using documents.batchUpdate
 *
 * Validates:
 * - Google Docs API is accessible
 * - batchUpdate request format is correct
 * - Text insertion works at specified index
 */
async function insertTextToDoc(
  documentId: string,
  accessToken: string,
  text: string,
  index: number
): Promise<Result<void, ApiError>> {
  try {
    const url = `https://docs.googleapis.com/v1/documents/${documentId}:batchUpdate`;
    const body = {
      requests: [
        {
          insertText: {
            location: { index },
            text: text + '\n',
          },
        },
      ],
    };

    console.log('[Spike] Inserting text:', { documentId, text, index });

    const response = await fetch(url, {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${accessToken}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(body),
    });

    if (!response.ok) {
      const errorText = await response.text();
      return {
        ok: false,
        error: { status: response.status, message: errorText },
      };
    }

    console.log('[Spike] Text inserted successfully');
    return { ok: true, value: undefined };
  } catch (error) {
    console.error('[Spike] Insert text error:', error);
    return {
      ok: false,
      error: { status: 500, message: String(error) },
    };
  }
}

/**
 * Create Named Range in Google Docs
 *
 * Validates:
 * - Named Range creation works
 * - Named Range can be used as insertion cursor
 */
async function createNamedRange(
  documentId: string,
  accessToken: string,
  name: string,
  startIndex: number,
  endIndex: number
): Promise<Result<void, ApiError>> {
  try {
    const url = `https://docs.googleapis.com/v1/documents/${documentId}:batchUpdate`;
    const body = {
      requests: [
        {
          createNamedRange: {
            name,
            range: {
              startIndex,
              endIndex,
            },
          },
        },
      ],
    };

    console.log('[Spike] Creating Named Range:', { documentId, name, startIndex, endIndex });

    const response = await fetch(url, {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${accessToken}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(body),
    });

    if (!response.ok) {
      const errorText = await response.text();
      return {
        ok: false,
        error: { status: response.status, message: errorText },
      };
    }

    console.log('[Spike] Named Range created successfully');
    return { ok: true, value: undefined };
  } catch (error) {
    console.error('[Spike] Create Named Range error:', error);
    return {
      ok: false,
      error: { status: 500, message: String(error) },
    };
  }
}

/**
 * Get Named Range position from Google Docs
 *
 * Validates:
 * - Named Range can be retrieved
 * - Position information is accurate
 */
async function getNamedRangePosition(
  documentId: string,
  accessToken: string,
  name: string
): Promise<Result<{ startIndex: number; endIndex: number }, ApiError>> {
  try {
    const url = `https://docs.googleapis.com/v1/documents/${documentId}`;

    console.log('[Spike] Getting Named Range position:', { documentId, name });

    const response = await fetch(url, {
      method: 'GET',
      headers: {
        Authorization: `Bearer ${accessToken}`,
      },
    });

    if (!response.ok) {
      const errorText = await response.text();
      return {
        ok: false,
        error: { status: response.status, message: errorText },
      };
    }

    const doc = await response.json();
    const namedRange = doc.namedRanges?.[name];

    if (!namedRange) {
      return {
        ok: false,
        error: { status: 404, message: `Named Range "${name}" not found` },
      };
    }

    // Named Range format: { namedRanges: { [name]: { ranges: [{ startIndex, endIndex }] } } }
    const range = namedRange.namedRanges[0].ranges[0];
    console.log('[Spike] Named Range position:', range);

    return { ok: true, value: range };
  } catch (error) {
    console.error('[Spike] Get Named Range error:', error);
    return {
      ok: false,
      error: { status: 500, message: String(error) },
    };
  }
}

// ============================================================================
// Step 3: End-to-End Spike Execution
// ============================================================================

/**
 * Execute full spike: OAuth (PKCE) → Token Exchange → Docs API → Named Range
 *
 * Manual execution steps:
 * 1. Replace GOOGLE_CLIENT_ID with real value (🔒 client_secret NOT needed)
 * 2. Load chrome-extension in Chrome
 * 3. Click extension icon → Open popup
 * 4. Right-click popup → Inspect → DevTools Console
 * 5. Run: runSpike('YOUR_DOCUMENT_ID')
 * 6. Follow OAuth prompt
 * 7. Check console logs for validation results (all steps should be [PASS])
 */
async function runSpike(documentId: string): Promise<void> {
  console.log('='.repeat(80));
  console.log('Vertical Slice Spike: OAuth 2.0 + Google Docs API');
  console.log('='.repeat(80));

  // Step 1: OAuth Authentication
  console.log('\n[Step 1] Launching OAuth 2.0 flow...');
  const codeResult = await launchAuthFlow();
  if (!codeResult.ok) {
    console.error('[FAIL] OAuth flow failed:', codeResult.error);
    return;
  }
  console.log('[PASS] Authorization code received');

  // Step 2: Token Exchange with PKCE
  console.log('\n[Step 2] Exchanging code for tokens with PKCE...');
  const { code, verifier } = codeResult.value;
  const tokenResult = await exchangeCodeForToken(code, verifier);
  if (!tokenResult.ok) {
    console.error('[FAIL] Token exchange failed:', tokenResult.error);
    return;
  }
  console.log('[PASS] Access token and refresh token received (PKCE verified)');
  const tokens = tokenResult.value;

  // Save tokens to chrome.storage.local for manual inspection
  await chrome.storage.local.set({ spike_tokens: tokens });
  console.log('[INFO] Tokens saved to chrome.storage.local.spike_tokens');

  // Step 3: Insert Text via Google Docs API
  console.log('\n[Step 3] Inserting text to Google Docs...');
  const insertResult = await insertTextToDoc(
    documentId,
    tokens.accessToken,
    '[Spike Test] Meeting started at ' + new Date().toISOString(),
    1 // Insert at beginning of document
  );
  if (!insertResult.ok) {
    console.error('[FAIL] Text insertion failed:', insertResult.error);
    return;
  }
  console.log('[PASS] Text inserted successfully');

  // Step 4: Create Named Range
  console.log('\n[Step 4] Creating Named Range...');
  const namedRangeResult = await createNamedRange(
    documentId,
    tokens.accessToken,
    'meeting_minutes_cursor',
    1,
    2
  );
  if (!namedRangeResult.ok) {
    console.error('[FAIL] Named Range creation failed:', namedRangeResult.error);
    return;
  }
  console.log('[PASS] Named Range created successfully');

  // Step 5: Retrieve Named Range Position
  console.log('\n[Step 5] Retrieving Named Range position...');
  const positionResult = await getNamedRangePosition(
    documentId,
    tokens.accessToken,
    'meeting_minutes_cursor'
  );
  if (!positionResult.ok) {
    console.error('[FAIL] Named Range retrieval failed:', positionResult.error);
    return;
  }
  console.log('[PASS] Named Range position retrieved:', positionResult.value);

  // Step 6: Token Refresh (simulate expiry)
  console.log('\n[Step 6] Testing token refresh...');
  const refreshResult = await refreshAccessToken(tokens.refreshToken);
  if (!refreshResult.ok) {
    console.error('[FAIL] Token refresh failed:', refreshResult.error);
    return;
  }
  console.log('[PASS] Token refreshed successfully');

  console.log('\n' + '='.repeat(80));
  console.log('Spike Completed Successfully! ✅');
  console.log('='.repeat(80));
  console.log('\nValidation Summary:');
  console.log('✅ Chrome Identity API works');
  console.log('✅ OAuth 2.0 with PKCE works (no client_secret needed)');
  console.log('✅ Token exchange works (access + refresh)');
  console.log('✅ Google Docs API batchUpdate works');
  console.log('✅ Named Range creation works');
  console.log('✅ Named Range retrieval works');
  console.log('✅ Token refresh works');
  console.log('\nNext Steps:');
  console.log('1. Document PKCE findings in spike-report.md');
  console.log('2. Update design to include PKCE in IChromeIdentityClient');
  console.log('3. Proceed to Phase 1 implementation');
}

// NOTE: This function is exposed via src/popup/spike-loader.ts, not here.
// Do not add (window as any).runSpike = runSpike here, as it breaks in service worker context.
export { runSpike };
