"""Integration tests for zerok_sync Python bindings.

Tests the actual PyO3 bindings against real bridge types.
No mocking — these tests verify the full Python → Rust → bridge path.
"""

import pytest
from zerok_sync import (
    SyncClient,
    SyncConfig,
    PushResult,
    SyncBlob,
    SyncInvite,
    DeriveResult,
    derive_secret,
    invite_from_qr,
)


# ============================================================
# derive_secret
# ============================================================


class TestDeriveSecret:
    def test_returns_derive_result(self):
        result = derive_secret("test-pass", b"salt-00000000000!")
        assert isinstance(result, DeriveResult)

    def test_secret_is_32_bytes(self):
        result = derive_secret("test-pass", b"salt-00000000000!")
        assert len(result.secret_bytes) == 32

    def test_group_id_is_nonempty(self):
        result = derive_secret("test-pass", b"salt-00000000000!")
        assert len(result.group_id) > 0

    def test_deterministic(self):
        r1 = derive_secret("test-pass", b"salt-00000000000!")
        r2 = derive_secret("test-pass", b"salt-00000000000!")
        assert r1.secret_bytes == r2.secret_bytes
        assert r1.group_id == r2.group_id

    def test_different_passphrases_differ(self):
        r1 = derive_secret("pass-a", b"salt-00000000000!")
        r2 = derive_secret("pass-b", b"salt-00000000000!")
        assert r1.secret_bytes != r2.secret_bytes

    def test_different_salts_differ(self):
        r1 = derive_secret("test-pass", b"salt-AAAAAAAAAA!!")
        r2 = derive_secret("test-pass", b"salt-BBBBBBBBBB!!")
        assert r1.secret_bytes != r2.secret_bytes

    def test_repr(self):
        result = derive_secret("test-pass", b"salt-00000000000!")
        r = repr(result)
        assert "secret_len=32" in r
        assert "group_id_len=" in r


# ============================================================
# SyncConfig validation
# ============================================================


class TestSyncConfig:
    def test_from_secret_bytes(self):
        config = SyncConfig(
            secret_bytes=bytes(32),
            relay_addresses=["fake-relay-id"],
        )
        assert config.secret_bytes == bytes(32)
        assert config.relay_addresses == ["fake-relay-id"]

    def test_from_passphrase_with_salt(self):
        config = SyncConfig(
            passphrase="test-pass",
            salt=b"salt-00000000000!",
            relay_addresses=["fake-relay"],
        )
        assert config.passphrase == "test-pass"
        assert config.salt == b"salt-00000000000!"

    def test_keyword_only_args(self):
        """SyncConfig requires keyword arguments."""
        with pytest.raises(TypeError):
            SyncConfig("pass", b"salt", None, ["relay"])  # type: ignore

    def test_optional_fields(self):
        config = SyncConfig(
            secret_bytes=bytes(32),
            relay_addresses=["relay"],
            device_name="my-device",
            ttl=600,
        )
        assert config.device_name == "my-device"
        assert config.ttl == 600

    def test_defaults_are_none(self):
        config = SyncConfig(
            secret_bytes=bytes(32),
            relay_addresses=["relay"],
        )
        assert config.passphrase is None
        assert config.salt is None
        assert config.device_name is None
        assert config.ttl is None


# ============================================================
# SyncClient.create — config validation errors
# ============================================================


class TestSyncClientCreateValidation:
    @pytest.mark.asyncio
    async def test_rejects_passphrase_without_salt(self):
        config = SyncConfig(
            passphrase="test-pass",
            relay_addresses=["relay"],
        )
        with pytest.raises(RuntimeError, match="(?i)salt"):
            await SyncClient.create(config)

    @pytest.mark.asyncio
    async def test_rejects_both_passphrase_and_secret(self):
        config = SyncConfig(
            passphrase="test-pass",
            salt=b"salt-00000000000!",
            secret_bytes=bytes(32),
            relay_addresses=["relay"],
        )
        with pytest.raises(RuntimeError, match="(?i)both"):
            await SyncClient.create(config)

    @pytest.mark.asyncio
    async def test_rejects_empty_relay_addresses(self):
        config = SyncConfig(
            secret_bytes=bytes(32),
            relay_addresses=[],
        )
        with pytest.raises(RuntimeError, match="(?i)empty"):
            await SyncClient.create(config)

    @pytest.mark.asyncio
    async def test_rejects_wrong_length_secret(self):
        config = SyncConfig(
            secret_bytes=bytes(16),  # Should be 32
            relay_addresses=["relay"],
        )
        with pytest.raises(RuntimeError):
            await SyncClient.create(config)


# ============================================================
# SyncClient — state before connect
# ============================================================


class TestSyncClientState:
    @pytest.mark.asyncio
    async def test_create_with_secret_bytes(self):
        config = SyncConfig(
            secret_bytes=bytes(32),
            relay_addresses=["fake-relay-id"],
            device_name="Test Client",
        )
        client = await SyncClient.create(config)
        assert client is not None
        assert isinstance(client, SyncClient)

    @pytest.mark.asyncio
    async def test_create_with_passphrase(self):
        config = SyncConfig(
            passphrase="test-pass",
            salt=b"salt-00000000000!",
            relay_addresses=["fake-relay"],
        )
        client = await SyncClient.create(config)
        assert client is not None

    @pytest.mark.asyncio
    async def test_is_connected_false_before_connect(self):
        config = SyncConfig(
            secret_bytes=bytes(32),
            relay_addresses=["fake-relay"],
        )
        client = await SyncClient.create(config)
        connected = await client.is_connected()
        assert connected is False

    @pytest.mark.asyncio
    async def test_current_cursor_zero_before_sync(self):
        config = SyncConfig(
            secret_bytes=bytes(32),
            relay_addresses=["fake-relay"],
        )
        client = await SyncClient.create(config)
        cursor = await client.current_cursor()
        assert cursor == 0

    @pytest.mark.asyncio
    async def test_active_relay_none_before_connect(self):
        config = SyncConfig(
            secret_bytes=bytes(32),
            relay_addresses=["fake-relay"],
        )
        client = await SyncClient.create(config)
        relay = await client.active_relay()
        assert relay is None

    @pytest.mark.asyncio
    async def test_push_without_connect_raises(self):
        config = SyncConfig(
            secret_bytes=bytes(32),
            relay_addresses=["fake-relay"],
        )
        client = await SyncClient.create(config)
        with pytest.raises(RuntimeError, match="(?i)not connected"):
            await client.push(b"hello")

    @pytest.mark.asyncio
    async def test_pull_without_connect_raises(self):
        config = SyncConfig(
            secret_bytes=bytes(32),
            relay_addresses=["fake-relay"],
        )
        client = await SyncClient.create(config)
        with pytest.raises(RuntimeError, match="(?i)not connected"):
            await client.pull()

    @pytest.mark.asyncio
    async def test_pull_after_without_connect_raises(self):
        config = SyncConfig(
            secret_bytes=bytes(32),
            relay_addresses=["fake-relay"],
        )
        client = await SyncClient.create(config)
        with pytest.raises(RuntimeError, match="(?i)not connected"):
            await client.pull_after(0)

    @pytest.mark.asyncio
    async def test_shutdown_without_connect_succeeds(self):
        config = SyncConfig(
            secret_bytes=bytes(32),
            relay_addresses=["fake-relay"],
        )
        client = await SyncClient.create(config)
        await client.shutdown()  # Should not raise


# ============================================================
# Async context manager
# ============================================================


class TestAsyncContextManager:
    @pytest.mark.asyncio
    async def test_context_manager_entry_returns_client(self):
        config = SyncConfig(
            secret_bytes=bytes(32),
            relay_addresses=["fake-relay"],
        )
        client = await SyncClient.create(config)
        async with client as ctx:
            assert ctx is not None

    @pytest.mark.asyncio
    async def test_context_manager_exit_without_connect(self):
        """Exit should not raise even if never connected."""
        config = SyncConfig(
            secret_bytes=bytes(32),
            relay_addresses=["fake-relay"],
        )
        client = await SyncClient.create(config)
        async with client:
            pass  # Should exit cleanly


# ============================================================
# Invite operations
# ============================================================


class TestInvite:
    @pytest.mark.asyncio
    async def test_create_invite(self):
        config = SyncConfig(
            secret_bytes=bytes(range(32)),
            relay_addresses=["relay-a"],
        )
        client = await SyncClient.create(config)
        invite = client.create_invite(["relay-a"])
        assert isinstance(invite, SyncInvite)
        assert invite.version == 3
        assert len(invite.relay_addresses) >= 1
        assert len(invite.group_secret) == 32
        assert len(invite.qr_payload) > 0
        assert len(invite.short_code) > 0

    @pytest.mark.asyncio
    async def test_invite_short_code_format(self):
        config = SyncConfig(
            secret_bytes=bytes(range(32)),
            relay_addresses=["relay-a"],
        )
        client = await SyncClient.create(config)
        invite = client.create_invite(["relay-a"])
        parts = invite.short_code.split("-")
        assert len(parts) == 4
        assert all(len(p) == 4 for p in parts)
        assert len(invite.short_code) == 19

    @pytest.mark.asyncio
    async def test_invite_qr_roundtrip(self):
        config = SyncConfig(
            secret_bytes=bytes(range(32)),
            relay_addresses=["relay-a"],
        )
        client = await SyncClient.create(config)
        invite = client.create_invite(["relay-a"])
        restored = invite_from_qr(invite.qr_payload)
        assert isinstance(restored, SyncInvite)
        assert restored.group_secret == invite.group_secret
        assert restored.version == invite.version

    @pytest.mark.asyncio
    async def test_invite_repr(self):
        config = SyncConfig(
            secret_bytes=bytes(range(32)),
            relay_addresses=["relay-a"],
        )
        client = await SyncClient.create(config)
        invite = client.create_invite(["relay-a"])
        r = repr(invite)
        assert "SyncInvite" in r
        assert "version=3" in r
