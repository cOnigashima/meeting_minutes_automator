# Meeting Minutes STT ADR Evolution (2025-10)

## Timeline
- **2025-10-13 – ADR-008 (Dedicated Session Task)**  
  Proposed a single recording-session task to resolve IPC deadlocks. Rejected after external review uncovered three blockers: structural deadlock, false `no_speech`, and frame drops.
- **2025-10-13 – ADR-009 (Sender/Receiver Concurrent Architecture)**  
  Introduced split sender/receiver tasks, but shared mutexes serialized the work and `blocking_send()` risked halting CPAL. Rejected in favor of deeper fixes.
- **2025-10-13 – ADR-010 (External Review Evaluation v2)**  
  Captured the independent audit that invalidated ADR-008 and validated the need for true full-duplex IPC.
- **2025-10-13 – ADR-011 (IPC Stdin/Stdout Mutex Separation)**  
  Proposed isolating stdin/stdout locks so send and receive could progress independently; positioned as part 1 of the replacement plan for ADR-009.
- **2025-10-13 – ADR-012 (Audio Callback Backpressure Redesign)**  
  Complemented ADR-011 by replacing `blocking_send()` with non-blocking buffering (or timeouts) to protect the audio callback thread; designated as part 2 of the plan.
- **2025-10-14 – ADR-013 (Sidecar Full-Duplex IPC Final Design)**  
  Approved consolidation of ADR-011/012 into a finalized architecture with a Sidecar facade, line-delimited JSON framing, and a clarified buffer policy.

## Status Relationships
- ADR-008 → rejected by ADR-009 findings and external review.
- ADR-009 → rejected because its mutex/backpressure approach still caused blocking; replaced by ADR-011 + ADR-012.
- ADR-011 + ADR-012 → both superseded by ADR-013 once the facade, framing, and buffer policy were specified.

## Key Lessons
- Split design work between concurrency (IPC separation) and realtime safety (audio backpressure), then converge on a single façade.
- External audits (ADR-010) are documented as part of the ADR stream to clarify why proposals were rejected.
- Finalized ADRs should bake in operational policies (buffer sizing, framing) to avoid ambiguity when superseding earlier drafts.
