//! End-to-end on regtest: a stock captaind (built unmodified from bark
//! c419159) co-signs and delivers an SPc/Ark-tier payout to a tweaked key,
//! and the miner client in this crate claims and spends it.
//!
//! Run with `./run_e2e.sh` (sets BITCOIND_EXEC / CAPTAIND_EXEC).

use bitcoin::secp256k1::{Keypair, PublicKey, Secp256k1, SecretKey};
use bitcoin::Amount;

use ark::mailbox::{BlindedMailboxIdentifier, MailboxIdentifier};
use ark::vtxo::{Full, Vtxo};
use ark_testing::{sat, TestContext};
use server::database::Db;
use server_rpc::ServerConnection;

use spc_ark::{ark_user_pubkey, build_bark_address, pool_blinded_mailbox_id, tier_tweaks, SharedSecret};
use spc_ark_client::{
	claim_vtxos, query_vtxo_status, read_arkoor_mailbox, spend_tweaked_vtxo, MinerKeys, PoolBlock,
	SpArkKeystore,
};

#[tokio::test]
async fn spc_ark_payout_on_stock_server() {
	let secp = Secp256k1::new();
	let ctx = TestContext::new("spc_ark/payout_on_stock_server").await;
	let srv = ctx.captaind("server").create().await;

	// --- Miner: one BIP39 seed backs both the stock bark wallet and the SP keys.
	let mnemonic = bip39::Mnemonic::generate(12).unwrap();
	let keys = MinerKeys::from_bip39_seed(&mnemonic.to_seed(""));
	let miner = ctx.bark_sdk("miner", &srv).mnemonic(mnemonic.clone()).create().await;
	let m: MailboxIdentifier = miner.mailbox_identifier();
	assert_eq!(m, MailboxIdentifier::from_pubkey(keys.mailbox.public_key()),
		"our m/350'/1' derivation must equal bark's wallet mailbox id");

	let ark_info = srv.ark_info().await;
	let s_mb = ark_info.mailbox_pubkey;
	let server_pk = ark_info.server_pubkey;

	// Registration (private, once per server): sp1q (B_scan, B_spend) + (M, D).
	let d = keys.mailbox_hint(&s_mb).unwrap();
	let b_scan_pub = keys.scan.public_key();
	let b_ark_pub = keys.ark.public_key();

	// --- Pool: a funded stock bark wallet.
	let pool = ctx.bark_sdk("pool", &srv).boarded(sat(400_000)).create().await;

	// The pool "finds" a block at this height with ephemeral A_send in its tag.
	// (The height only feeds input_hash; the miner reads both from the tag.)
	let height = 201;
	let a_send = SecretKey::new(&mut bitcoin::secp256k1::rand::thread_rng());
	let a_send_pub = PublicKey::from_secret_key(&secp, &a_send);
	let ecdh = SharedSecret::pool_side(height, &a_send, &b_scan_pub).unwrap();
	let t_ark = tier_tweaks(&ecdh, 0).unwrap().t_ark;
	let u_pub = ark_user_pubkey(&b_ark_pub, &t_ark).unwrap();
	let blinded = pool_blinded_mailbox_id(&m, &d, &t_ark, &s_mb).unwrap();
	let address = build_bark_address(&server_pk, &u_pub, blinded, true);
	println!("U (tweaked user pubkey) = {u_pub}");
	println!("M (miner mailbox id)    = {}", m.as_pubkey());
	println!("blinded = M + D + t*S   = {}", blinded.as_pubkey());
	println!("pool-built address      = {address}");

	// Stock bark send: cosign + register + post_arkoor_message(blinded).
	let payout = sat(100_000);
	pool.send_arkoor_payment(&address, payout).await.expect("stock server must cosign arkoor to U");
	assert!(pool.pending_arkoor_sends().await.unwrap().is_empty(), "send must have finalized");

	// --- Evidence from the server's own DB: what id did it unblind to?
	let db = Db::connect(&srv.config().postgres).await.unwrap();
	let rows: Vec<(String, String, i64)> = db.read(async |t| {
		let rows = t.query(
			"SELECT unblinded_mailbox_id, vtxo_id, checkpoint FROM mailbox WHERE mailbox_type = 'arkoor-receive'",
			&[],
		).await?;
		Ok(rows.iter().map(|r| (r.get(0), r.get(1), r.get(2))).collect())
	}).await.unwrap();
	println!("server mailbox rows (arkoor-receive): {rows:?}");
	let m_hex = m.to_string();
	let ours = rows.iter().filter(|r| r.0 == m_hex).collect::<Vec<_>>();
	assert_eq!(ours.len(), 1, "server must have stored exactly one VTXO under unblinded id M={m_hex}");
	println!("SERVER UNBLINDED TO {} == M: {}", ours[0].0, ours[0].0 == m_hex);

	// --- Stock miner wallet: drops the VTXO (no key for U).
	miner.sync().await;
	assert!(miner.spendable_vtxos().await.unwrap().is_empty(),
		"stock bark wallet should refuse a VTXO it can't sign for");
	println!("stock bark wallet spendable after sync: 0 (as expected)");

	// --- Modified client: scan pool blocks, read mailbox M, claim.
	let mut conn = ServerConnection::builder()
		.address(srv.ark_url())
		.network(bitcoin::Network::Regtest)
		.connect().await.unwrap();
	let keystore = SpArkKeystore::scan(
		&keys.scan.secret_key(), &keys.ark.secret_key(),
		&[PoolBlock { height, a_send: a_send_pub }], 4,
	).unwrap();
	let raw = read_arkoor_mailbox(&mut conn, &keys.mailbox).await.unwrap();
	assert_eq!(raw.len(), 1);
	let claimed = claim_vtxos(&raw, server_pk, &keystore);
	assert_eq!(claimed.len(), 1, "client must recognise U from its SP keystore");
	let (vtxo, u_kp): (Vtxo<Full>, Keypair) = claimed.into_iter().next().unwrap();
	assert_eq!(vtxo.user_pubkey(), u_pub);
	assert_eq!(vtxo.amount(), payout);
	assert_eq!(vtxo.id().to_string(), ours[0].1, "mailbox row is this vtxo");
	miner.validate_vtxo(&vtxo).await.expect("vtxo chain must validate against on-chain anchor");
	println!("claimed vtxo {} amount {} user_pubkey {}", vtxo.id(), vtxo.amount(), vtxo.user_pubkey());

	// Ownership proof 1: the server verifies a schnorr sig by u over the vtxo id.
	let state = query_vtxo_status(&mut conn, &vtxo, &u_kp).await.unwrap();
	assert_eq!(state, 1, "VTXO_SPEND_STATE_SPENDABLE");
	println!("server accepted u-signed VtxoStatusAttestation: spend_state={state} (SPENDABLE)");

	// Control: a wrong key is rejected by the same RPC.
	let wrong = Keypair::new(&secp, &mut bitcoin::secp256k1::rand::thread_rng());
	assert!(query_vtxo_status(&mut conn, &vtxo, &wrong).await.is_err());

	// Ownership proof 2: actually spend it with u, into the miner's normal
	// (stock) bark address, then the stock wallet receives it.
	let miner_addr = miner.new_address().await.unwrap();
	let signed = spend_tweaked_vtxo(&mut conn, vtxo.clone(), &u_kp, &miner_addr).await
		.expect("stock server must cosign an arkoor spent by u");
	assert_eq!(signed.iter().map(|v| v.amount()).sum::<Amount>(), payout);
	println!("spent tweaked vtxo via arkoor; outputs: {:?}",
		signed.iter().map(|v| (v.id(), v.amount())).collect::<Vec<_>>());

	let state = query_vtxo_status(&mut conn, &vtxo, &u_kp).await.unwrap();
	assert_eq!(state, 2, "VTXO_SPEND_STATE_SPENT");

	miner.sync().await;
	let got = miner.spendable_vtxos().await.unwrap();
	let total = got.iter().map(|v| v.amount()).sum::<Amount>();
	assert_eq!(total, payout, "stock miner wallet now holds the payout");
	println!("stock miner wallet spendable after spend-to-self: {total}");

	// Sanity: the pool's blinding equals bark's own blinding with u.
	let bark_blind = BlindedMailboxIdentifier::new(m, s_mb, &u_kp).unwrap();
	assert_eq!(bark_blind, blinded);
}
