import { describe, it, expect } from 'vitest';
import { SyncStateMachine } from '@/sync/SyncStateMachine';

describe('SyncStateMachine', () => {
  describe('getCurrentState', () => {
    it('should return NotStarted by default', () => {
      const machine = new SyncStateMachine();
      expect(machine.getCurrentState()).toBe('NotStarted');
    });

    it('should return current state after transition', () => {
      const machine = new SyncStateMachine();
      const result = machine.transition('Syncing');
      expect(result.ok).toBe(true);
      expect(machine.getCurrentState()).toBe('Syncing');
    });
  });

  describe('transition', () => {
    it('should allow NotStarted -> Syncing', () => {
      const machine = new SyncStateMachine();
      const result = machine.transition('Syncing');
      expect(result.ok).toBe(true);
    });

    it('should allow Syncing -> Paused', () => {
      const machine = new SyncStateMachine();
      machine.transition('Syncing');
      const result = machine.transition('Paused');
      expect(result.ok).toBe(true);
    });

    it('should allow Paused -> Syncing', () => {
      const machine = new SyncStateMachine();
      machine.transition('Syncing');
      machine.transition('Paused');
      const result = machine.transition('Syncing');
      expect(result.ok).toBe(true);
    });

    it('should allow Syncing -> Error', () => {
      const machine = new SyncStateMachine();
      machine.transition('Syncing');
      const result = machine.transition('Error');
      expect(result.ok).toBe(true);
    });

    it('should return InvalidTransitionError for invalid transitions', () => {
      const machine = new SyncStateMachine();
      const result = machine.transition('Paused');
      expect(result.ok).toBe(false);
      if (!result.ok) {
        expect(result.error.type).toBe('InvalidTransition');
        expect(result.error.from).toBe('NotStarted');
        expect(result.error.to).toBe('Paused');
      }
    });
  });

  describe('reset', () => {
    it('should reset state to NotStarted', () => {
      const machine = new SyncStateMachine();
      machine.transition('Syncing');
      machine.reset();
      expect(machine.getCurrentState()).toBe('NotStarted');
    });
  });
});
