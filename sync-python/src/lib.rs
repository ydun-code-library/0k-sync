//! # sync-python
//!
//! Python bindings for 0k-Sync via PyO3.
//!
//! Wraps [`zerok_sync_bridge::SyncHandle`] into Python classes with
//! async methods that return coroutines (awaitable from asyncio).

#![warn(clippy::all)]

use std::sync::Arc;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use zerok_sync_bridge::{
    error::SyncBridgeError,
    handle::derive_secret as bridge_derive_secret,
    SyncHandle, SyncHandleConfig,
};

// ============================================================
// Error conversion
// ============================================================

fn to_py_err(err: SyncBridgeError) -> PyErr {
    PyRuntimeError::new_err(err.to_string())
}

// ============================================================
// FFI types — #[pyclass(frozen)] for immutable Python objects
// ============================================================

/// Configuration for creating a SyncClient.
#[pyclass(frozen)]
pub struct SyncConfig {
    #[pyo3(get)]
    passphrase: Option<String>,
    #[pyo3(get)]
    salt: Option<Vec<u8>>,
    #[pyo3(get)]
    secret_bytes: Option<Vec<u8>>,
    #[pyo3(get)]
    relay_addresses: Vec<String>,
    #[pyo3(get)]
    device_name: Option<String>,
    #[pyo3(get)]
    ttl: Option<u32>,
}

#[pymethods]
impl SyncConfig {
    #[new]
    #[pyo3(signature = (*, passphrase=None, salt=None, secret_bytes=None, relay_addresses, device_name=None, ttl=None))]
    fn new(
        passphrase: Option<String>,
        salt: Option<Vec<u8>>,
        secret_bytes: Option<Vec<u8>>,
        relay_addresses: Vec<String>,
        device_name: Option<String>,
        ttl: Option<u32>,
    ) -> Self {
        Self {
            passphrase,
            salt,
            secret_bytes,
            relay_addresses,
            device_name,
            ttl,
        }
    }
}

/// Result of a push operation.
#[pyclass(frozen)]
pub struct PushResult {
    /// The blob identifier (UUID string).
    #[pyo3(get)]
    blob_id: String,
    /// The assigned cursor position.
    #[pyo3(get)]
    cursor: i64,
}

#[pymethods]
impl PushResult {
    fn __repr__(&self) -> String {
        format!("PushResult(blob_id='{}', cursor={})", self.blob_id, self.cursor)
    }
}

/// A received blob from a pull operation.
#[pyclass(frozen)]
pub struct SyncBlob {
    /// The blob identifier (UUID string).
    #[pyo3(get)]
    blob_id: String,
    /// The decrypted payload bytes.
    #[pyo3(get)]
    data: Vec<u8>,
    /// The cursor position.
    #[pyo3(get)]
    cursor: i64,
    /// Original timestamp.
    #[pyo3(get)]
    timestamp: i64,
}

#[pymethods]
impl SyncBlob {
    fn __repr__(&self) -> String {
        format!(
            "SyncBlob(blob_id='{}', cursor={}, len={})",
            self.blob_id, self.cursor, self.data.len()
        )
    }
}

/// An invite for sharing group access.
#[pyclass(frozen)]
pub struct SyncInvite {
    /// Invite format version.
    #[pyo3(get)]
    version: u32,
    /// Relay addresses.
    #[pyo3(get)]
    relay_addresses: Vec<String>,
    /// The group secret bytes (32 bytes).
    #[pyo3(get)]
    group_secret: Vec<u8>,
    /// Argon2id salt.
    #[pyo3(get)]
    salt: Vec<u8>,
    /// QR payload string (base64-encoded JSON).
    #[pyo3(get)]
    qr_payload: String,
    /// Short code (XXXX-XXXX-XXXX-XXXX).
    #[pyo3(get)]
    short_code: String,
}

#[pymethods]
impl SyncInvite {
    fn __repr__(&self) -> String {
        format!(
            "SyncInvite(version={}, relays={}, short_code='{}')",
            self.version,
            self.relay_addresses.len(),
            self.short_code
        )
    }
}

/// Result of deriving a group secret.
#[pyclass(frozen)]
pub struct DeriveResult {
    /// The 32-byte derived secret.
    #[pyo3(get)]
    secret_bytes: Vec<u8>,
    /// The 32-byte group identifier.
    #[pyo3(get)]
    group_id: Vec<u8>,
}

#[pymethods]
impl DeriveResult {
    fn __repr__(&self) -> String {
        format!(
            "DeriveResult(secret_len={}, group_id_len={})",
            self.secret_bytes.len(),
            self.group_id.len()
        )
    }
}

// ============================================================
// Internal conversion helpers
// ============================================================

fn py_config_to_bridge(config: &SyncConfig) -> PyResult<SyncHandleConfig> {
    let bridge = SyncHandleConfig {
        passphrase: config.passphrase.clone(),
        salt: config.salt.clone(),
        secret_bytes: config.secret_bytes.clone(),
        relay_addresses: config.relay_addresses.clone(),
        device_name: config.device_name.clone(),
        ttl: config.ttl,
    };
    bridge.validate().map_err(to_py_err)?;
    Ok(bridge)
}

fn bridge_push_to_py(result: zerok_sync_bridge::PushResult) -> PushResult {
    PushResult {
        blob_id: result.blob_id,
        cursor: result.cursor as i64,
    }
}

fn bridge_blob_to_py(blob: zerok_sync_bridge::SyncBlob) -> SyncBlob {
    SyncBlob {
        blob_id: blob.blob_id,
        data: blob.data,
        cursor: blob.cursor as i64,
        timestamp: blob.timestamp as i64,
    }
}

fn bridge_invite_to_py(invite: zerok_sync_bridge::SyncInvite) -> SyncInvite {
    SyncInvite {
        version: invite.version,
        relay_addresses: invite.relay_addresses,
        group_secret: invite.group_secret,
        salt: invite.salt,
        qr_payload: invite.qr_payload,
        short_code: invite.short_code,
    }
}

// ============================================================
// SyncClient — the main pyclass
//
// Uses Arc<SyncHandle> so async closures can share ownership
// without requiring Clone on SyncHandle itself.
// ============================================================

/// The main sync client for Python.
///
/// All async methods return coroutines. Create via `await SyncClient.create(config)`.
///
/// Supports async context manager:
///     async with await SyncClient.create(config) as client:
///         ...
#[pyclass]
pub struct SyncClient {
    handle: Arc<SyncHandle>,
}

#[pymethods]
impl SyncClient {
    /// Create a new SyncClient from configuration.
    ///
    /// This binds a real iroh endpoint (async, may take 0-3s).
    #[staticmethod]
    fn create<'py>(py: Python<'py>, config: &SyncConfig) -> PyResult<Bound<'py, PyAny>> {
        let bridge_config = py_config_to_bridge(config)?;
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let handle = SyncHandle::create(bridge_config)
                .await
                .map_err(to_py_err)?;
            Ok(SyncClient {
                handle: Arc::new(handle),
            })
        })
    }

    /// Check if connected to a relay.
    fn is_connected<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let handle = Arc::clone(&self.handle);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            Ok(handle.is_connected().await)
        })
    }

    /// Get the current cursor position.
    fn current_cursor<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let handle = Arc::clone(&self.handle);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            Ok(handle.current_cursor().await as i64)
        })
    }

    /// Get the address of the active relay (if connected).
    fn active_relay<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let handle = Arc::clone(&self.handle);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            Ok(handle.active_relay().await)
        })
    }

    /// Connect to the relay(s).
    fn connect<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let handle = Arc::clone(&self.handle);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            handle.connect().await.map_err(to_py_err)?;
            Ok(())
        })
    }

    /// Disconnect from the relay.
    fn disconnect<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let handle = Arc::clone(&self.handle);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            handle.disconnect().await.map_err(to_py_err)?;
            Ok(())
        })
    }

    /// Push encrypted data to the sync group.
    fn push<'py>(&self, py: Python<'py>, data: Vec<u8>) -> PyResult<Bound<'py, PyAny>> {
        let handle = Arc::clone(&self.handle);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let result = handle.push(&data).await.map_err(to_py_err)?;
            Ok(bridge_push_to_py(result))
        })
    }

    /// Pull new blobs from the sync group.
    fn pull<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let handle = Arc::clone(&self.handle);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let blobs = handle.pull().await.map_err(to_py_err)?;
            Ok(blobs.into_iter().map(bridge_blob_to_py).collect::<Vec<_>>())
        })
    }

    /// Pull blobs after a specific cursor.
    fn pull_after<'py>(&self, py: Python<'py>, cursor: i64) -> PyResult<Bound<'py, PyAny>> {
        let handle = Arc::clone(&self.handle);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let blobs = handle
                .pull_after(cursor as u64)
                .await
                .map_err(to_py_err)?;
            Ok(blobs.into_iter().map(bridge_blob_to_py).collect::<Vec<_>>())
        })
    }

    /// Create an invite for sharing group access.
    fn create_invite(&self, relay_addresses: Vec<String>) -> PyResult<SyncInvite> {
        let invite = self
            .handle
            .create_invite(&relay_addresses)
            .map_err(to_py_err)?;
        Ok(bridge_invite_to_py(invite))
    }

    /// Disconnect and release resources.
    fn shutdown<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let handle = Arc::clone(&self.handle);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            handle.disconnect().await.map_err(to_py_err)?;
            Ok(())
        })
    }

    /// Async context manager entry — returns self.
    fn __aenter__<'py>(slf: Bound<'py, Self>, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let slf_py: Py<Self> = slf.unbind();
        pyo3_async_runtimes::tokio::future_into_py(py, async move { Ok(slf_py) })
    }

    /// Async context manager exit — calls shutdown.
    #[pyo3(signature = (_exc_type=None, _exc_val=None, _exc_tb=None))]
    fn __aexit__<'py>(
        &self,
        py: Python<'py>,
        _exc_type: Option<Bound<'py, PyAny>>,
        _exc_val: Option<Bound<'py, PyAny>>,
        _exc_tb: Option<Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let handle = Arc::clone(&self.handle);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            // Best-effort shutdown, ignore errors
            let _ = handle.disconnect().await;
            Ok(false) // Don't suppress exceptions
        })
    }
}

// ============================================================
// Standalone functions
// ============================================================

/// Decode an invite from a QR payload string.
#[pyfunction]
fn invite_from_qr(payload: String) -> PyResult<SyncInvite> {
    let invite = SyncHandle::invite_from_qr(&payload).map_err(to_py_err)?;
    Ok(bridge_invite_to_py(invite))
}

/// Derive a group secret from a passphrase and salt.
///
/// Returns a DeriveResult with the 32-byte secret and 32-byte group ID.
#[pyfunction]
fn derive_secret(passphrase: String, salt: Vec<u8>) -> DeriveResult {
    let (secret_bytes, group_id) = bridge_derive_secret(&passphrase, &salt);
    DeriveResult {
        secret_bytes,
        group_id,
    }
}

// ============================================================
// Module definition
// ============================================================

#[pymodule]
fn _zerok_sync(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<SyncConfig>()?;
    m.add_class::<SyncClient>()?;
    m.add_class::<PushResult>()?;
    m.add_class::<SyncBlob>()?;
    m.add_class::<SyncInvite>()?;
    m.add_class::<DeriveResult>()?;
    m.add_function(wrap_pyfunction!(invite_from_qr, m)?)?;
    m.add_function(wrap_pyfunction!(derive_secret, m)?)?;
    Ok(())
}

// ============================================================
// Tests — bridge-level only (no Python interpreter in tests)
// ============================================================

#[cfg(test)]
mod tests {
    use zerok_sync_bridge::{
        error::SyncBridgeError, handle::derive_secret as bridge_derive_secret, PushResult,
        SyncBlob, SyncHandleConfig, SyncInvite,
    };

    // --- Config validation (bridge types, no PyO3) ---

    #[test]
    fn config_from_passphrase_validates() {
        let config =
            SyncHandleConfig::from_passphrase("test-pass", b"test-salt-00000!", "relay-node");
        config.validate().unwrap();
        assert_eq!(config.passphrase, Some("test-pass".to_string()));
        assert_eq!(config.relay_addresses, vec!["relay-node"]);
    }

    #[test]
    fn config_from_secret_bytes_validates() {
        let config = SyncHandleConfig::from_secret_bytes(&[0x42; 32], "relay");
        config.validate().unwrap();
        assert_eq!(config.secret_bytes.as_deref(), Some([0x42; 32].as_slice()));
    }

    #[test]
    fn config_rejects_both_passphrase_and_secret() {
        let config = SyncHandleConfig {
            passphrase: Some("pass".to_string()),
            salt: Some(b"salt".to_vec()),
            secret_bytes: Some(vec![0u8; 32]),
            relay_addresses: vec!["relay".to_string()],
            device_name: None,
            ttl: None,
        };
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("both"));
    }

    #[test]
    fn config_rejects_empty_relays() {
        let config = SyncHandleConfig {
            passphrase: None,
            salt: None,
            secret_bytes: Some(vec![0u8; 32]),
            relay_addresses: vec![],
            device_name: None,
            ttl: None,
        };
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    // --- Bridge type conversions ---

    #[test]
    fn push_result_fields_map_correctly() {
        let result = PushResult {
            blob_id: "abc-123".to_string(),
            cursor: 42,
        };
        assert_eq!(result.blob_id, "abc-123");
        assert_eq!(result.cursor as i64, 42i64);
    }

    #[test]
    fn sync_blob_fields_map_correctly() {
        let blob = SyncBlob {
            blob_id: "def-456".to_string(),
            data: vec![1, 2, 3],
            cursor: 10,
            timestamp: 1705000000,
        };
        assert_eq!(blob.blob_id, "def-456");
        assert_eq!(blob.data, vec![1, 2, 3]);
        assert_eq!(blob.cursor as i64, 10i64);
        assert_eq!(blob.timestamp as i64, 1705000000i64);
    }

    #[test]
    fn sync_invite_fields_map_correctly() {
        let invite = SyncInvite {
            version: 3,
            relay_addresses: vec!["relay-a".to_string(), "relay-b".to_string()],
            group_secret: vec![0x42; 32],
            salt: vec![0x01; 16],
            qr_payload: "encoded-payload".to_string(),
            short_code: "ABCD-EFGH-IJKL-MNOP".to_string(),
        };
        assert_eq!(invite.version, 3);
        assert_eq!(invite.relay_addresses.len(), 2);
        assert_eq!(invite.group_secret.len(), 32);
        assert_eq!(invite.qr_payload, "encoded-payload");
        assert_eq!(invite.short_code, "ABCD-EFGH-IJKL-MNOP");
    }

    // --- Error conversion ---

    #[test]
    fn bridge_error_to_string_is_human_readable() {
        let err = SyncBridgeError::NotConnected;
        assert_eq!(err.to_string(), "not connected");

        let err = SyncBridgeError::InvalidConfig("missing salt".to_string());
        assert!(err.to_string().contains("missing salt"));
    }

    #[test]
    fn bridge_error_connection_failed_preserves_reason() {
        let err = SyncBridgeError::ConnectionFailed("relay unreachable".to_string());
        assert!(err.to_string().contains("relay unreachable"));
    }

    // --- derive_secret ---

    #[test]
    fn derive_secret_is_deterministic() {
        let r1 = bridge_derive_secret("pass", b"salt-00000000000!");
        let r2 = bridge_derive_secret("pass", b"salt-00000000000!");
        assert_eq!(r1.0, r2.0);
        assert_eq!(r1.1, r2.1);
        assert_eq!(r1.0.len(), 32);
        assert!(!r1.1.is_empty());
    }

    #[test]
    fn derive_secret_different_passphrases_differ() {
        let r1 = bridge_derive_secret("pass-a", b"salt-00000000000!");
        let r2 = bridge_derive_secret("pass-b", b"salt-00000000000!");
        assert_ne!(r1.0, r2.0);
    }
}
