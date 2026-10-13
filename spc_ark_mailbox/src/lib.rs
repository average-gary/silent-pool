//! Rotating per-epoch mailbox ids with on-demand reads, layered on `spc_ark`.
//! Spec: `MAILBOX.md`.
//!
//! An epoch is the height `i` of a block the pool found. Per epoch the wallet
//! derives a fresh mailbox keypair `m_i` at `m/352'/0'/0'/3'/i'`, so
//! `M_i = m_i * G` is unrelated to every other `M_j` and to bark's static `M`.
//! The pool blinds to `M_i` with the existing once-per-server hint
//! `D = b_ark * S_mb`:
//!
//! ```text
//! blinded_i = M_i + D + t_ark_i * S_mb  ==  M_i + u_i * S_mb
//! ```
//!
//! which the server unblinds to `M_i`. The wallet doesn't hold a subscription
//! per mailbox. When a pool block's payout is due, it does one authorized
//! `ReadMailbox` call for that block's `M_i`.
//!
//! Variant B (`derived_*`) needs no per-epoch registration: `M_i` is a tweak
//! of the static `M` by a hash of the SPc ECDH the pool already computes.

use std::fmt;

use bitcoin::bip32::{ChildNumber, Xpriv};
use bitcoin::hashes::{sha256, sha256t_hash_newtype, Hash, HashEngine};
use bitcoin::secp256k1::{Keypair, Message, PublicKey, Scalar, Secp256k1, SecretKey};
use bitcoin::Network;

use ark::mailbox::{BlindedMailboxIdentifier, MailboxAuthorization, MailboxIdentifier};
use ark::ProtocolEncoding;

use spc_ark::SharedSecret;

sha256t_hash_newtype! {
    /// Tag for variant B: `t_mb_i = hash_SPc/Mailbox(ser_P(ecdh_i))`.
    pub struct MailboxTweakTag = hash_str("SPc/Mailbox");

    /// Variant-B per-epoch mailbox tweak.
    pub struct MailboxTweakHash(_);
}

/// Hardened branch under `m/352'/0'/0'` for per-epoch mailbox keys
/// (0' spend, 1' scan, 2' ark, 3' mailbox).
pub const MAILBOX_BRANCH: u32 = 3;

/// Prefix of bark's mailbox authorization challenge
/// (`MailboxAuthorization::CHALENGE_MESSAGE_PREFIX`, `lib/src/mailbox.rs:251`).
pub const AUTH_CHALLENGE_PREFIX: &[u8; 32] = b"Ark VTXO mailbox authorization: ";

/// Validity of a one-shot read authorization. bark itself uses 10s for
/// subscribe and 10 min for a full sync. One read with a few pages fits well
/// inside a minute, and the token works as a bearer credential until it
/// expires, so keep it short.
pub const READ_AUTH_VALIDITY_SECS: i64 = 60;

/// Page limit per read, same as bark's `MAX_MAILBOX_REQUEST_BURST`.
pub const MAX_READ_PAGES: usize = 10;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// Epoch height is `>= 2^31`, so it can't be a hardened BIP32 index.
    EpochOutOfRange(u32),
    /// Error from the underlying SPc derivation.
    Spc(spc_ark::Error),
    /// Blinding or tweaking produced the point at infinity.
    PointAtInfinity,
    /// Hash output was not a valid scalar.
    InvalidScalar,
    /// The read transport failed (RPC error, server rejected auth, ...).
    Transport(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::EpochOutOfRange(h) => write!(f, "epoch {h} is not a valid hardened index"),
            Error::Spc(e) => write!(f, "spc derivation: {e}"),
            Error::PointAtInfinity => f.write_str("result is the point at infinity"),
            Error::InvalidScalar => f.write_str("hash output is not a valid scalar"),
            Error::Transport(e) => write!(f, "mailbox transport: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<spc_ark::Error> for Error {
    fn from(e: spc_ark::Error) -> Self {
        Error::Spc(e)
    }
}

// ---------------------------------------------------------------------------
// 1. Per-epoch mailbox key derivation
// ---------------------------------------------------------------------------

/// Wallet-side source of per-epoch mailbox keys. Holds the BIP32 root because
/// `m_i` is on a hardened branch (the pool can't derive it from an xpub).
/// This sits next to `spc_ark::SpKeys` and doesn't modify it.
pub struct EpochMailboxKeys {
    root: Xpriv,
}

impl EpochMailboxKeys {
    /// Same seed and root as `spc_ark::SpKeys::from_seed`.
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        EpochMailboxKeys { root: Xpriv::new_master(Network::Bitcoin, seed).expect("valid seed") }
    }

    /// `m_i` at `m/352'/0'/0'/3'/i'`, where `i` is the pool block height.
    pub fn keypair(&self, epoch: u32) -> Result<Keypair, Error> {
        let secp = Secp256k1::new();
        let h = |i| ChildNumber::from_hardened_idx(i);
        let path = [
            h(352).unwrap(),
            h(0).unwrap(),
            h(0).unwrap(),
            h(MAILBOX_BRANCH).unwrap(),
            h(epoch).map_err(|_| Error::EpochOutOfRange(epoch))?,
        ];
        let sk = self.root.derive_priv(&secp, &path).expect("hardened derivation").private_key;
        Ok(Keypair::from_secret_key(&secp, &sk))
    }

    /// `M_i = MailboxIdentifier::from_pubkey(m_i * G)`.
    pub fn mailbox_id(&self, epoch: u32) -> Result<MailboxIdentifier, Error> {
        Ok(MailboxIdentifier::from_pubkey(self.keypair(epoch)?.public_key()))
    }

    /// Registration payloads for a batch of epochs (see [EpochRegistration]).
    pub fn registrations(&self, epochs: impl IntoIterator<Item = u32>) -> Result<Vec<EpochRegistration>, Error> {
        epochs
            .into_iter()
            .map(|epoch| Ok(EpochRegistration { epoch, mailbox_id: self.mailbox_id(epoch)? }))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// 2. Per-epoch registration and pool-side blinding
// ---------------------------------------------------------------------------

/// What the miner gives the pool per epoch, over the same private,
/// authenticated account channel used for the once-per-server `D`.
///
/// Wire format (37 bytes): `ser_32(epoch) || ser_P(M_i)`.
///
/// `D = b_ark * S_mb` doesn't depend on the epoch: `U_i = B_ark + t_ark_i*G`
/// only rotates `t_ark_i`, and the pool already knows that. So `M_i` is the
/// only per-epoch data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EpochRegistration {
    pub epoch: u32,
    pub mailbox_id: MailboxIdentifier,
}

impl EpochRegistration {
    pub const ENCODED_LEN: usize = 4 + 33;

    pub fn encode(&self) -> [u8; Self::ENCODED_LEN] {
        let mut out = [0u8; Self::ENCODED_LEN];
        out[..4].copy_from_slice(&self.epoch.to_be_bytes());
        out[4..].copy_from_slice(&self.mailbox_id.as_pubkey().serialize());
        out
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != Self::ENCODED_LEN {
            return None;
        }
        let epoch = u32::from_be_bytes(bytes[..4].try_into().unwrap());
        let pk = PublicKey::from_slice(&bytes[4..]).ok()?;
        Some(EpochRegistration { epoch, mailbox_id: MailboxIdentifier::from_pubkey(pk) })
    }
}

/// Pool side: `blinded_i = M_i + D + t_ark_i * S_mb`. Uses one EC
/// multiplication (`t_ark_i * S_mb`) and needs only registered public data
/// plus the block's `t_ark_i`. This is `spc_ark::pool_blinded_mailbox_id`
/// with `M_i` in place of the static `M`.
pub fn pool_blinded_epoch_id(
    registration: &EpochRegistration,
    hint: &PublicKey,
    t_ark_i: &Scalar,
    server_mailbox_pubkey: &PublicKey,
) -> Result<BlindedMailboxIdentifier, Error> {
    Ok(spc_ark::pool_blinded_mailbox_id(&registration.mailbox_id, hint, t_ark_i, server_mailbox_pubkey)?)
}

// ---------------------------------------------------------------------------
// Variant B: ECDH-derived epoch mailbox (no per-epoch registration)
// ---------------------------------------------------------------------------

/// `t_mb_i = hash_SPc/Mailbox(ser_P(ecdh_i))`, failing if 0 or `>= n`.
pub fn derived_mailbox_tweak(ecdh: &SharedSecret) -> Result<Scalar, Error> {
    let bytes = MailboxTweakHash::hash(&ecdh.0).to_byte_array();
    let sk = SecretKey::from_slice(&bytes).map_err(|_| Error::InvalidScalar)?;
    Ok(Scalar::from(sk))
}

/// Pool side (variant B): `M_i = M + t_mb_i * G` from the static,
/// already-registered `M`.
pub fn derived_epoch_mailbox_id(static_id: &MailboxIdentifier, ecdh: &SharedSecret) -> Result<MailboxIdentifier, Error> {
    let t = derived_mailbox_tweak(ecdh)?;
    let pk = static_id
        .as_pubkey()
        .add_exp_tweak(&Secp256k1::new(), &t)
        .map_err(|_| Error::PointAtInfinity)?;
    Ok(MailboxIdentifier::from_pubkey(pk))
}

/// Wallet side (variant B): `m_i = m + t_mb_i`.
pub fn derived_epoch_keypair(static_mailbox_sk: &SecretKey, ecdh: &SharedSecret) -> Result<Keypair, Error> {
    let t = derived_mailbox_tweak(ecdh)?;
    let sk = static_mailbox_sk.add_tweak(&t).map_err(|_| Error::PointAtInfinity)?;
    Ok(Keypair::from_secret_key(&Secp256k1::new(), &sk))
}

// ---------------------------------------------------------------------------
// 3. On-demand read: authorization + request construction
// ---------------------------------------------------------------------------

/// bark's challenge: `sha256(b"Ark VTXO mailbox authorization: " || le64(expiry))`.
/// Plain SHA256, not a tagged hash. The id isn't in the message; the BIP340
/// challenge already commits to the signing pubkey `M_i`.
pub fn authorization_message(expiry_unix: i64) -> Message {
    let mut eng = sha256::Hash::engine();
    eng.input(AUTH_CHALLENGE_PREFIX);
    eng.input(&expiry_unix.to_le_bytes());
    Message::from_digest(sha256::Hash::from_engine(eng).to_byte_array())
}

/// Production path: bark's own constructor (random BIP340 aux).
pub fn epoch_authorization(m_i: &Keypair, expiry: chrono::DateTime<chrono::Local>) -> MailboxAuthorization {
    MailboxAuthorization::new(m_i, expiry)
}

/// Deterministic authorization bytes (zero aux) in bark's wire format, used
/// for test vectors: `ser_P(M_i) || le64(expiry) || sig64` (105 bytes).
/// The result decodes with `MailboxAuthorization::deserialize`.
pub fn deterministic_authorization_bytes(m_i: &Keypair, expiry_unix: i64) -> Vec<u8> {
    let sig = Secp256k1::new().sign_schnorr_no_aux_rand(&authorization_message(expiry_unix), m_i);
    let mut out = Vec::with_capacity(105);
    out.extend_from_slice(&m_i.public_key().serialize());
    out.extend_from_slice(&expiry_unix.to_le_bytes());
    out.extend_from_slice(sig.as_ref());
    out
}

/// Mirrors `mailbox_server.MailboxRequest`
/// (`server-rpc/protos/mailbox_server.proto:24-31`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailboxRequest {
    pub mailbox_id: Vec<u8>,
    pub authorization: Option<Vec<u8>>,
    pub checkpoint: u64,
}

/// One message from a `ReadMailbox` page. Only arkoor deliveries matter for
/// pool payouts. Other kinds are kept opaque.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MailboxMessage {
    Arkoor { checkpoint: u64, vtxos: Vec<Vec<u8>> },
    Other { checkpoint: u64 },
}

impl MailboxMessage {
    pub fn checkpoint(&self) -> u64 {
        match self {
            MailboxMessage::Arkoor { checkpoint, .. } | MailboxMessage::Other { checkpoint } => *checkpoint,
        }
    }
}

/// Mirrors `mailbox_server.MailboxMessages`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MailboxMessages {
    pub messages: Vec<MailboxMessage>,
    pub have_more: bool,
}

/// The one-shot read transport.
///
/// RPC PLUG-IN POINT: a real implementation wraps bark's tonic
/// `mailbox_client.read_mailbox(protos::mailbox_server::MailboxRequest { .. })`
/// (`bark/src/mailbox.rs:318-326`), copying the three fields across and
/// mapping `MailboxMessages` back. Do not use `subscribe_mailbox`: the point
/// of this design is that nothing holds a stream per mailbox.
pub trait MailboxReader {
    fn read_mailbox(&mut self, req: MailboxRequest) -> Result<MailboxMessages, Error>;
}

/// Build the authorized one-shot request for epoch `i`.
pub fn build_read_request(
    keys: &EpochMailboxKeys,
    epoch: u32,
    checkpoint: u64,
    now: chrono::DateTime<chrono::Local>,
) -> Result<MailboxRequest, Error> {
    let m_i = keys.keypair(epoch)?;
    let expiry = now + chrono::Duration::seconds(READ_AUTH_VALIDITY_SECS);
    let auth = epoch_authorization(&m_i, expiry);
    Ok(MailboxRequest {
        mailbox_id: auth.mailbox().serialize(),
        authorization: Some(auth.serialize()),
        checkpoint,
    })
}

/// Result of an on-demand read for one epoch.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EpochReadResult {
    /// Raw VTXO bytes from arkoor deliveries. They still need bark's
    /// validation and the SP-aware keystore (`u_i = b_ark + t_ark_i`).
    pub vtxos: Vec<Vec<u8>>,
    /// Highest checkpoint seen. Store it per epoch and use it to resume.
    pub checkpoint: u64,
    /// `true` if the page limit was hit with `have_more` still set.
    pub truncated: bool,
}

/// Wallet-side trigger handler: the pool found block `epoch` (and its payout
/// is due). Re-derive `m_i`, sign one short-lived authorization, and page
/// through `ReadMailbox` for `M_i`. No subscription is opened.
///
/// One authorization covers every page of the read, as in bark's
/// `sync_mailbox`.
pub fn read_epoch_mailbox(
    keys: &EpochMailboxKeys,
    epoch: u32,
    since_checkpoint: u64,
    now: chrono::DateTime<chrono::Local>,
    reader: &mut impl MailboxReader,
) -> Result<EpochReadResult, Error> {
    let template = build_read_request(keys, epoch, since_checkpoint, now)?;
    let mut out = EpochReadResult { checkpoint: since_checkpoint, ..Default::default() };

    for _ in 0..MAX_READ_PAGES {
        let resp = reader.read_mailbox(MailboxRequest { checkpoint: out.checkpoint, ..template.clone() })?;
        for msg in resp.messages {
            out.checkpoint = out.checkpoint.max(msg.checkpoint());
            if let MailboxMessage::Arkoor { vtxos, .. } = msg {
                out.vtxos.extend(vtxos);
            }
        }
        if !resp.have_more {
            return Ok(out);
        }
    }
    out.truncated = true;
    Ok(out)
}

// ---------------------------------------------------------------------------
// 4. Catch-up / lookahead scheduling
// ---------------------------------------------------------------------------

/// Coinbase maturity: the pool can't board pool-block proceeds into Ark
/// before this.
pub const COINBASE_MATURITY: u32 = 100;

/// Default retry schedule, in blocks after maturity, for epochs that are
/// still open. One read per entry, so at most `RETRY_OFFSETS.len()` reads per
/// epoch. The last entry is the give-up horizon.
pub const RETRY_OFFSETS: [u32; 5] = [0, 6, 36, 144, 1008];

/// Which retry offsets of `epoch` are due at `tip`, given how many reads
/// were already made for it. Returns `true` if a read should happen now.
pub fn read_due(epoch: u32, reads_done: usize, tip: u32) -> bool {
    match RETRY_OFFSETS.get(reads_done) {
        Some(off) => tip >= epoch.saturating_add(COINBASE_MATURITY).saturating_add(*off),
        None => false,
    }
}

/// Catch-up plan for startup or recovery: given the pool-block heights found
/// in `(last_synced, tip]` (from scanning coinbases for the pool tag) and
/// the epochs already closed, return the epochs to read now. Each returned
/// epoch costs one `ReadMailbox` call, plus a page for every
/// `max_read_mailbox_items` messages.
pub fn catch_up_epochs(pool_blocks: &[u32], closed: &[u32], tip: u32) -> Vec<u32> {
    pool_blocks
        .iter()
        .copied()
        .filter(|e| !closed.contains(e))
        .filter(|e| tip >= e.saturating_add(COINBASE_MATURITY))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::hex::DisplayHex;
    use spc_ark::{ark_user_pubkey, ark_user_secret, mailbox_blinding_hint, tier_tweaks, SpKeys};
    use std::collections::HashMap;

    const SEED: [u8; 32] = [0x42; 32];
    const EPOCH: u32 = 800_000;

    struct Fx {
        keys: SpKeys,
        epochs: EpochMailboxKeys,
        a_send: SecretKey,
        server_pk: PublicKey,
        server_mailbox: Keypair,
    }

    fn fx() -> Fx {
        let secp = Secp256k1::new();
        Fx {
            keys: SpKeys::from_seed(&SEED),
            epochs: EpochMailboxKeys::from_seed(&SEED),
            a_send: SecretKey::from_slice(&[0x07; 32]).unwrap(),
            server_pk: PublicKey::from_secret_key(&secp, &SecretKey::from_slice(&[0x09; 32]).unwrap()),
            server_mailbox: Keypair::from_secret_key(&secp, &SecretKey::from_slice(&[0x0a; 32]).unwrap()),
        }
    }

    fn ecdh(f: &Fx, epoch: u32) -> SharedSecret {
        SharedSecret::pool_side(epoch, &f.a_send, &f.keys.scan.1).unwrap()
    }

    /// Pool side for one epoch: (U_i, blinded_i).
    fn pool_pays(f: &Fx, epoch: u32, reg: &EpochRegistration) -> (PublicKey, BlindedMailboxIdentifier, Scalar) {
        let s_mb = f.server_mailbox.public_key();
        let t_ark = tier_tweaks(&ecdh(f, epoch), 0).unwrap().t_ark;
        let hint = mailbox_blinding_hint(&f.keys.ark.0, &s_mb).unwrap();
        let u = ark_user_pubkey(&f.keys.ark.1, &t_ark).unwrap();
        (u, pool_blinded_epoch_id(reg, &hint, &t_ark, &s_mb).unwrap(), t_ark)
    }

    /// (a) Deterministic, distinct across epochs, distinct from bark's
    /// static `m/350'/1'` mailbox, and only valid for hardened indices.
    #[test]
    fn epoch_derivation_deterministic_and_distinct() {
        let f = fx();
        let again = EpochMailboxKeys::from_seed(&SEED);
        let ids: Vec<_> = (EPOCH..EPOCH + 16).map(|e| f.epochs.mailbox_id(e).unwrap()).collect();
        for (i, e) in (EPOCH..EPOCH + 16).enumerate() {
            assert_eq!(ids[i], again.mailbox_id(e).unwrap());
            assert_eq!(ids[i].as_pubkey(), f.epochs.keypair(e).unwrap().public_key());
            assert_ne!(ids[i].as_pubkey(), f.keys.mailbox.1);
            for j in 0..i {
                assert_ne!(ids[i], ids[j]);
            }
        }
        assert_ne!(EpochMailboxKeys::from_seed(&[0x43; 32]).mailbox_id(EPOCH).unwrap(), ids[0]);
        assert_eq!(f.epochs.keypair(1 << 31).unwrap_err(), Error::EpochOutOfRange(1 << 31));

        let reg = f.epochs.registrations([EPOCH]).unwrap()[0];
        assert_eq!(EpochRegistration::decode(&reg.encode()), Some(reg));
    }

    /// (b) The pool's `blinded_i` equals bark's `BlindedMailboxIdentifier::new`
    /// for `(M_i, u_i)`, and the server's `from_blinded` recovers `M_i`, for
    /// several epochs. The same `D` serves every epoch.
    #[test]
    fn pool_blinding_unblinds_to_epoch_mailbox() {
        let f = fx();
        let secp = Secp256k1::new();
        for epoch in [EPOCH, EPOCH + 1, EPOCH + 2016] {
            let reg = f.epochs.registrations([epoch]).unwrap()[0];
            let (u, blinded, t_ark) = pool_pays(&f, epoch, &reg);

            let u_kp = Keypair::from_secret_key(&secp, &ark_user_secret(&f.keys.ark.0, &t_ark).unwrap());
            assert_eq!(u_kp.public_key(), u);
            let bark = BlindedMailboxIdentifier::new(reg.mailbox_id, f.server_mailbox.public_key(), &u_kp).unwrap();
            assert_eq!(blinded, bark);

            let unblinded = MailboxIdentifier::from_blinded(blinded, u, &f.server_mailbox).unwrap();
            assert_eq!(unblinded, reg.mailbox_id);
            assert_ne!(unblinded.as_pubkey(), f.keys.mailbox.1, "must not land in the static M");
        }
    }

    /// Variant B: pool-side `M + t_mb_i*G` matches the wallet's `m + t_mb_i`
    /// and unblinds correctly, with no per-epoch registration.
    #[test]
    fn variant_b_derived_mailbox() {
        let f = fx();
        let m = MailboxIdentifier::from_pubkey(f.keys.mailbox.1);
        let ids: Vec<_> = [EPOCH, EPOCH + 1].iter().map(|&e| {
            let pool_id = derived_epoch_mailbox_id(&m, &ecdh(&f, e)).unwrap();
            let wallet_kp = derived_epoch_keypair(&f.keys.mailbox.0, &ecdh(&f, e)).unwrap();
            assert_eq!(pool_id.as_pubkey(), wallet_kp.public_key());

            let reg = EpochRegistration { epoch: e, mailbox_id: pool_id };
            let (u, blinded, _) = pool_pays(&f, e, &reg);
            assert_eq!(MailboxIdentifier::from_blinded(blinded, u, &f.server_mailbox).unwrap(), pool_id);

            let auth = epoch_authorization(&wallet_kp, chrono::Local::now() + chrono::Duration::seconds(60));
            assert!(auth.verify());
            pool_id
        }).collect();
        assert_ne!(ids[0], ids[1]);
        assert_ne!(ids[0], m);
    }

    /// (c) The per-epoch authorization verifies, is bound to `M_i`, survives
    /// bark's wire encoding, and fails for another epoch's id.
    #[test]
    fn epoch_authorization_verifies() {
        let f = fx();
        let m_i = f.epochs.keypair(EPOCH).unwrap();
        let expiry = chrono::Local::now() + chrono::Duration::seconds(READ_AUTH_VALIDITY_SECS);

        let auth = epoch_authorization(&m_i, expiry);
        assert!(auth.verify());
        assert!(!auth.is_expired());
        assert_eq!(auth.mailbox(), f.epochs.mailbox_id(EPOCH).unwrap());

        let bytes = auth.serialize();
        assert_eq!(bytes.len(), 105);
        let decoded = MailboxAuthorization::deserialize(&bytes).unwrap();
        assert_eq!(decoded, auth);
        assert!(decoded.verify());

        // Our reimplementation of the challenge matches bark's: a
        // deterministic auth decodes and verifies under bark's verify().
        let det = MailboxAuthorization::deserialize(&deterministic_authorization_bytes(&m_i, expiry.timestamp())).unwrap();
        assert!(det.verify());

        // Swap in M_{i+1}: signature no longer verifies.
        let mut forged = bytes.clone();
        forged[..33].copy_from_slice(&f.epochs.mailbox_id(EPOCH + 1).unwrap().as_pubkey().serialize());
        assert!(!MailboxAuthorization::deserialize(&forged).unwrap().verify());

        // An expired authorization is rejected by the server's check.
        let old = epoch_authorization(&m_i, chrono::Local::now() - chrono::Duration::seconds(60));
        assert!(old.verify() && old.is_expired());
    }

    /// Stand-in for captaind's mailbox: `PostArkoorMessage` unblinds with
    /// `from_blinded`; `ReadMailbox` repeats the server checks
    /// (`server/src/rpcserver/mailbox.rs:182-212`). Pages hold one message.
    #[derive(Default)]
    struct MockServer {
        key: Option<Keypair>,
        boxes: HashMap<MailboxIdentifier, Vec<(u64, Vec<u8>)>>,
        next_cp: u64,
        reads: usize,
    }

    impl MockServer {
        fn post(&mut self, blinded: BlindedMailboxIdentifier, u: PublicKey, vtxo: Vec<u8>) {
            let id = MailboxIdentifier::from_blinded(blinded, u, self.key.as_ref().unwrap()).unwrap();
            self.next_cp += 1;
            self.boxes.entry(id).or_default().push((self.next_cp, vtxo));
        }
    }

    impl MailboxReader for MockServer {
        fn read_mailbox(&mut self, req: MailboxRequest) -> Result<MailboxMessages, Error> {
            self.reads += 1;
            let id = MailboxIdentifier::deserialize(&req.mailbox_id).map_err(|e| Error::Transport(e.to_string()))?;
            let auth = MailboxAuthorization::deserialize(req.authorization.as_deref().ok_or(Error::Transport("auth required".into()))?)
                .map_err(|e| Error::Transport(e.to_string()))?;
            if auth.mailbox() != id || auth.is_expired() || !auth.verify() {
                return Err(Error::Transport("bad authorization".into()));
            }
            let pending: Vec<_> = self.boxes.get(&id).into_iter().flatten().filter(|(cp, _)| *cp > req.checkpoint).collect();
            Ok(MailboxMessages {
                have_more: pending.len() > 1,
                messages: pending.iter().take(1).map(|(cp, v)| MailboxMessage::Arkoor { checkpoint: *cp, vtxos: vec![v.clone()] }).collect(),
            })
        }
    }

    /// End to end: pool pays epochs i and j to rotating ids; one targeted
    /// read for epoch i returns only epoch i's payouts.
    #[test]
    fn on_demand_read_end_to_end() {
        let f = fx();
        let mut server = MockServer { key: Some(f.server_mailbox), ..Default::default() };
        for (epoch, tag) in [(EPOCH, b'a'), (EPOCH + 7, b'b'), (EPOCH, b'c')] {
            let reg = f.epochs.registrations([epoch]).unwrap()[0];
            let (u, blinded, _) = pool_pays(&f, epoch, &reg);
            server.post(blinded, u, vec![tag]);
        }
        assert_eq!(server.boxes.len(), 2, "two epochs, two unlinkable mailboxes");

        let got = read_epoch_mailbox(&f.epochs, EPOCH, 0, chrono::Local::now(), &mut server).unwrap();
        assert_eq!(got.vtxos, vec![vec![b'a'], vec![b'c']]);
        assert_eq!(got.checkpoint, 3);
        assert!(!got.truncated);
        assert_eq!(server.reads, 2, "two pages, no subscription");

        // Resuming from the stored checkpoint returns nothing new.
        let again = read_epoch_mailbox(&f.epochs, EPOCH, got.checkpoint, chrono::Local::now(), &mut server).unwrap();
        assert!(again.vtxos.is_empty());

        // The static mailbox received nothing.
        assert!(!server.boxes.contains_key(&MailboxIdentifier::from_pubkey(f.keys.mailbox.1)));
    }

    #[test]
    fn catch_up_schedule() {
        let pool_blocks = [EPOCH, EPOCH + 50, EPOCH + 120, EPOCH + 300];
        let tip = EPOCH + 250;
        // EPOCH is closed; EPOCH+300 is not mature yet.
        assert_eq!(catch_up_epochs(&pool_blocks, &[EPOCH], tip), vec![EPOCH + 50, EPOCH + 120]);
        assert!(read_due(EPOCH, 0, EPOCH + 100));
        assert!(!read_due(EPOCH, 0, EPOCH + 99));
        assert!(!read_due(EPOCH, 1, EPOCH + 105));
        assert!(read_due(EPOCH, 1, EPOCH + 106));
        assert!(read_due(EPOCH, 4, EPOCH + 1108));
        assert!(!read_due(EPOCH, 5, u32::MAX), "give up after the last retry");
    }

    /// (d) Frozen vector for epoch 800000. Same fixture as `spc_ark`'s
    /// `test_vector` (seed = 0x42*32, a_send = 0x07*32, k = 0, server pubkey
    /// = (0x09*32)*G, server mailbox pubkey = (0x0a*32)*G), plus
    /// auth expiry = 1_800_000_000 and a zero-aux BIP340 signature.
    #[test]
    fn test_vector() {
        let f = fx();
        let m_i = f.epochs.keypair(EPOCH).unwrap();
        let reg = EpochRegistration { epoch: EPOCH, mailbox_id: MailboxIdentifier::from_pubkey(m_i.public_key()) };
        let (u, blinded, t_ark) = pool_pays(&f, EPOCH, &reg);
        let hint = mailbox_blinding_hint(&f.keys.ark.0, &f.server_mailbox.public_key()).unwrap();
        let addr = spc_ark::build_bark_address(&f.server_pk, &u, blinded, false);
        let expiry = 1_800_000_000i64;
        let auth = deterministic_authorization_bytes(&m_i, expiry);
        assert!(MailboxAuthorization::deserialize(&auth).unwrap().verify());
        assert_eq!(MailboxIdentifier::from_blinded(blinded, u, &f.server_mailbox).unwrap(), reg.mailbox_id);
        let derived_b = derived_epoch_mailbox_id(&MailboxIdentifier::from_pubkey(f.keys.mailbox.1), &ecdh(&f, EPOCH)).unwrap();

        let got = [
            ("m_i", m_i.secret_key().secret_bytes().to_lower_hex_string()),
            ("M_i", reg.mailbox_id.to_string()),
            ("registration", reg.encode().to_lower_hex_string()),
            ("t_ark", t_ark.to_be_bytes().to_lower_hex_string()),
            ("U_ark", u.to_string()),
            ("mailbox_hint", hint.to_string()),
            ("blinded_mailbox_id_i", blinded.as_pubkey().to_string()),
            ("ark_address_i", addr.to_string()),
            ("auth_message", authorization_message(expiry).as_ref().to_lower_hex_string()),
            ("auth_bytes", auth.to_lower_hex_string()),
            ("variant_b_M_i", derived_b.to_string()),
        ];
        for (k, v) in &got {
            println!("{k} = {v}");
        }

        let expected: [(&str, &str); 11] = [
            ("m_i", "fa83f7461a305cccaadafa3d3c2efe115543d8b39a2b83d09ac2ae5eaaa308e4"),
            ("M_i", "03f4da68f5676d28c34153149dcb5cacce602cd9b0b515f5c57666c3fbe6daefc3"),
            ("registration", "000c350003f4da68f5676d28c34153149dcb5cacce602cd9b0b515f5c57666c3fbe6daefc3"),
            ("t_ark", "5f6d66827770b92b1d7250175da5f90862be9d19f73ed09a485251286158dd0c"),
            ("U_ark", "028774fd21598aa994083488788d42c87d344b116e2552d2236c3644bd47f53937"),
            ("mailbox_hint", "02d864313e3db31b98210d966a3a02589696d63491921b7a119bd6943be353820a"),
            ("blinded_mailbox_id_i", "0360539d99a4da9b50c72bbc9bcc9b40d19bf26bf15969ec6039de8022e40063dc"),
            ("ark_address_i", "ark1pw4zjupfzqqpgwa8ay9vc42v5pq6gs7ydgty86dztz9hz25kjydkrv39agl6njdezqypkq5uanxjd4x6scu4mex7vndqdrxljd0c4j60vvquaaqpzusqx8hq6au2cp"),
            ("auth_message", "9fb17cbdff11be6d831da0e9f72119f7c7291e50503e100be9c274367399c77b"),
            ("auth_bytes", "03f4da68f5676d28c34153149dcb5cacce602cd9b0b515f5c57666c3fbe6daefc300d2496b00000000d25fe29d03bbefe95ae2cebfda734d1fc50aa0808f7d24d044b57bd783f4399dc973034683ca333d42d8011719c4b43eef2587b40a3303f775f25bbc6a2bc310"),
            ("variant_b_M_i", "031ff5ec81ad81bcbf75e828fec51c334d97191d1f001bfebc85cea1989b1b23a2"),
        ];
        for ((gk, gv), (ek, ev)) in got.iter().zip(expected.iter()) {
            assert_eq!(gk, ek);
            assert_eq!(gv, ev, "{gk}");
        }
    }
}
