//! Prototype of the SPc + Ark "one miner identity" derivation from DESIGN.md.
//!
//! Derives a BIP352-style scan/spend keypair (plus a separate `B_ark`) from a
//! seed, runs the SPc shared secret from both sides (pool and miner), derives
//! domain-separated tier tweaks (`t_onchain` for the coinbase output, `t_ark`
//! for the Ark user pubkey), and builds a real bark `ark1` address for the
//! tweaked Ark pubkey with a correctly blinded mailbox id.
//!
//! Serialization follows BIP352 conventions: `ser_32` is 4-byte big-endian,
//! `ser_P` is the 33-byte SEC1 compressed point. All hashes are BIP340-style
//! tagged hashes; every hash-to-scalar fails (rather than reducing mod n) if
//! the digest is zero or `>= n`.

use std::fmt;

use bitcoin::hashes::{sha256t_hash_newtype, Hash, HashEngine};
use bitcoin::secp256k1::{PublicKey, Scalar, Secp256k1, SecretKey};

use ark::mailbox::{BlindedMailboxIdentifier, MailboxIdentifier};

sha256t_hash_newtype! {
    /// Tag for `input_hash = hash_SPc/Inputs(ser_32(block_height) || ser_P(A_send))`.
    pub struct InputsTag = hash_str("SPc/Inputs");

    /// SPc nonce replacing BIP352's outpoint-based `input_hash`.
    pub struct InputsHash(_);
}

sha256t_hash_newtype! {
    /// Tag for an optional 32-byte digest of `ser_P(ecdh)`. Not used by the
    /// tier tweaks, which hash `ser_P(ecdh)` directly under their own tags.
    pub struct SharedSecretTag = hash_str("SPc/SharedSecret");

    /// `hash_SPc/SharedSecret(ser_P(ecdh))`.
    pub struct SharedSecretHash(_);
}

sha256t_hash_newtype! {
    /// Tag for `t_onchain = hash_SPc/TierSpc(ser_P(ecdh) || ser_32(k))`.
    pub struct TierSpcTag = hash_str("SPc/TierSpc");

    /// On-chain (coinbase) tier tweak preimage hash.
    pub struct TierSpcHash(_);
}

sha256t_hash_newtype! {
    /// Tag for `t_ark = hash_SPc/TierArk(ser_P(ecdh) || ser_32(k))`.
    pub struct TierArkTag = hash_str("SPc/TierArk");

    /// Ark tier tweak preimage hash.
    pub struct TierArkHash(_);
}

/// Derivation failures. Per BIP352, an invalid scalar or a point at infinity
/// means "fail" (pick a new `A_send` / skip this `k`), never "reduce mod n".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// A hash output was zero or `>= n`.
    InvalidScalar,
    /// A point addition produced the point at infinity.
    PointAtInfinity,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidScalar => f.write_str("hash output is not a valid scalar (0 or >= n)"),
            Error::PointAtInfinity => f.write_str("result is the point at infinity"),
        }
    }
}

impl std::error::Error for Error {}

/// Interpret a 32-byte digest as a scalar in `[1, n-1]`, failing otherwise.
fn hash_to_scalar(bytes: [u8; 32]) -> Result<Scalar, Error> {
    let sk = SecretKey::from_slice(&bytes).map_err(|_| Error::InvalidScalar)?;
    Ok(Scalar::from(sk))
}

/// `input_hash = hash_SPc/Inputs(ser_32(block_height) || ser_P(A_send))`.
pub fn input_hash(block_height: u32, a_send: &PublicKey) -> InputsHash {
    let mut eng = InputsHash::engine();
    eng.input(&block_height.to_be_bytes());
    eng.input(&a_send.serialize());
    InputsHash::from_engine(eng)
}

/// SP key material derived from a seed.
pub struct SpKeys {
    pub scan: (SecretKey, PublicKey),
    pub spend: (SecretKey, PublicKey),
    /// Separate key for the Ark tier (`B_ark`).
    pub ark: (SecretKey, PublicKey),
    /// Bark wallet mailbox key (`m/350'/1'`), whose pubkey is the static
    /// `MailboxIdentifier` a bark wallet on the same seed polls.
    pub mailbox: (SecretKey, PublicKey),
}

impl SpKeys {
    /// Derive from a raw BIP32 seed:
    /// - spend:   `m/352'/0'/0'/0'/0` (BIP352)
    /// - scan:    `m/352'/0'/0'/1'/0` (BIP352)
    /// - ark:     `m/352'/0'/0'/2'/0` (this design's extension)
    /// - mailbox: `m/350'/1'` (bark `WalletSeed::to_mailbox_keypair`)
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        use bitcoin::bip32::{ChildNumber, Xpriv};
        use bitcoin::Network;

        let secp = Secp256k1::new();
        let root = Xpriv::new_master(Network::Bitcoin, seed).expect("valid seed");
        let h = |i| ChildNumber::from_hardened_idx(i).unwrap();

        let derive = |path: &[ChildNumber]| {
            let sk = root.derive_priv(&secp, &path).expect("derivation").private_key;
            (sk, PublicKey::from_secret_key(&secp, &sk))
        };
        let sp = |branch| derive(&[h(352), h(0), h(0), h(branch), ChildNumber::from_normal_idx(0).unwrap()]);

        SpKeys {
            spend: sp(0),
            scan: sp(1),
            ark: sp(2),
            mailbox: derive(&[h(350), h(1)]),
        }
    }
}

/// The SPc ECDH shared secret: `ser_P(input_hash * a_send * B_scan)`.
/// Both sides must compute the same 33 bytes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SharedSecret(pub [u8; 33]);

impl SharedSecret {
    /// Pool side: `input_hash * a_send * B_scan`.
    pub fn pool_side(block_height: u32, a_send: &SecretKey, b_scan: &PublicKey) -> Result<Self, Error> {
        let secp = Secp256k1::new();
        let a_pub = PublicKey::from_secret_key(&secp, a_send);
        let ih = hash_to_scalar(input_hash(block_height, &a_pub).to_byte_array())?;
        let s = a_send.mul_tweak(&ih).map_err(|_| Error::InvalidScalar)?;
        let point = b_scan.mul_tweak(&secp, &Scalar::from(s)).map_err(|_| Error::PointAtInfinity)?;
        Ok(SharedSecret(point.serialize()))
    }

    /// Miner side: `input_hash * b_scan * A_send`.
    pub fn miner_side(block_height: u32, b_scan: &SecretKey, a_send: &PublicKey) -> Result<Self, Error> {
        let secp = Secp256k1::new();
        let ih = hash_to_scalar(input_hash(block_height, a_send).to_byte_array())?;
        let s = b_scan.mul_tweak(&ih).map_err(|_| Error::InvalidScalar)?;
        let point = a_send.mul_tweak(&secp, &Scalar::from(s)).map_err(|_| Error::PointAtInfinity)?;
        Ok(SharedSecret(point.serialize()))
    }

    /// `hash_SPc/SharedSecret(ser_P(ecdh))`, a fixed-size digest of the
    /// secret. Not an input to the tier tweaks.
    pub fn digest(&self) -> SharedSecretHash {
        SharedSecretHash::hash(&self.0)
    }
}

/// Domain-separated tier tweaks derived from one shared secret.
pub struct TierTweaks {
    /// `t_onchain = hash_SPc/TierSpc(ser_P(ecdh) || ser_32(k))`
    pub t_onchain: Scalar,
    /// `t_ark = hash_SPc/TierArk(ser_P(ecdh) || ser_32(k))`
    pub t_ark: Scalar,
}

/// Compute the two tier tweaks for output index `k`.
pub fn tier_tweaks(ecdh: &SharedSecret, k: u32) -> Result<TierTweaks, Error> {
    let t_onchain = {
        let mut eng = TierSpcHash::engine();
        eng.input(&ecdh.0);
        eng.input(&k.to_be_bytes());
        hash_to_scalar(TierSpcHash::from_engine(eng).to_byte_array())?
    };
    let t_ark = {
        let mut eng = TierArkHash::engine();
        eng.input(&ecdh.0);
        eng.input(&k.to_be_bytes());
        hash_to_scalar(TierArkHash::from_engine(eng).to_byte_array())?
    };
    Ok(TierTweaks { t_onchain, t_ark })
}

/// `base + t*G`.
fn tweak_pubkey(base: &PublicKey, t: &Scalar) -> Result<PublicKey, Error> {
    base.add_exp_tweak(&Secp256k1::new(), t).map_err(|_| Error::PointAtInfinity)
}

/// SPc on-chain output pubkey: `P = B_spend + t_onchain * G`.
pub fn spc_output_pubkey(b_spend: &PublicKey, t_onchain: &Scalar) -> Result<PublicKey, Error> {
    tweak_pubkey(b_spend, t_onchain)
}

/// Ark user pubkey: `U = B_ark + t_ark * G`.
pub fn ark_user_pubkey(b_ark: &PublicKey, t_ark: &Scalar) -> Result<PublicKey, Error> {
    tweak_pubkey(b_ark, t_ark)
}

/// The SPc output secret key: `p = b_spend + t_onchain`.
pub fn spc_output_secret(b_spend: &SecretKey, t_onchain: &Scalar) -> Result<SecretKey, Error> {
    b_spend.add_tweak(t_onchain).map_err(|_| Error::PointAtInfinity)
}

/// The Ark user secret key: `u = b_ark + t_ark`.
pub fn ark_user_secret(b_ark: &SecretKey, t_ark: &Scalar) -> Result<SecretKey, Error> {
    b_ark.add_tweak(t_ark).map_err(|_| Error::PointAtInfinity)
}

/// Miner side, once per Ark server: `D = b_ark * S_mb`, where `S_mb` is the
/// server's mailbox pubkey (`ArkInfo::mailbox_pubkey`). Registered privately
/// with the pool alongside the miner's `MailboxIdentifier`.
pub fn mailbox_blinding_hint(b_ark: &SecretKey, server_mailbox_pubkey: &PublicKey) -> Result<PublicKey, Error> {
    server_mailbox_pubkey
        .mul_tweak(&Secp256k1::new(), &Scalar::from(*b_ark))
        .map_err(|_| Error::PointAtInfinity)
}

/// Pool side: blind the miner's mailbox id for `U = B_ark + t_ark*G` without
/// knowing `u`: `blinded = M + D + t_ark * S_mb`, which equals bark's
/// `M + u * S_mb` (`BlindedMailboxIdentifier::new`), so the server unblinds it
/// to `M`.
pub fn pool_blinded_mailbox_id(
    mailbox_id: &MailboxIdentifier,
    hint: &PublicKey,
    t_ark: &Scalar,
    server_mailbox_pubkey: &PublicKey,
) -> Result<BlindedMailboxIdentifier, Error> {
    let t_s = server_mailbox_pubkey
        .mul_tweak(&Secp256k1::new(), t_ark)
        .map_err(|_| Error::PointAtInfinity)?;
    let blinded = PublicKey::combine_keys(&[&mailbox_id.as_pubkey(), hint, &t_s])
        .map_err(|_| Error::PointAtInfinity)?;
    Ok(BlindedMailboxIdentifier::from_pubkey(blinded))
}

/// Build a payable bark `ark1` address for the tweaked Ark user pubkey `U`:
/// `ArkId` of the server pubkey, a `Pubkey` policy carrying `U`, and a
/// `ServerMailbox` delivery with the given blinded id.
pub fn build_bark_address(
    server_pubkey: &PublicKey,
    ark_user_pubkey: &PublicKey,
    blinded_id: BlindedMailboxIdentifier,
    testnet: bool,
) -> ark::Address {
    use ark::address::VtxoDelivery;

    ark::Address::builder()
        .testnet(testnet)
        .server_pubkey(*server_pubkey)
        .pubkey_policy(*ark_user_pubkey)
        .delivery(VtxoDelivery::ServerMailbox { blinded_id })
        .into_address()
        .expect("all fields set")
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::hex::DisplayHex;
    use bitcoin::secp256k1::Keypair;
    use std::str::FromStr;

    struct Fixture {
        keys: SpKeys,
        block_height: u32,
        a_send: SecretKey,
        a_send_pub: PublicKey,
        server_pk: PublicKey,
        server_mailbox: Keypair,
    }

    fn fixture() -> Fixture {
        let secp = Secp256k1::new();
        let keys = SpKeys::from_seed(&[0x42u8; 32]);
        let a_send = SecretKey::from_slice(&[0x07u8; 32]).unwrap();
        let server_sk = SecretKey::from_slice(&[0x09u8; 32]).unwrap();
        let server_mailbox_sk = SecretKey::from_slice(&[0x0au8; 32]).unwrap();
        Fixture {
            keys,
            block_height: 800_000,
            a_send,
            a_send_pub: PublicKey::from_secret_key(&secp, &a_send),
            server_pk: PublicKey::from_secret_key(&secp, &server_sk),
            server_mailbox: Keypair::from_secret_key(&secp, &server_mailbox_sk),
        }
    }

    fn pool_ecdh(f: &Fixture) -> SharedSecret {
        SharedSecret::pool_side(f.block_height, &f.a_send, &f.keys.scan.1).unwrap()
    }

    /// Pool side: the full address the pool would pay for this fixture.
    fn pool_address(f: &Fixture, t_ark: &Scalar, u: &PublicKey) -> ark::Address {
        let s_mb = f.server_mailbox.public_key();
        let m = MailboxIdentifier::from_pubkey(f.keys.mailbox.1);
        let hint = mailbox_blinding_hint(&f.keys.ark.0, &s_mb).unwrap();
        let blinded = pool_blinded_mailbox_id(&m, &hint, t_ark, &s_mb).unwrap();
        build_bark_address(&f.server_pk, u, blinded, false)
    }

    #[test]
    fn ecdh_sides_agree() {
        let f = fixture();
        let miner = SharedSecret::miner_side(f.block_height, &f.keys.scan.0, &f.a_send_pub).unwrap();
        assert_eq!(pool_ecdh(&f), miner);
    }

    #[test]
    fn spc_output_spendable() {
        let f = fixture();
        let tweaks = tier_tweaks(&pool_ecdh(&f), 0).unwrap();
        let p = spc_output_pubkey(&f.keys.spend.1, &tweaks.t_onchain).unwrap();
        let p_secret = spc_output_secret(&f.keys.spend.0, &tweaks.t_onchain).unwrap();
        assert_eq!(p, PublicKey::from_secret_key(&Secp256k1::new(), &p_secret));
    }

    #[test]
    fn ark_user_spendable() {
        let f = fixture();
        let tweaks = tier_tweaks(&pool_ecdh(&f), 0).unwrap();
        let u = ark_user_pubkey(&f.keys.ark.1, &tweaks.t_ark).unwrap();
        let u_secret = ark_user_secret(&f.keys.ark.0, &tweaks.t_ark).unwrap();
        assert_eq!(u, PublicKey::from_secret_key(&Secp256k1::new(), &u_secret));
    }

    #[test]
    fn tier_tweaks_differ() {
        let f = fixture();
        let ecdh = pool_ecdh(&f);
        let t0 = tier_tweaks(&ecdh, 0).unwrap();
        let t1 = tier_tweaks(&ecdh, 1).unwrap();
        assert_ne!(t0.t_onchain, t0.t_ark);
        assert_ne!(t0.t_onchain, t1.t_onchain);
        assert_ne!(t0.t_ark, t1.t_ark);
    }

    #[test]
    fn invalid_scalar_fails() {
        assert_eq!(hash_to_scalar([0u8; 32]), Err(Error::InvalidScalar));
        assert_eq!(hash_to_scalar([0xffu8; 32]), Err(Error::InvalidScalar));
        let mut one = [0u8; 32];
        one[31] = 1;
        assert!(hash_to_scalar(one).is_ok());
    }

    /// The pool's blinding (no `u` needed) equals bark's own blinding with
    /// `u`, and the server unblinds it to the miner's static mailbox id.
    #[test]
    fn pool_blinding_matches_bark() {
        let f = fixture();
        let tweaks = tier_tweaks(&pool_ecdh(&f), 0).unwrap();
        let s_mb = f.server_mailbox.public_key();
        let m = MailboxIdentifier::from_pubkey(f.keys.mailbox.1);

        let hint = mailbox_blinding_hint(&f.keys.ark.0, &s_mb).unwrap();
        let pool = pool_blinded_mailbox_id(&m, &hint, &tweaks.t_ark, &s_mb).unwrap();

        let u_sk = ark_user_secret(&f.keys.ark.0, &tweaks.t_ark).unwrap();
        let u_kp = Keypair::from_secret_key(&Secp256k1::new(), &u_sk);
        let bark = BlindedMailboxIdentifier::new(m, s_mb, &u_kp).unwrap();
        assert_eq!(pool, bark);

        let unblinded = MailboxIdentifier::from_blinded(pool, u_kp.public_key(), &f.server_mailbox).unwrap();
        assert_eq!(unblinded, m);

        // The naive unblinded `from_pubkey(U)` lands in a mailbox nobody polls.
        let naive = BlindedMailboxIdentifier::from_pubkey(u_kp.public_key());
        let dead = MailboxIdentifier::from_blinded(naive, u_kp.public_key(), &f.server_mailbox).unwrap();
        assert_ne!(dead, m);
    }

    #[test]
    fn bark_address_roundtrip() {
        let f = fixture();
        let tweaks = tier_tweaks(&pool_ecdh(&f), 0).unwrap();
        let u = ark_user_pubkey(&f.keys.ark.1, &tweaks.t_ark).unwrap();

        let s = pool_address(&f, &tweaks.t_ark, &u).to_string();
        assert!(s.starts_with("ark1p"), "mainnet address: {s}");

        let parsed = ark::Address::from_str(&s).expect("parse");
        assert!(parsed.is_for_server(f.server_pk));
        assert_eq!(parsed.policy().user_pubkey(), u);
        assert_eq!(parsed.delivery().len(), 1);

        // Payload layout: ark_id(4) | len=34 | 0x00 Pubkey | U(33) | len=34 | 0x01 ServerMailbox | blinded(33)
        let mut payload = Vec::new();
        parsed.encode_payload(&mut payload).unwrap();
        assert_eq!(payload.len(), 74);
        assert_eq!(&payload[0..4], &bitcoin::hashes::sha256::Hash::hash(&f.server_pk.serialize())[..4]);
        assert_eq!(&payload[4..6], &[34, 0x00]);
        assert_eq!(&payload[6..39], &u.serialize());
        assert_eq!(&payload[39..41], &[34, 0x01]);
    }

    /// Frozen test vector pinning the spec.
    ///
    /// seed = 0x42 * 32, a_send = 0x07 * 32, block_height = 800000, k = 0,
    /// server pubkey = (0x09 * 32)*G, server mailbox pubkey = (0x0a * 32)*G.
    #[test]
    fn test_vector() {
        let f = fixture();
        let ih = input_hash(f.block_height, &f.a_send_pub);
        let ecdh = pool_ecdh(&f);
        let tweaks = tier_tweaks(&ecdh, 0).unwrap();
        let p = spc_output_pubkey(&f.keys.spend.1, &tweaks.t_onchain).unwrap();
        let u = ark_user_pubkey(&f.keys.ark.1, &tweaks.t_ark).unwrap();
        let s_mb = f.server_mailbox.public_key();
        let hint = mailbox_blinding_hint(&f.keys.ark.0, &s_mb).unwrap();
        let addr = pool_address(&f, &tweaks.t_ark, &u);

        let got = [
            ("A_send", f.a_send_pub.to_string()),
            ("B_scan", f.keys.scan.1.to_string()),
            ("B_spend", f.keys.spend.1.to_string()),
            ("B_ark", f.keys.ark.1.to_string()),
            ("mailbox_id", f.keys.mailbox.1.to_string()),
            ("input_hash", ih.to_byte_array().to_lower_hex_string()),
            ("ecdh", ecdh.0.to_lower_hex_string()),
            ("t_onchain", tweaks.t_onchain.to_be_bytes().to_lower_hex_string()),
            ("t_ark", tweaks.t_ark.to_be_bytes().to_lower_hex_string()),
            ("P_spc", p.to_string()),
            ("U_ark", u.to_string()),
            ("mailbox_hint", hint.to_string()),
            ("blinded_mailbox_id", match &addr.delivery()[0] {
                ark::address::VtxoDelivery::ServerMailbox { blinded_id } => blinded_id.as_pubkey().to_string(),
                other => panic!("unexpected delivery {other:?}"),
            }),
            ("ark_address", addr.to_string()),
        ];
        for (k, v) in &got {
            println!("{k} = {v}");
        }

        let expected = [
            ("A_send", "02989c0b76cb563971fdc9bef31ec06c3560f3249d6ee9e5d83c57625596e05f6f"),
            ("B_scan", "024b19f0da60684ff1b6ee04c39c753a823c4bc758872931bdbef4ca14ff4b9c50"),
            ("B_spend", "032cd165ae1dadd742f410eb06c56467feef432b218a14b216bada9573f9176907"),
            ("B_ark", "02c6accb73e60944acb90d0eaade2d6ddb56cf91449cd11dcd42771fabdf222b18"),
            ("mailbox_id", "0235739eb757168f51be9a504da5b5cda61c37d95133ac677a76605d6971c5274f"),
            ("input_hash", "04df9f4ac4c6ce0c80eca39eca7d4d256eceb2ab2fbe6f52a0acfa8cde3dfd4b"),
            ("ecdh", "02cbef249596b2d7beff88c66723ef7241ec4557c2110831172619f5e277fa24ee"),
            ("t_onchain", "1a06accc037f59f18a114acbb36df8e6a1c5d4b3754f326b2189e8b6e3db72bb"),
            ("t_ark", "5f6d66827770b92b1d7250175da5f90862be9d19f73ed09a485251286158dd0c"),
            ("P_spc", "02a159acc1fa366ab4d5fb9e82adc37d6f5875e0dc9887faeffb58607bd809e831"),
            ("U_ark", "028774fd21598aa994083488788d42c87d344b116e2552d2236c3644bd47f53937"),
            ("mailbox_hint", "02d864313e3db31b98210d966a3a02589696d63491921b7a119bd6943be353820a"),
            ("blinded_mailbox_id", "023682649a5662a1e1d7e559003325ad2fe05f49be7a2b45a68673fceb9d009420"),
            ("ark_address", "ark1pw4zjupfzqqpgwa8ay9vc42v5pq6gs7ydgty86dztz9hz25kjydkrv39agl6njdezqyprdqnynftx9g0p6lj4jqpnykkjlczlfxl8526956r88l8tn5qfggqlrm0es"),
        ];
        assert_eq!(got.len(), expected.len());
        for ((gk, gv), (ek, ev)) in got.iter().zip(expected.iter()) {
            assert_eq!(gk, ek);
            assert_eq!(gv, ev, "{gk}");
        }
    }
}
