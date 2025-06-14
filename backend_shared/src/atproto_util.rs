use crate::atproto_util::schema::{Commit, SignedCommit};
use atrium_api::agent::atp_agent::{AtpAgent, AtpSession};
use atrium_api::did_doc::DidDocument;
use atrium_api::types::Collection;
use atrium_api::types::string::{Cid, RecordKey};
use atrium_common::store::memory::MemoryStore;
use atrium_crypto::did::parse_multikey;
use atrium_crypto::verify::Verifier;
use atrium_repo::Repository;
use atrium_repo::blockstore::{AsyncBlockStoreRead, CarStore, Error};
use atrium_xrpc_client::reqwest::ReqwestClient;
use serde::Deserialize;
use serde_ipld_dagcbor::{DecodeError, EncodeError};
use std::collections::TryReserveError;
use std::io::Cursor;
use thiserror::Error;
use types_2048::blue;
use types_2048::blue::_2048::game::Record;

pub struct ParsedDIDDoc {
    pub did: String,
    pub pds_url: String,
    pub handle: Option<String>,
    pub multi_key: Option<String>,
}

pub fn parse_did_doc(did_doc: DidDocument) -> Result<ParsedDIDDoc, String> {
    let handle = match did_doc.also_known_as {
        None => None,
        Some(also_known_as) => {
            match also_known_as.is_empty() {
                true => None,
                false => {
                    //also_known as a list starts the array with the highest priority handle
                    let formatted_handle = format!("@{}", also_known_as[0]).replace("at://", "");
                    Some(formatted_handle)
                }
            }
        }
    };

    let pds_url = match did_doc.service.as_ref().and_then(|services| {
        services
            .iter()
            .find(|service| service.r#type == "AtprotoPersonalDataServer")
            .map(|service| service.service_endpoint.clone())
    }) {
        None => {
            log::error!("No pds url found for {}", &did_doc.id);
            return Err("No pds url found".to_string());
        }
        Some(url) => url,
    };

    let multi_key = match did_doc.verification_method.as_ref().and_then(|services| {
        services
            .iter()
            .find(|service| service.r#type == "Multikey" && service.controller == did_doc.id)
            .map(|service| service.public_key_multibase.clone())
    }) {
        None => {
            log::error!("No pds url found for {}", &did_doc.id);
            return Err("No pds url found".to_string());
        }
        Some(url) => url,
    };

    Ok(ParsedDIDDoc {
        did: did_doc.id,
        pds_url,
        handle,
        multi_key,
    })
}

#[derive(Debug, Error, PartialEq)]
pub enum RecordValidationError {
    #[error("Operation not implemented yet")]
    NotImplementedYet,
    #[error("Error getting the CAR file")]
    ErrorGettingTheCar,
    #[error("Error reading the CAR file: {0}")]
    ErrorReadingTheCar(String),
    #[error("No multikey found")]
    NoMultiKey,
    #[error("Multikey could not be parsed")]
    MultiKeyCouldNotBeParsed,
    #[error("Record not verified")]
    NotVerified,
}

pub async fn get_and_validate_record<T: atrium_api::types::Collection>(
    agent: &AtpAgent<MemoryStore<(), AtpSession>, ReqwestClient>,
    doc: &ParsedDIDDoc,
    cid: Cid,
    record_key: RecordKey,
    collection: &str,
) -> Result<Option<T::Record>, RecordValidationError>
where
    T: Collection,
{
    let multi_key = match doc.multi_key.as_ref() {
        None => {
            return Err(RecordValidationError::NoMultiKey);
        }
        Some(key) => key.as_str(),
    };

    let car_buffer = match agent
        .api
        .com
        .atproto
        .sync
        .get_record(
            atrium_api::com::atproto::sync::get_record::ParametersData {
                collection: collection.parse().unwrap(),
                did: doc.did.parse().unwrap(),
                rkey: record_key.clone(),
            }
            .into(),
        )
        .await
    {
        Ok(buffer) => buffer,
        Err(e) => {
            log::error!("Error getting record: {}", e);
            return Err(RecordValidationError::NotImplementedYet);
        }
    };

    let mut bs = CarStore::open(Cursor::new(car_buffer)).await.unwrap();
    // log::info!("roots: {:#?}", bs.roots());
    let root_cid = match bs.roots().next() {
        None => {
            return Err(RecordValidationError::ErrorReadingTheCar(String::from(
                "No root CID found in CAR file",
            )));
        }
        Some(root_cid) => root_cid,
    };
    log::info!("Root CID: {}", root_cid);
    let root_commit: SignedCommit = match bs.read_block(root_cid).await {
        Ok(bytes) => match serde_ipld_dagcbor::from_reader(&bytes[..]) {
            Ok(commit) => commit,
            Err(err) => {
                return Err(RecordValidationError::ErrorReadingTheCar(err.to_string()));
            }
        },
        Err(err) => {
            return Err(RecordValidationError::ErrorReadingTheCar(err.to_string()));
        }
    };

    let (alg, key) = match parse_multikey(multi_key) {
        Ok((alg, key)) => (alg, key),
        Err(err) => {
            return Err(RecordValidationError::MultiKeyCouldNotBeParsed);
        }
    };
    //TODO may need to pass in true?
    let verifier = Verifier::new(false);

    let mut repo = match Repository::open(bs, root_cid).await {
        Ok(repo) => repo,
        Err(err) => {
            return Err(RecordValidationError::ErrorReadingTheCar(err.to_string()));
        }
    };

    let data_to_verify = match serde_ipld_dagcbor::to_vec(&Commit {
        did: root_commit.did.clone(),
        version: root_commit.version,
        data: root_commit.data,
        rev: root_commit.rev.clone(),
        prev: root_commit.prev,
    }) {
        Ok(data) => data,
        Err(err) => {
            return Err(RecordValidationError::ErrorReadingTheCar(err.to_string()));
        }
    };

    let result = verifier.verify(alg, &key, &data_to_verify[..], &root_commit.sig[..]);
    if let Err(err) = result {
        log::error!("Error verifying record: {}", err);
        return Err(RecordValidationError::NotImplementedYet);
    }

    let possible_record = match repo.get::<T>(record_key).await {
        Ok(record) => record,
        Err(err) => {
            return Err(RecordValidationError::ErrorReadingTheCar(err.to_string()));
        }
    };

    log::info!("Verifying record: {:?}", possible_record);

    match possible_record {
        None => Ok(None),
        Some(record) => Ok(Some(record)),
    }
}

pub mod schema {
    use super::*;
    use atrium_api::types::string::{Did, Tid};
    use serde::{Deserialize, Serialize};

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
        pub data: ipld_core::cid::Cid,
        /// revision of the repo, used as a logical clock. Must increase monotonically
        pub rev: Tid,
        /// pointer (by hash) to a previous commit object for this repository
        pub prev: Option<ipld_core::cid::Cid>,
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
        pub data: ipld_core::cid::Cid,
        /// revision of the repo, used as a logical clock. Must increase monotonically
        pub rev: Tid,
        /// pointer (by hash) to a previous commit object for this repository
        pub prev: Option<ipld_core::cid::Cid>,
        /// cryptographic signature of this commit, as raw bytes
        #[serde(with = "serde_bytes")]
        pub sig: Vec<u8>,
    }
}
