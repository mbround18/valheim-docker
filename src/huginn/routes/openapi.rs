use serde_json::json;
use warp::reply::{json, Json};

pub fn invoke() -> Json {
  let spec = json!({
    "openapi": "3.0.3",
    "info": {
      "title": "Valheim Docker API",
      "version": "1.0.0"
    },
    "paths": {
      "/status": { "get": { "summary": "Server status", "responses": {"200": {"description": "OK"}}}},
      "/metrics": { "get": { "summary": "Prometheus metrics", "responses": {"200": {"description": "OK"}}}},
      "/health": { "get": { "summary": "Health status", "responses": {"200": {"description": "Online"}, "503": {"description": "Offline"}}}},
      "/players": { "get": { "summary": "Players info", "responses": {"200": {"description": "OK"}}}},
      "/openapi.json": { "get": { "summary": "OpenAPI spec", "responses": {"200": {"description": "OK"}}}}
    }
  });

  json(&spec)
}
