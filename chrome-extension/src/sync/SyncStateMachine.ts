/**
 * 同期状態遷移管理実装
 *
 * Implementation: Phase 3
 */

import type { Result } from '@/types/Result';
import type { SyncState, InvalidTransitionError } from '@/types/SyncTypes';
import type { ISyncStateMachine } from './ISyncStateMachine';
import { ok, err } from '@/types/Result';

export class SyncStateMachine implements ISyncStateMachine {
  private currentState: SyncState = 'NotStarted';

  getCurrentState(): SyncState {
    return this.currentState;
  }

  transition(toState: SyncState): Result<void, InvalidTransitionError> {
    const fromState = this.currentState;
    if (fromState === toState) {
      return ok(undefined);
    }

    const validTransitions: Record<SyncState, SyncState[]> = {
      NotStarted: ['Syncing'],
      Syncing: ['Paused', 'Error'],
      Paused: ['Syncing'],
      Error: ['Syncing'],
    };

    if (!validTransitions[fromState].includes(toState)) {
      return err({
        type: 'InvalidTransition',
        from: fromState,
        to: toState,
        message: `Invalid transition: ${fromState} -> ${toState}`,
      });
    }

    this.currentState = toState;
    return ok(undefined);
  }

  reset(): void {
    this.currentState = 'NotStarted';
  }
}
