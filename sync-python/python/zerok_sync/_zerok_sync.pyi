"""Type stubs for the zerok_sync native module."""

from typing import Optional

class SyncConfig:
    """Configuration for creating a SyncClient."""

    passphrase: Optional[str]
    salt: Optional[bytes]
    secret_bytes: Optional[bytes]
    relay_addresses: list[str]
    device_name: Optional[str]
    ttl: Optional[int]

    def __init__(
        self,
        *,
        passphrase: Optional[str] = None,
        salt: Optional[bytes] = None,
        secret_bytes: Optional[bytes] = None,
        relay_addresses: list[str],
        device_name: Optional[str] = None,
        ttl: Optional[int] = None,
    ) -> None: ...

class PushResult:
    """Result of a push operation."""

    blob_id: str
    cursor: int

    def __repr__(self) -> str: ...

class SyncBlob:
    """A received blob from a pull operation."""

    blob_id: str
    data: bytes
    cursor: int
    timestamp: int

    def __repr__(self) -> str: ...

class SyncInvite:
    """An invite for sharing group access."""

    version: int
    relay_addresses: list[str]
    group_secret: bytes
    salt: bytes
    qr_payload: str
    short_code: str

    def __repr__(self) -> str: ...

class DeriveResult:
    """Result of deriving a group secret."""

    secret_bytes: bytes
    group_id: bytes

    def __repr__(self) -> str: ...

class SyncClient:
    """The main sync client.

    All async methods return coroutines. Create via ``await SyncClient.create(config)``.

    Supports async context manager::

        async with await SyncClient.create(config) as client:
            ...
    """

    @staticmethod
    async def create(config: SyncConfig) -> "SyncClient": ...
    async def is_connected(self) -> bool: ...
    async def current_cursor(self) -> int: ...
    async def active_relay(self) -> Optional[str]: ...
    async def connect(self) -> None: ...
    async def disconnect(self) -> None: ...
    async def push(self, data: bytes) -> PushResult: ...
    async def pull(self) -> list[SyncBlob]: ...
    async def pull_after(self, cursor: int) -> list[SyncBlob]: ...
    def create_invite(self, relay_addresses: list[str]) -> SyncInvite: ...
    async def shutdown(self) -> None: ...
    async def __aenter__(self) -> "SyncClient": ...
    async def __aexit__(
        self,
        exc_type: Optional[type] = None,
        exc_val: Optional[BaseException] = None,
        exc_tb: Optional[object] = None,
    ) -> bool: ...

def invite_from_qr(payload: str) -> SyncInvite:
    """Decode an invite from a QR payload string."""
    ...

def derive_secret(passphrase: str, salt: bytes) -> DeriveResult:
    """Derive a group secret from a passphrase and salt."""
    ...
