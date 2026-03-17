// ======================================================================
// Offline Queue, Optimistic Updates & Background Sync (TAP-11)
//
// Every UI mutation is instant. Network writes are async. Offline?
// The queue persists and flushes on reconnect. The user NEVER waits
// for the network.
//
// All code below was developed via strict RED/GREEN TDD:
//   1. RED  -- failing test written first
//   2. GREEN -- minimum production code to pass
//   3. REFACTOR -- clean up, keep all tests green
// ======================================================================

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

// ── Mutation Operation ──

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MutationOp {
    Insert,
    Upsert,
}

// ── PendingMutation ──

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingMutation {
    pub id: Uuid,
    pub table: String,
    pub operation: MutationOp,
    pub payload: Value,
    pub created_at: DateTime<Utc>,
    pub retry_count: u32,
}

impl PendingMutation {
    pub fn new(table: &str, operation: MutationOp, payload: Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            table: table.to_string(),
            operation,
            payload,
            created_at: Utc::now(),
            retry_count: 0,
        }
    }
}

// ── Offline Queue ──

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfflineQueue {
    items: Vec<PendingMutation>,
}

impl OfflineQueue {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn enqueue(&mut self, mutation: PendingMutation) {
        self.items.push(mutation);
    }

    pub fn dequeue(&mut self) -> Option<PendingMutation> {
        if self.items.is_empty() {
            None
        } else {
            Some(self.items.remove(0))
        }
    }

    pub fn peek(&self) -> Option<&PendingMutation> {
        self.items.first()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    // ── Persistence ──

    pub fn save_to_json(&self) -> String {
        serde_json::to_string(&self.items)
            .unwrap_or_else(|_| "[]".to_string())
    }

    pub fn load_from_json(json: &str) -> Self {
        match serde_json::from_str::<Vec<PendingMutation>>(json) {
            Ok(items) => Self { items },
            Err(e) => {
                log::warn!(
                    "Failed to deserialize offline queue, returning empty: {}",
                    e
                );
                Self::new()
            }
        }
    }

    // ── Flush ──

    /// Process all queued mutations in FIFO order using the provided sender.
    /// Stops on first failure, keeping remaining items.
    /// Returns (sent_count, failed: bool).
    pub fn flush<F>(&mut self, mut sender_fn: F) -> (usize, bool)
    where
        F: FnMut(&PendingMutation) -> Result<(), String>,
    {
        let mut sent = 0usize;

        while !self.items.is_empty() {
            match sender_fn(&self.items[0]) {
                Ok(()) => {
                    self.items.remove(0);
                    sent += 1;
                }
                Err(_) => {
                    self.items[0].retry_count += 1;
                    return (sent, true);
                }
            }
        }

        (sent, false)
    }
}

// ── Optimistic Update Helpers ──

/// Represents the result of an optimistic update that can be rolled back.
#[derive(Debug)]
pub struct OptimisticHandle<T: Clone> {
    pub previous: T,
    pub mutation: PendingMutation,
}

/// Apply an optimistic update: captures the old value, sets the new value,
/// queues a mutation, and returns a handle for potential rollback.
pub fn optimistic_update<T: Clone>(
    current: &mut T,
    new_value: T,
    mutation: PendingMutation,
    queue: &mut OfflineQueue,
) -> OptimisticHandle<T> {
    let previous = current.clone();
    *current = new_value;
    queue.enqueue(mutation);
    OptimisticHandle { previous, mutation: queue.items.last().unwrap().clone() }
}

/// On network failure: rollback signal to previous value and return a toast.
pub fn rollback<T: Clone>(current: &mut T, handle: &OptimisticHandle<T>) -> String {
    *current = handle.previous.clone();
    "Sync failed. Your progress will retry automatically.".to_string()
}

// ── Conflict Resolution ──

/// Resolve a conflict between local and server values for a given table/field.
/// Returns the winning value.
pub fn resolve_conflict(table: &str, field: &str, local: &Value, server: &Value) -> Value {
    match table {
        "user_state" => match field {
            "total_xp" | "current_streak" | "longest_streak" => {
                // Max-value wins
                let local_num = local.as_i64().unwrap_or(0);
                let server_num = server.as_i64().unwrap_or(0);
                Value::from(local_num.max(server_num))
            }
            _ => server.clone(),
        },
        // streak_log: idempotent (PK = user_id + day), no conflict
        "streak_log" => local.clone(),
        // challenge_attempts: append-only, no conflict
        "challenge_attempts" => local.clone(),
        _ => server.clone(),
    }
}

// ── User Communication ──

/// Connectivity status tracker.
#[derive(Debug, Clone, PartialEq)]
pub struct ConnectivityState {
    pub is_online: bool,
    pub banner_message: Option<String>,
}

impl ConnectivityState {
    pub fn new() -> Self {
        Self {
            is_online: true,
            banner_message: None,
        }
    }

    pub fn set_offline(&mut self) {
        self.is_online = false;
        self.banner_message =
            Some("Offline \u{2014} progress will sync when you reconnect".to_string());
    }

    pub fn set_online(&mut self) {
        self.is_online = true;
        self.banner_message = None;
    }
}

impl Default for ConnectivityState {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if the oldest item in the queue is older than 24 hours.
/// Returns true if the queue has stale items requiring user notification.
pub fn stale_queue_check(queue: &OfflineQueue) -> bool {
    match queue.peek() {
        Some(oldest) => {
            let age = Utc::now() - oldest.created_at;
            age.num_hours() >= 24
        }
        None => false,
    }
}

/// Get a stale queue notification message if applicable.
pub fn stale_queue_message(queue: &OfflineQueue) -> Option<String> {
    if stale_queue_check(queue) {
        Some(
            "Some progress hasn't synced yet. We'll retry automatically."
                .to_string(),
        )
    } else {
        None
    }
}

// ======================================================================
// Tests -- 22 TDD scenarios across 6 groups (TAP-11)
// ======================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use serde_json::json;

    // ── Helpers ──

    fn make_mutation(table: &str, op: MutationOp, payload: Value) -> PendingMutation {
        PendingMutation::new(table, op, payload)
    }

    fn make_xp_mutation(xp: u32) -> PendingMutation {
        make_mutation(
            "user_state",
            MutationOp::Upsert,
            json!({ "total_xp": xp }),
        )
    }

    // ════════════════════════════════════════════
    // R1: PendingMutation struct + queue
    // ════════════════════════════════════════════

    #[test]
    fn r1_1_mutation_struct_serializes() {
        let m = make_xp_mutation(100);
        let json_str = serde_json::to_string(&m).expect("should serialize");
        let restored: PendingMutation =
            serde_json::from_str(&json_str).expect("should deserialize");
        assert_eq!(m.id, restored.id);
        assert_eq!(m.table, restored.table);
        assert_eq!(m.operation, restored.operation);
        assert_eq!(m.payload, restored.payload);
        assert_eq!(m.created_at, restored.created_at);
        assert_eq!(m.retry_count, restored.retry_count);
    }

    #[test]
    fn r1_2_enqueue_mutation() {
        let mut queue = OfflineQueue::new();
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);

        queue.enqueue(make_xp_mutation(20));
        assert_eq!(queue.len(), 1);
        assert!(!queue.is_empty());
    }

    #[test]
    fn r1_3_queue_fifo_order() {
        let mut queue = OfflineQueue::new();
        let a = make_mutation("table_a", MutationOp::Insert, json!({"a": 1}));
        let b = make_mutation("table_b", MutationOp::Insert, json!({"b": 2}));
        let a_id = a.id;

        queue.enqueue(a);
        queue.enqueue(b);

        let peeked = queue.peek().expect("queue not empty");
        assert_eq!(peeked.id, a_id, "peek should return first-enqueued (A)");
    }

    #[test]
    fn r1_4_dequeue_removes_front() {
        let mut queue = OfflineQueue::new();
        let a = make_mutation("table_a", MutationOp::Insert, json!({"a": 1}));
        let b = make_mutation("table_b", MutationOp::Insert, json!({"b": 2}));
        let a_id = a.id;
        let b_id = b.id;

        queue.enqueue(a);
        queue.enqueue(b);

        let removed = queue.dequeue().expect("should return A");
        assert_eq!(removed.id, a_id);
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.peek().unwrap().id, b_id);
    }

    // ════════════════════════════════════════════
    // R2: Optimistic updates
    // ════════════════════════════════════════════

    #[test]
    fn r2_1_xp_signal_updates_before_network() {
        let mut xp: u32 = 100;
        let mut queue = OfflineQueue::new();
        let mutation = make_xp_mutation(120);

        let _handle = optimistic_update(&mut xp, 120, mutation, &mut queue);

        // XP updated immediately, mutation queued
        assert_eq!(xp, 120, "XP must update before any network call");
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn r2_2_network_success_keeps_value() {
        let mut xp: u32 = 100;
        let mut queue = OfflineQueue::new();
        let mutation = make_xp_mutation(120);

        let _handle = optimistic_update(&mut xp, 120, mutation, &mut queue);

        // Simulate network success: just dequeue
        queue.dequeue();

        assert_eq!(xp, 120, "Value stays at optimistic value after success");
        assert!(queue.is_empty());
    }

    #[test]
    fn r2_3_network_failure_rolls_back() {
        let mut xp: u32 = 100;
        let mut queue = OfflineQueue::new();
        let mutation = make_xp_mutation(120);

        let handle = optimistic_update(&mut xp, 120, mutation, &mut queue);
        assert_eq!(xp, 120);

        // Simulate network failure: rollback
        let _toast = rollback(&mut xp, &handle);

        assert_eq!(xp, 100, "XP must roll back to previous value on failure");
    }

    #[test]
    fn r2_4_rollback_shows_toast() {
        let mut xp: u32 = 100;
        let mut queue = OfflineQueue::new();
        let mutation = make_xp_mutation(120);

        let handle = optimistic_update(&mut xp, 120, mutation, &mut queue);

        let toast = rollback(&mut xp, &handle);

        assert!(
            !toast.is_empty(),
            "Toast message must be set on rollback"
        );
        assert!(
            toast.contains("retry"),
            "Toast should mention retry: got '{}'",
            toast
        );
    }

    #[test]
    fn r2_5_two_rapid_optimistic_updates() {
        let mut xp: u32 = 100;
        let mut queue = OfflineQueue::new();

        let m1 = make_xp_mutation(120);
        let _h1 = optimistic_update(&mut xp, 120, m1, &mut queue);
        assert_eq!(xp, 120);

        let m2 = make_xp_mutation(140);
        let _h2 = optimistic_update(&mut xp, 140, m2, &mut queue);
        assert_eq!(xp, 140);

        assert_eq!(queue.len(), 2, "Both mutations must be queued");
        // FIFO: first mutation is the 120 one
        assert_eq!(
            queue.peek().unwrap().payload["total_xp"],
            json!(120)
        );
    }

    // ════════════════════════════════════════════
    // R3: Persistence
    // ════════════════════════════════════════════

    #[test]
    fn r3_1_queue_serializes_to_json() {
        let mut queue = OfflineQueue::new();
        queue.enqueue(make_xp_mutation(10));
        queue.enqueue(make_xp_mutation(20));
        queue.enqueue(make_xp_mutation(30));

        let json_str = queue.save_to_json();
        let parsed: Value =
            serde_json::from_str(&json_str).expect("must produce valid JSON");

        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 3);
    }

    #[test]
    fn r3_2_queue_deserializes_from_json() {
        let mut queue = OfflineQueue::new();
        queue.enqueue(make_xp_mutation(10));
        queue.enqueue(make_xp_mutation(20));
        queue.enqueue(make_xp_mutation(30));

        let json_str = queue.save_to_json();
        let restored = OfflineQueue::load_from_json(&json_str);

        assert_eq!(restored.len(), 3);
    }

    #[test]
    fn r3_3_roundtrip_serialize_deserialize() {
        let mut queue = OfflineQueue::new();
        let m1 = make_xp_mutation(42);
        let m2 = make_mutation(
            "challenge_attempts",
            MutationOp::Insert,
            json!({"challenge_id": "rust-m1-c1", "correct": true}),
        );
        let m1_id = m1.id;
        let m2_id = m2.id;

        queue.enqueue(m1);
        queue.enqueue(m2);

        let json_str = queue.save_to_json();
        let restored = OfflineQueue::load_from_json(&json_str);

        assert_eq!(restored.len(), 2);
        assert_eq!(restored.peek().unwrap().id, m1_id);
        // Check second item
        let mut restored_clone = restored.clone();
        restored_clone.dequeue();
        assert_eq!(restored_clone.peek().unwrap().id, m2_id);
    }

    #[test]
    fn r3_4_corrupted_storage_returns_empty() {
        let garbage = "this is not valid json {{{[[[";
        let restored = OfflineQueue::load_from_json(garbage);
        assert!(
            restored.is_empty(),
            "Corrupted data should produce empty queue"
        );
    }

    #[test]
    fn r3_5_empty_storage_returns_empty() {
        // Empty string
        let restored = OfflineQueue::load_from_json("");
        assert!(restored.is_empty());

        // Valid empty array
        let restored2 = OfflineQueue::load_from_json("[]");
        assert!(restored2.is_empty());
    }

    // ════════════════════════════════════════════
    // R4: Flush on reconnect
    // ════════════════════════════════════════════

    #[test]
    fn r4_1_flush_sends_all_queued() {
        let mut queue = OfflineQueue::new();
        queue.enqueue(make_xp_mutation(10));
        queue.enqueue(make_xp_mutation(20));
        queue.enqueue(make_xp_mutation(30));

        let mut sent_payloads: Vec<Value> = Vec::new();
        let (sent, failed) = queue.flush(|m| {
            sent_payloads.push(m.payload.clone());
            Ok(())
        });

        assert_eq!(sent, 3);
        assert!(!failed);
        assert!(queue.is_empty());
        assert_eq!(sent_payloads.len(), 3);
    }

    #[test]
    fn r4_2_flush_fifo_order() {
        let mut queue = OfflineQueue::new();
        queue.enqueue(make_mutation("t", MutationOp::Insert, json!({"order": 1})));
        queue.enqueue(make_mutation("t", MutationOp::Insert, json!({"order": 2})));
        queue.enqueue(make_mutation("t", MutationOp::Insert, json!({"order": 3})));

        let mut order_seen: Vec<i64> = Vec::new();
        let (sent, _) = queue.flush(|m| {
            order_seen.push(m.payload["order"].as_i64().unwrap());
            Ok(())
        });

        assert_eq!(sent, 3);
        assert_eq!(order_seen, vec![1, 2, 3], "Must flush in FIFO order");
    }

    #[test]
    fn r4_3_partial_flush_failure() {
        let mut queue = OfflineQueue::new();
        let a = make_mutation("t", MutationOp::Insert, json!({"id": "A"}));
        let b = make_mutation("t", MutationOp::Insert, json!({"id": "B"}));
        let c = make_mutation("t", MutationOp::Insert, json!({"id": "C"}));
        let b_id = b.id;

        queue.enqueue(a);
        queue.enqueue(b);
        queue.enqueue(c);

        let mut call_count = 0u32;
        let (sent, failed) = queue.flush(|m| {
            call_count += 1;
            if m.payload["id"] == "B" {
                Err("network error".to_string())
            } else {
                Ok(())
            }
        });

        assert_eq!(sent, 1, "Only A should have been sent");
        assert!(failed, "Flush should report failure");
        assert_eq!(queue.len(), 2, "B and C should remain");
        assert_eq!(
            queue.peek().unwrap().id, b_id,
            "B should be at front"
        );
        assert_eq!(call_count, 2, "Should have attempted A and B only");
    }

    #[test]
    fn r4_4_retry_increments_count() {
        let mut queue = OfflineQueue::new();
        let m = make_xp_mutation(50);
        assert_eq!(m.retry_count, 0);
        queue.enqueue(m);

        // First failed flush
        let (_, _) = queue.flush(|_| Err("fail".to_string()));
        assert_eq!(
            queue.peek().unwrap().retry_count, 1,
            "retry_count should be 1 after first failure"
        );

        // Second failed flush
        let (_, _) = queue.flush(|_| Err("fail again".to_string()));
        assert_eq!(
            queue.peek().unwrap().retry_count, 2,
            "retry_count should be 2 after second failure"
        );
    }

    // ════════════════════════════════════════════
    // R5: Conflict resolution
    // ════════════════════════════════════════════

    #[test]
    fn r5_1_xp_max_value_local_wins() {
        let result = resolve_conflict(
            "user_state",
            "total_xp",
            &json!(120),
            &json!(100),
        );
        assert_eq!(result, json!(120));
    }

    #[test]
    fn r5_2_xp_max_value_server_wins() {
        let result = resolve_conflict(
            "user_state",
            "total_xp",
            &json!(100),
            &json!(120),
        );
        assert_eq!(result, json!(120));
    }

    #[test]
    fn r5_3_streak_max_value_wins() {
        let result = resolve_conflict(
            "user_state",
            "current_streak",
            &json!(5),
            &json!(3),
        );
        assert_eq!(result, json!(5));
    }

    #[test]
    fn r5_4_streak_log_idempotent() {
        let local = json!({"user_id": "abc", "day": "2026-03-17"});
        let server = json!({"user_id": "abc", "day": "2026-03-17"});
        let result = resolve_conflict("streak_log", "", &local, &server);
        // Idempotent: returns local, no error, duplicate insert is harmless
        assert_eq!(result, local);
    }

    #[test]
    fn r5_5_challenge_attempts_append_only() {
        let local = json!({"challenge_id": "rust-m1-c1", "correct": true, "attempt_num": 1});
        let server = json!({"challenge_id": "rust-m1-c1", "correct": false, "attempt_num": 2});
        // Append-only: both rows preserved, local insert proceeds
        let result = resolve_conflict("challenge_attempts", "", &local, &server);
        assert_eq!(result, local, "Local insert should proceed (append-only)");
    }

    // ════════════════════════════════════════════
    // R6: User communication
    // ════════════════════════════════════════════

    #[test]
    fn r6_1_offline_shows_banner() {
        let mut state = ConnectivityState::new();
        assert!(state.is_online);
        assert!(state.banner_message.is_none());

        state.set_offline();
        assert!(!state.is_online);
        let banner = state.banner_message.as_ref().expect("banner should be set");
        assert!(
            banner.contains("Offline"),
            "Banner should mention 'Offline': got '{}'",
            banner
        );
        assert!(
            banner.contains("sync"),
            "Banner should mention sync: got '{}'",
            banner
        );
    }

    #[test]
    fn r6_2_online_hides_banner() {
        let mut state = ConnectivityState::new();
        state.set_offline();
        assert!(state.banner_message.is_some());

        state.set_online();
        assert!(state.is_online);
        assert!(
            state.banner_message.is_none(),
            "Banner should be cleared when online"
        );
    }

    #[test]
    fn r6_3_stale_queue_shows_notification() {
        let mut queue = OfflineQueue::new();
        let mut old_mutation = make_xp_mutation(50);
        // Set created_at to 25 hours ago
        old_mutation.created_at = Utc::now() - Duration::hours(25);
        queue.enqueue(old_mutation);

        assert!(
            stale_queue_check(&queue),
            "Queue with 25h-old item should be stale"
        );
        let msg = stale_queue_message(&queue);
        assert!(msg.is_some());
        assert!(
            msg.as_ref().unwrap().contains("hasn't synced"),
            "Message should mention unsynced progress: got '{}'",
            msg.unwrap()
        );
    }

    #[test]
    fn r6_4_fresh_queue_no_notification() {
        let mut queue = OfflineQueue::new();
        // Fresh mutation (just created)
        queue.enqueue(make_xp_mutation(50));

        assert!(
            !stale_queue_check(&queue),
            "Fresh queue should not be stale"
        );
        assert!(
            stale_queue_message(&queue).is_none(),
            "No notification for fresh queue"
        );
    }

    #[test]
    fn r6_5_empty_queue_no_notification() {
        let queue = OfflineQueue::new();
        assert!(!stale_queue_check(&queue));
        assert!(stale_queue_message(&queue).is_none());
    }
}
