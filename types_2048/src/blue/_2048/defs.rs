// @generated - This file is generated by esquema-codegen (forked from atrium-codegen). DO NOT EDIT.
//!Definitions for the `blue.2048.defs` namespace.
//!Reusable types for blue.2048 lexicons
///The sync status for a record used to help sync between your ATProto record and local record.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatusData {
    pub created_at: atrium_api::types::string::Datetime,
    ///A XXH3 hash of the record to tell if anything has changed
    pub hash: String,
    ///A flag to know if it has been synced with the AT repo. Used mostly client side to filter what records need syncing
    pub synced_with_at_repo: bool,
    pub updated_at: atrium_api::types::string::Datetime,
}

//TODO flatten strikes again.
pub type SyncStatus = atrium_api::types::Object<SyncStatusData>;
