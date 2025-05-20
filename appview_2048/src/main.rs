// Copyright 2021 Oxide Computer Company

//! Example using Dropshot to serve files

use dropshot::ConfigLogging;
use dropshot::ConfigLoggingLevel;
use dropshot::HttpError;
use dropshot::RequestContext;
use dropshot::ServerBuilder;
use dropshot::{ApiDescription, ConfigDropshot};
use dropshot::{Body, HttpResponseOk};
use dropshot::{Path, endpoint};
use http::{Response, StatusCode};
use schemars::JsonSchema;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

/// Our context is simply the root of the directory we want to serve.
struct FileServerContext {
    base: PathBuf,
}

#[derive(Deserialize)]
struct MyAppConfig {
    http_api_server: ConfigDropshot,
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let config: MyAppConfig = match fs::read_to_string("Dev.toml") {
        //TODO bad unwrap
        Ok(config) => toml::from_str(&config).unwrap_or_else(|e| {
            println!("Error parsing config file: {}", e);
            MyAppConfig {
                http_api_server: ConfigDropshot::default(),
            }
        }),
        Err(_) => {
            println!("Error reading config file");
            MyAppConfig {
                http_api_server: ConfigDropshot::default(),
            }
        }
    };
    // See dropshot/examples/basic.rs for more details on most of these pieces.
    let config_logging = ConfigLogging::StderrTerminal {
        level: ConfigLoggingLevel::Info,
    };
    let log = config_logging
        .to_logger("example-basic")
        .map_err(|error| format!("failed to create logger: {}", error))?;

    let mut api = ApiDescription::new();
    api.register(example_api_get_counter).unwrap();
    // api.register(static_content).unwrap();

    let context = FileServerContext {
        base: PathBuf::from("../../client_2048/dist/"),
    };

    let server = ServerBuilder::new(api, context, log)
        .config(config.http_api_server)
        .start()
        .map_err(|error| format!("failed to create server: {}", error))?;

    server.await
}

/// Fetch the current value of the counter.
#[endpoint {
    method = GET,
    path = "/api/test",
    }]
async fn example_api_get_counter(
    request_context: RequestContext<FileServerContext>,
) -> Result<HttpResponseOk<String>, HttpError> {
    let api_context = request_context.context();

    Ok(HttpResponseOk("Nice".to_string()))
}

/// Dropshot deserializes the input path into this Vec.
#[derive(Deserialize, JsonSchema)]
struct AllPath {
    path: Vec<String>,
}
