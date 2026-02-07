import { describe, it } from 'node:test';
import assert from 'node:assert/strict';

// Load the native addon
const { SyncClient, inviteFromQr, deriveSecret } = await import('../index.js');

// ============================================================
// deriveSecret (standalone, sync — no relay needed)
// ============================================================

describe('deriveSecret', () => {
  it('returns 32-byte secret and group ID', () => {
    const result = deriveSecret('test-pass', Buffer.alloc(16, 0));
    assert.equal(result.secretBytes.length, 32);
    assert.equal(result.groupId.length, 32);
  });

  it('is deterministic', () => {
    const a = deriveSecret('pass', Buffer.from('salt-00000000000!'));
    const b = deriveSecret('pass', Buffer.from('salt-00000000000!'));
    assert.deepEqual(a.secretBytes, b.secretBytes);
    assert.deepEqual(a.groupId, b.groupId);
  });

  it('different passphrases produce different secrets', () => {
    const a = deriveSecret('pass-a', Buffer.from('salt-00000000000!'));
    const b = deriveSecret('pass-b', Buffer.from('salt-00000000000!'));
    assert.notDeepEqual(a.secretBytes, b.secretBytes);
  });

  it('different salts produce different secrets', () => {
    const a = deriveSecret('pass', Buffer.from('salt-AAAAAAAAAA!!'));
    const b = deriveSecret('pass', Buffer.from('salt-BBBBBBBBBB!!'));
    assert.notDeepEqual(a.secretBytes, b.secretBytes);
  });
});

// ============================================================
// SyncClient.create — config validation
// ============================================================

describe('SyncClient.create', () => {
  it('creates with secretBytes', async () => {
    const client = await SyncClient.create({
      secretBytes: Buffer.alloc(32, 42),
      relayAddresses: ['fake-relay-id'],
      deviceName: 'Test Client',
    });
    assert.ok(client);
    assert.equal(typeof client.isConnected, 'function');
    await client.shutdown();
  });

  it('creates with passphrase and salt', async () => {
    const client = await SyncClient.create({
      passphrase: 'test-pass',
      salt: Buffer.alloc(16, 0),
      relayAddresses: ['fake-relay'],
    });
    assert.ok(client);
    await client.shutdown();
  });

  it('rejects passphrase without salt', async () => {
    await assert.rejects(
      () => SyncClient.create({
        passphrase: 'test-pass',
        relayAddresses: ['relay'],
      }),
      (err) => {
        assert.ok(err.message.includes('salt'), `Expected "salt" in: ${err.message}`);
        return true;
      },
    );
  });

  it('rejects empty relay addresses', async () => {
    await assert.rejects(
      () => SyncClient.create({
        secretBytes: Buffer.alloc(32, 42),
        relayAddresses: [],
      }),
      (err) => {
        assert.ok(err.message.includes('empty'), `Expected "empty" in: ${err.message}`);
        return true;
      },
    );
  });

  it('rejects wrong-length secret bytes', async () => {
    await assert.rejects(
      () => SyncClient.create({
        secretBytes: Buffer.alloc(16, 42), // 16 bytes, not 32
        relayAddresses: ['relay'],
      }),
      (err) => {
        assert.ok(err.message.includes('32'), `Expected "32" in: ${err.message}`);
        return true;
      },
    );
  });

  it('rejects both passphrase and secretBytes', async () => {
    await assert.rejects(
      () => SyncClient.create({
        passphrase: 'pass',
        salt: Buffer.alloc(16, 0),
        secretBytes: Buffer.alloc(32, 42),
        relayAddresses: ['relay'],
      }),
      (err) => {
        assert.ok(err.message.includes('both'), `Expected "both" in: ${err.message}`);
        return true;
      },
    );
  });
});

// ============================================================
// SyncClient instance — state before connect
// ============================================================

describe('SyncClient (before connect)', () => {
  let client;

  it('setup: create client', async () => {
    client = await SyncClient.create({
      secretBytes: Buffer.alloc(32, 42),
      relayAddresses: ['fake-relay'],
    });
  });

  it('isConnected returns false', async () => {
    const connected = await client.isConnected();
    assert.equal(connected, false);
  });

  it('currentCursor returns 0', async () => {
    const cursor = await client.currentCursor();
    assert.equal(cursor, 0);
  });

  it('activeRelay returns null', async () => {
    const relay = await client.activeRelay();
    assert.equal(relay, null);
  });

  it('push throws NotConnected', async () => {
    await assert.rejects(
      () => client.push(Buffer.from('hello')),
      (err) => {
        assert.ok(
          err.message.toLowerCase().includes('not connected'),
          `Expected "not connected" in: ${err.message}`,
        );
        return true;
      },
    );
  });

  it('pull throws NotConnected', async () => {
    await assert.rejects(
      () => client.pull(),
      (err) => {
        assert.ok(
          err.message.toLowerCase().includes('not connected'),
          `Expected "not connected" in: ${err.message}`,
        );
        return true;
      },
    );
  });

  it('pullAfter throws NotConnected', async () => {
    await assert.rejects(
      () => client.pullAfter(0),
      (err) => {
        assert.ok(
          err.message.toLowerCase().includes('not connected'),
          `Expected "not connected" in: ${err.message}`,
        );
        return true;
      },
    );
  });

  it('createInvite returns valid structure', () => {
    const invite = client.createInvite(['relay-a', 'relay-b']);
    assert.equal(typeof invite.version, 'number');
    assert.ok(Array.isArray(invite.relayAddresses));
    assert.equal(invite.relayAddresses.length, 2);
    assert.equal(invite.groupSecret.length, 32);
    // salt may be empty when using secretBytes (no Argon2id derivation)
    assert.ok(Buffer.isBuffer(invite.salt));
    assert.equal(typeof invite.qrPayload, 'string');
    assert.ok(invite.qrPayload.length > 0);
    assert.equal(typeof invite.shortCode, 'string');
    assert.match(invite.shortCode, /^[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{4}$/);
  });

  it('invite QR roundtrip preserves group secret', () => {
    const invite = client.createInvite(['relay-id']);
    const restored = inviteFromQr(invite.qrPayload);
    // Group secret and version roundtrip faithfully
    assert.deepEqual(restored.groupSecret, invite.groupSecret);
    assert.equal(restored.version, invite.version);
    // Relay addresses are blake3-hashed RelayNodeIds in the encoded payload,
    // so restored addresses differ from the original human-readable strings
    assert.equal(restored.relayAddresses.length, 1);
    assert.equal(typeof restored.relayAddresses[0], 'string');
  });

  it('shutdown succeeds', async () => {
    await client.shutdown();
  });
});

// ============================================================
// SyncClient with passphrase — same tests
// ============================================================

describe('SyncClient (passphrase mode)', () => {
  it('creates and verifies state', async () => {
    const client = await SyncClient.create({
      passphrase: 'my-secure-passphrase',
      salt: Buffer.from('salt-00000000000!'),
      relayAddresses: ['fake-relay'],
      deviceName: 'Integration Test',
      ttl: 3600,
    });
    assert.ok(client);

    const connected = await client.isConnected();
    assert.equal(connected, false);

    const cursor = await client.currentCursor();
    assert.equal(cursor, 0);

    await client.shutdown();
  });
});
