//!  File: web/routes/mcp.rs
//!  Description: Opens up MCP endpoint for AI agents
//!
//!  Author: Roman Lancos <support@prieco.net>
//!  License: AGPL v3.0
//!
//!  Date Created: 2026-11-01
//!  Last Modified: 2026-11-01
//!
//!  Usage: claude mcp add --transport http prieco "https://prieco.net/mcp?a=<API key>"
//!  TODO:

/*
  Import external libraries
*/
use rocket::{State, get, http::Status, post, serde::json::Json};
use serde_json::{Value, json};

/*
  Import own libraries
*/
use crate::web::routes::{apis::api, pages::ClientIp};
use prieco_core::globals::EmbeddingService;

#[post("/mcp?<a>", data = "<body>")]
pub async fn mcp(
    a: Option<&str>,
    body: Json<Value>,
    client_ip: ClientIp,
    embedding_service: &State<EmbeddingService>,
) -> Result<Json<Value>, Status> {
    let id = body.get("id").cloned().ok_or(Status::Accepted)?;
    let params = &body["params"];

    let result = match body["method"].as_str() {
        Some("initialize") => json!({
            "protocolVersion": params["protocolVersion"],
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "PriEco", "version": "3.0.0" }
        }),
        Some("ping") => json!({}),
        Some("tools/list") => json!({
            "tools": [{
                "name": "web_search",
                "description": "Search the web with PriEco. It's an independent, EU, privacy-focused search engine. Returns url, title, description. lang: language code like \"en\" or \"all\"; loc: lowercase country code like \"us\" or \"all\", page content (first 500 chars), safe search and image URL.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string" },
                        "lang": { "type": "string" },
                        "loc": { "type": "string" }
                    },
                    "required": ["query"]
                },
                "annotations": { "readOnlyHint": true, "openWorldHint": true }
            }]
        }),
        Some("tools/call") => {
            let args = &params["arguments"];

            // Call API
            let response = api(
                a,
                args["lang"].as_str().unwrap_or("en"),
                args["loc"].as_str().unwrap_or("us"),
                args["query"].as_str().unwrap_or(""),
                None,
                None,
                Some(true),
                ClientIp(client_ip.0),
                embedding_service,
            )
            .await;

            // Process response
            let (text, is_error) = match response {
                Ok(Json(results)) => (serde_json::to_string(&results).unwrap_or_default(), false),
                Err((_, Json(error))) => (error.to_string(), true),
            };

            json!({
                "content": [{ "type": "text", "text": text }],
                "isError": is_error
            })
        }
        _ => {
            return Ok(Json(json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": "Method not found" }
            })));
        }
    };

    Ok(Json(
        json!({ "jsonrpc": "2.0", "id": id, "result": result }),
    ))
}

#[get("/mcp")]
pub fn mcp_get() -> Status {
    Status::MethodNotAllowed
}
