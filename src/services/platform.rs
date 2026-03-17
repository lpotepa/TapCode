/// Platform abstraction traits for haptics, secure storage, and safe area.
/// Implementations are selected at app init based on the target platform.

// ── Haptic Engine ──

pub trait HapticEngine: Send + Sync {
    fn light_tap(&self);
    fn success_pulse(&self);
    fn error_tap(&self);
    fn double_pulse(&self);
    fn medium_tap(&self);
}

/// No-op implementation for web and testing
pub struct NoOpHaptics;

impl HapticEngine for NoOpHaptics {
    fn light_tap(&self) {}
    fn success_pulse(&self) {}
    fn error_tap(&self) {}
    fn double_pulse(&self) {}
    fn medium_tap(&self) {}
}

// ── Secure Storage ──

pub trait SecureStorage: Send + Sync {
    fn set(&self, key: &str, value: &str) -> Result<(), StorageError>;
    fn get(&self, key: &str) -> Result<Option<String>, StorageError>;
    fn delete(&self, key: &str) -> Result<(), StorageError>;
}

#[derive(Debug)]
pub enum StorageError {
    NotAvailable,
    WriteError(String),
    ReadError(String),
}

/// In-memory storage for web (wraps localStorage when available)
pub struct MemoryStorage {
    data: std::sync::Mutex<std::collections::HashMap<String, String>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            data: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl SecureStorage for MemoryStorage {
    fn set(&self, key: &str, value: &str) -> Result<(), StorageError> {
        self.data
            .lock()
            .map_err(|e| StorageError::WriteError(e.to_string()))?
            .insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Option<String>, StorageError> {
        Ok(self
            .data
            .lock()
            .map_err(|e| StorageError::ReadError(e.to_string()))?
            .get(key)
            .cloned())
    }

    fn delete(&self, key: &str) -> Result<(), StorageError> {
        self.data
            .lock()
            .map_err(|e| StorageError::WriteError(e.to_string()))?
            .remove(key);
        Ok(())
    }
}
