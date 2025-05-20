#[allow(dead_code)]
use atrium_identity::handle::DnsTxtResolver;
use gloo::net::http::Request;
use serde::{Deserialize, Serialize};

/// Setup for dns resolver for the handle resolver
pub struct ApiDNSTxtResolver;

impl Default for ApiDNSTxtResolver {
    fn default() -> Self {
        Self {}
    }
}

// curl --http2 --header "accept: application/dns-json" "https://one.one.one.one/dns-query?name=_atproto.baileytownsend.dev&type=TXT"
impl DnsTxtResolver for ApiDNSTxtResolver {
    async fn resolve(
        &self,
        query: &str,
    ) -> core::result::Result<Vec<String>, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let request_url = format!(
            "https://one.one.one.one/dns-query?name={}&type=TXT",
            query.to_lowercase()
        );
        let resp = Request::get(request_url.as_str())
            .header("accept", "application/dns-json")
            .send()
            .await;

        let resp = resp?.json::<DnsResponse>().await?;

        let response_data = resp
            .Answer
            .iter()
            .map(|a| a.data.clone().replace("\"", ""))
            .collect::<Vec<String>>();
        Ok(response_data)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct DnsResponse {
    pub Status: i32,
    pub TC: bool,
    pub RD: bool,
    pub RA: bool,
    pub AD: bool,
    pub CD: bool,
    pub Question: Vec<Question>,
    pub Answer: Vec<Answer>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Question {
    pub name: String,
    #[serde(rename = "type")]
    pub type_: i32,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Answer {
    pub name: String,
    #[serde(rename = "type")]
    pub type_: i32,
    pub TTL: i32,
    pub data: String,
}
