//! Miner-side ("modified client") logic for receiving SPc/Ark-tier payouts
//! from a stock bark Ark server.
//!
//! This is the whole client patch surface, kept out of the bark tree:
//!
//! 1. [MinerKeys]: derive `b_scan`, `b_ark` and bark's mailbox key from one
//!    BIP39 seed (the mailbox key matches bark's `m/350'/1'`).
//! 2. [SpArkKeystore]: an SP-aware keystore mapping each tweaked
//!    `U = B_ark + t_ark*G` to `u = b_ark + t_ark`, built by scanning pool
//!    blocks. Stands in for bark's `pubkey_keypair` / `find_vtxo_keypairs`.
//! 3. [claim_vtxos]: the receive filter. Bark drops a mailbox VTXO when
//!    `find_signable_clause` can't find its key; this accepts it when the
//!    keystore can.
//! 4. [read_arkoor_mailbox], [query_vtxo_status], [spend_tweaked_vtxo]: the
//!    stock server RPCs, driven with `u` instead of a bark seed key.
//!
//! Nothing here touches the server: it uses the same public RPCs a stock
//! bark wallet uses.

use std::collections::HashMap;

use anyhow::{bail, Context};
use bitcoin::bip32::{ChildNumber, Xpriv};
use bitcoin::secp256k1::{Keypair, PublicKey, Secp256k1, SecretKey};
use bitcoin::Network;

use ark::arkoor::package::{ArkoorPackageBuilder, ArkoorPackageCosignResponse};
use ark::attestations::VtxoStatusAttestation;
use ark::mailbox::MailboxAuthorization;
use ark::vtxo::policy::VtxoPolicyKind;
use ark::vtxo::{Full, Vtxo};
use ark::ProtocolEncoding;
use server_rpc::{protos, ServerConnection};

use spc_ark::{ark_user_pubkey, ark_user_secret, tier_tweaks, SharedSecret};

/// Miner key material from one BIP39 seed.
pub struct MinerKeys {
	pub scan: Keypair,
	pub spend: Keypair,
	pub ark: Keypair,
	/// Bark's wallet mailbox key at `m/350'/1'`; its pubkey is the
	/// `MailboxIdentifier` `M` a bark wallet on the same seed polls.
	pub mailbox: Keypair,
}

impl MinerKeys {
	/// Same paths as `spc_ark::SpKeys`, but from a BIP39 (64-byte) seed as
	/// bark uses, so the mailbox key equals the bark wallet's.
	pub fn from_bip39_seed(seed: &[u8]) -> Self {
		let secp = Secp256k1::new();
		let root = Xpriv::new_master(Network::Regtest, seed).expect("valid seed");
		let h = |i| ChildNumber::from_hardened_idx(i).unwrap();
		let derive = |path: &[ChildNumber]| {
			root.derive_priv(&secp, &path).expect("derivation").to_keypair(&secp)
		};
		let sp = |branch| derive(&[h(352), h(0), h(0), h(branch), ChildNumber::from_normal_idx(0).unwrap()]);
		MinerKeys {
			spend: sp(0),
			scan: sp(1),
			ark: sp(2),
			mailbox: derive(&[h(350), h(1)]),
		}
	}

	/// Registration hint `D = b_ark * S_mb`, given privately to the pool once
	/// per Ark server, together with `M`.
	pub fn mailbox_hint(&self, server_mailbox_pubkey: &PublicKey) -> anyhow::Result<PublicKey> {
		Ok(spc_ark::mailbox_blinding_hint(&self.ark.secret_key(), server_mailbox_pubkey)?)
	}
}

/// A block found by the pool: its height and the `A_send` from its coinbase
/// pool tag.
#[derive(Clone, Copy, Debug)]
pub struct PoolBlock {
	pub height: u32,
	pub a_send: PublicKey,
}

/// SP-aware keystore: every Ark user pubkey the pool may have paid, mapped to
/// its secret key `u = b_ark + t_ark`.
#[derive(Default)]
pub struct SpArkKeystore {
	keys: HashMap<PublicKey, Keypair>,
}

impl SpArkKeystore {
	/// Derive `U -> u` for outputs `k in 0..outputs_per_block` of every
	/// scanned pool block. A `k` whose hash-to-scalar fails is skipped (the
	/// pool skips it too).
	pub fn scan(
		b_scan: &SecretKey,
		b_ark: &SecretKey,
		blocks: &[PoolBlock],
		outputs_per_block: u32,
	) -> anyhow::Result<Self> {
		let secp = Secp256k1::new();
		let b_ark_pub = PublicKey::from_secret_key(&secp, b_ark);
		let mut keys = HashMap::new();
		for block in blocks {
			let ecdh = SharedSecret::miner_side(block.height, b_scan, &block.a_send)?;
			for k in 0..outputs_per_block {
				let Ok(tweaks) = tier_tweaks(&ecdh, k) else { continue };
				let u_pub = ark_user_pubkey(&b_ark_pub, &tweaks.t_ark)?;
				let u = ark_user_secret(b_ark, &tweaks.t_ark)?;
				let kp = Keypair::from_secret_key(&secp, &u);
				debug_assert_eq!(kp.public_key(), u_pub);
				keys.insert(u_pub, kp);
			}
		}
		Ok(SpArkKeystore { keys })
	}

	pub fn keypair_for(&self, pubkey: &PublicKey) -> Option<&Keypair> {
		self.keys.get(pubkey)
	}

	pub fn len(&self) -> usize {
		self.keys.len()
	}

	pub fn is_empty(&self) -> bool {
		self.keys.is_empty()
	}
}

/// Receive filter for arkoor mailbox VTXOs, mirroring bark's
/// `process_raw_vtxos` (`bark/src/mailbox.rs`) but with the SP keystore in
/// place of `find_signable_clause`. Chain-anchor validation
/// (`Vtxo::validate`) is left to the caller, which has the chain source.
pub fn claim_vtxos(
	raw_vtxos: &[Vec<u8>],
	server_pubkey: PublicKey,
	keystore: &SpArkKeystore,
) -> Vec<(Vtxo<Full>, Keypair)> {
	let mut ret = Vec::new();
	for bytes in raw_vtxos {
		let Ok(vtxo) = Vtxo::<Full>::deserialize(bytes) else { continue };
		if vtxo.policy().policy_type() != VtxoPolicyKind::Pubkey {
			continue;
		}
		if vtxo.server_pubkey() != server_pubkey {
			continue;
		}
		if let Some(kp) = keystore.keypair_for(&vtxo.user_pubkey()) {
			ret.push((vtxo, *kp));
		}
	}
	ret
}

/// Read every arkoor VTXO in mailbox `M`, authorized by the mailbox key.
/// Same request a stock bark wallet's `sync_mailbox` sends.
pub async fn read_arkoor_mailbox(
	srv: &mut ServerConnection,
	mailbox_key: &Keypair,
) -> anyhow::Result<Vec<Vec<u8>>> {
	use protos::mailbox_server::mailbox_message::Message;

	let expiry = chrono::Local::now() + std::time::Duration::from_secs(600);
	let auth = MailboxAuthorization::new(mailbox_key, expiry);
	let mut checkpoint = 0;
	let mut ret = Vec::new();
	loop {
		let resp = srv.mailbox_client.read_mailbox(protos::mailbox_server::MailboxRequest {
			mailbox_id: auth.mailbox().serialize(),
			authorization: Some(auth.serialize()),
			checkpoint,
		}).await.context("read_mailbox")?.into_inner();
		for msg in resp.messages {
			checkpoint = checkpoint.max(msg.checkpoint);
			if let Some(Message::Arkoor(arkoor)) = msg.message {
				ret.extend(arkoor.vtxos);
			}
		}
		if !resp.have_more {
			break;
		}
	}
	Ok(ret)
}

/// Ask the server for the VTXO's spend state, proving ownership with a
/// schnorr signature by `keypair` (`VtxoStatusAttestation`). The server
/// rejects the request unless the signature verifies against the VTXO's user
/// pubkey.
pub async fn query_vtxo_status(
	srv: &mut ServerConnection,
	vtxo: &Vtxo<Full>,
	keypair: &Keypair,
) -> anyhow::Result<i32> {
	let attestation = VtxoStatusAttestation::new(vtxo.id(), keypair);
	let resp = srv.client.get_vtxo_status(protos::GetVtxoStatusRequest {
		vtxo_id: vtxo.id().to_bytes().to_vec(),
		attestation: attestation.serialize(),
	}).await.context("get_vtxo_status")?.into_inner();
	Ok(resp.spend_state)
}

/// Spend a VTXO held under a tweaked key by an arkoor to `destination`,
/// claiming its full amount. This is bark's arkoor send
/// (`create_checkpointed_arkoor_with_vtxos` + registration + mailbox
/// delivery), signed with `u`.
pub async fn spend_tweaked_vtxo(
	srv: &mut ServerConnection,
	vtxo: Vtxo<Full>,
	keypair: &Keypair,
	destination: &ark::Address,
) -> anyhow::Result<Vec<Vtxo<Full>>> {
	if vtxo.user_pubkey() != keypair.public_key() {
		bail!("keypair doesn't own vtxo {}", vtxo.id());
	}

	srv.client.register_vtxo_transactions(protos::RegisterVtxoTransactionsRequest {
		vtxos: vec![vtxo.serialize()],
	}).await.context("register input")?;

	let builder = ArkoorPackageBuilder::new_claim_all_with_checkpoints(
		[vtxo], destination.policy().clone(),
	)?.generate_user_nonces(&[*keypair])?;

	let resp = srv.client.request_arkoor_cosign(
		protos::ArkoorPackageCosignRequest::from(builder.cosign_request()),
	).await.context("request_arkoor_cosign")?.into_inner();
	let cosign = ArkoorPackageCosignResponse::try_from(resp)
		.context("bad cosign response")?;

	let signed = builder.user_cosign(&[*keypair], cosign)?.build_signed_vtxos();

	srv.client.register_vtxo_transactions(protos::RegisterVtxoTransactionsRequest {
		vtxos: signed.iter().map(|v| v.serialize()).collect(),
	}).await.context("register outputs")?;

	for d in destination.delivery() {
		if let ark::address::VtxoDelivery::ServerMailbox { blinded_id } = d {
			srv.mailbox_client.post_arkoor_message(protos::mailbox_server::PostArkoorMessageRequest {
				blinded_id: blinded_id.as_ref().to_vec(),
				vtxos: signed.iter().map(|v| v.serialize().to_vec()).collect(),
			}).await.context("post_arkoor_message")?;
		}
	}

	Ok(signed)
}
