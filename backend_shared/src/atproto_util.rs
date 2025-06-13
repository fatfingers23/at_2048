use atrium_api::agent::atp_agent::{AtpAgent, AtpSession};
use atrium_api::did_doc::DidDocument;
use atrium_api::types::string::{Cid, RecordKey};
use atrium_common::store::memory::MemoryStore;
use atrium_xrpc_client::reqwest::ReqwestClient;
use thiserror::Error;

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
}

pub async fn get_and_validate_record<T>(
    agent: &AtpAgent<MemoryStore<(), AtpSession>, ReqwestClient>,
    doc: &ParsedDIDDoc,
    cid: Cid,
    record_key: RecordKey,
    collection: &str,
) -> Result<T, RecordValidationError> {
    let car_buffer = match agent
        .api
        .com
        .atproto
        .sync
        .get_record(
            atrium_api::com::atproto::sync::get_record::ParametersData {
                collection: collection.parse().unwrap(),
                did: doc.did.parse().unwrap(),
                rkey: record_key,
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

    Err(RecordValidationError::NotImplementedYet)
}
