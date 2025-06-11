use crate::schema::{Commit, SignedCommit};
use atrium_api::types::string::{Did, Tid};
use atrium_crypto::did::parse_multikey;
use atrium_crypto::verify::Verifier;
use atrium_repo::Repository;
use atrium_repo::blockstore::{AsyncBlockStoreRead, CarStore};
use ipld_core::cid::Cid;
use serde::{Deserialize, Serialize};
use serde_ipld_dagcbor;
use std::io::Cursor;
use types_2048::blue;

// https://pds.baileytownsend.dev/xrpc/com.atproto.sync.getRecord?did=did:plc:rnpkyqnmsw4ipey6eotbdnnf&collection=blue.2048.game&rkey=3lr6w4y7tc222
//Hard coded test from my public multi key
const MULTI_KEY: &str = "zQ3shcesDYuekFas6nawd9zhsVDekFN45pAzbWNZWKpVTtKxx";
const TEST_CAR: &[u8] = include_bytes!("../3lr6w4y7tc222.car");

const RECORDS_CID: &str = "bafyreifcpk6rltiaqkmhb2xzl5ajcf6cgydf7qso5ymlnlwiczv33l6bdq";
const R_KEY: &str = "3lr6w4y7tc222";

#[tokio::main]
async fn main() {
    let mut bs = CarStore::open(Cursor::new(TEST_CAR)).await.unwrap();

    let mut root_commit: Option<SignedCommit> = None;
    let mut root_commit_cid: Option<Cid> = None;
    for root_cid in bs.roots() {
        root_commit_cid = Some(root_cid);
        println!("Root CID: {}", root_cid);
        let data = bs.read_block(root_cid).await.unwrap();
        root_commit = Some(serde_ipld_dagcbor::from_reader(&data[..]).unwrap());
    }
    let (alg, key) = parse_multikey(MULTI_KEY).unwrap();
    let verifier = Verifier::new(true);
    if let Some(commit) = root_commit {
        println!("Root commit: {:?}", commit.data);
        let mut repo = Repository::open(bs, root_commit_cid.unwrap())
            .await
            .unwrap();

        let data_to_verify = serde_ipld_dagcbor::to_vec(&Commit {
            did: commit.did.clone(),
            version: commit.version,
            data: commit.data,
            rev: commit.rev.clone(),
            prev: commit.prev,
        })
        .unwrap();
        println!("Verifying commit: {:?}", commit);
        let result = verifier.verify(alg, &key, &data_to_verify[..], &commit.sig.as_slice()[..]);
        if let Err(e) = result {
            println!("Error: {}", e);
        } else {
            println!("Verified!");
        }

        let data_from_car = repo
            .get::<blue::_2048::Game>(R_KEY.parse().unwrap())
            .await
            .unwrap();
        println!("Data from car: {:?}", data_from_car);
    }
}
pub mod schema {
    use super::*;

    /// Commit data
    ///
    /// Defined in: https://atproto.com/specs/repository
    ///
    /// https://github.com/bluesky-social/atproto/blob/c34426fc55e8b9f28d9b1d64eab081985d1b47b5/packages/repo/src/types.ts#L12-L19
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct Commit {
        /// the account DID associated with the repo, in strictly normalized form (eg, lowercase as appropriate)
        pub did: Did,
        /// fixed value of 3 for this repo format version
        pub version: i64,
        /// pointer to the top of the repo contents tree structure (MST)
        pub data: Cid,
        /// revision of the repo, used as a logical clock. Must increase monotonically
        pub rev: Tid,
        /// pointer (by hash) to a previous commit object for this repository
        pub prev: Option<Cid>,
    }

    /// Signed commit data. This is the exact same as a [Commit], but with a
    /// `sig` field appended.
    ///
    /// Defined in: https://atproto.com/specs/repository
    ///
    /// https://github.com/bluesky-social/atproto/blob/c34426fc55e8b9f28d9b1d64eab081985d1b47b5/packages/repo/src/types.ts#L22-L29
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct SignedCommit {
        /// the account DID associated with the repo, in strictly normalized form (eg, lowercase as appropriate)
        pub did: Did,
        /// fixed value of 3 for this repo format version
        pub version: i64,
        /// pointer to the top of the repo contents tree structure (MST)
        pub data: Cid,
        /// revision of the repo, used as a logical clock. Must increase monotonically
        pub rev: Tid,
        /// pointer (by hash) to a previous commit object for this repository
        pub prev: Option<Cid>,
        /// cryptographic signature of this commit, as raw bytes
        #[serde(with = "serde_bytes")]
        pub sig: Vec<u8>,
    }
}
