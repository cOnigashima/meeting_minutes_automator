/**
 * Reconnection Manager with Exponential Backoff
 *
 * Handles WebSocket reconnection with:
 * - Exponential backoff (500ms initial, 30s max)
 * - Jitter for collision avoidance
 * - Attempt count tracking
 *
 * Implementation: Smart Polling for Chrome Extension <-> Tauri connection
 */

export class ReconnectionManager {
  // Backoff parameters
  private readonly INITIAL_DELAY_MS = 500;
  private readonly MAX_DELAY_MS = 30000;
  private readonly BACKOFF_MULTIPLIER = 2.0;
  private readonly JITTER_FACTOR = 0.3; // ±15%

  // State
  private attemptCount = 0;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;

  /**
   * Calculate backoff delay with exponential increase and jitter
   *
   * Formula: min(INITIAL_DELAY * 2^attempt, MAX_DELAY) ± jitter
   *
   * Delay sequence (approximate):
   * - Attempt 0: 500ms
   * - Attempt 1: 1000ms
   * - Attempt 2: 2000ms
   * - Attempt 3: 4000ms
   * - Attempt 4: 8000ms
   * - Attempt 5: 16000ms
   * - Attempt 6+: 30000ms (capped)
   */
  calculateBackoffDelay(): number {
    const baseDelay = this.INITIAL_DELAY_MS * Math.pow(this.BACKOFF_MULTIPLIER, this.attemptCount);
    const clampedDelay = Math.min(baseDelay, this.MAX_DELAY_MS);

    // Apply jitter: ±15% (JITTER_FACTOR = 0.3, so random - 0.5 gives -0.15 to +0.15)
    const jitter = clampedDelay * this.JITTER_FACTOR * (Math.random() - 0.5);

    return Math.floor(clampedDelay + jitter);
  }

  /**
   * Reset attempt count (call on successful connection)
   */
  resetAttemptCount(): void {
    this.attemptCount = 0;
  }

  /**
   * Get current attempt count
   */
  getAttemptCount(): number {
    return this.attemptCount;
  }

  /**
   * Check if reconnect is scheduled
   */
  isReconnectScheduled(): boolean {
    return this.reconnectTimer !== null;
  }

  /**
   * Schedule a reconnection attempt with exponential backoff
   *
   * @param callback Function to call when timer fires
   */
  scheduleReconnect(callback: () => void): void {
    if (this.reconnectTimer) {
      return;
    }

    const delay = this.calculateBackoffDelay();
    console.log(`[ReconnectionManager] Scheduling reconnect in ${delay}ms (attempt #${this.attemptCount + 1})`);

    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      this.attemptCount++;
      callback();
    }, delay);
  }

  /**
   * Cancel any scheduled reconnection
   */
  cancelReconnect(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
  }

  /**
   * Reset manager state (cancel timer and reset count)
   */
  reset(): void {
    this.cancelReconnect();
    this.resetAttemptCount();
  }
}
