use serde_json::json;
use warp::reply::{json, Json};

pub fn invoke() -> Json {
  let spec = json!({
    "openapi": "3.0.3",
    "info": {
      "title": "Valheim Docker API",
      "version": "1.0.0",
      "description": "REST API for monitoring and managing Valheim Docker server"
    },
    "components": {
      "schemas": {
        "StatusResponse": {
          "type": "object",
          "required": [
            "name",
            "version",
            "players",
            "max_players",
            "map",
            "online",
            "bepinex",
            "jobs",
            "scheduler_state"
          ],
          "properties": {
            "name": { "type": "string" },
            "version": { "type": "string" },
            "players": { "type": "integer", "format": "uint8", "minimum": 0, "maximum": 255 },
            "max_players": { "type": "integer", "format": "uint8", "minimum": 0, "maximum": 255 },
            "map": { "type": "string" },
            "online": { "type": "boolean" },
            "bepinex": { "$ref": "#/components/schemas/BepInExInfo" },
            "jobs": {
              "type": "array",
              "items": { "$ref": "#/components/schemas/JobInfo" }
            },
            "scheduler_state": { "$ref": "#/components/schemas/SchedulerState" }
          }
        },
        "BepInExInfo": {
          "type": "object",
          "required": ["enabled", "mods"],
          "properties": {
            "enabled": { "type": "boolean" },
            "mods": {
              "type": "array",
              "items": { "$ref": "#/components/schemas/BepInExMod" }
            }
          }
        },
        "BepInExMod": {
          "type": "object",
          "required": ["name", "location"],
          "properties": {
            "name": { "type": "string" },
            "location": { "type": "string" },
            "version": { "type": "string", "nullable": true }
          }
        },
        "JobInfo": {
          "type": "object",
          "required": ["name", "enabled", "schedule"],
          "properties": {
            "name": { "type": "string" },
            "enabled": { "type": "boolean" },
            "schedule": { "type": "string" }
          }
        },
        "SchedulerState": {
          "type": "object",
          "required": ["updated_at", "jobs"],
          "properties": {
            "updated_at": { "type": "string", "nullable": true },
            "jobs": {
              "type": "array",
              "items": { "$ref": "#/components/schemas/JobRuntimeInfo" }
            }
          }
        },
        "JobRuntimeInfo": {
          "type": "object",
          "required": [
            "name",
            "last_started_at",
            "last_finished_at",
            "last_status",
            "last_message",
            "last_exit_code",
            "last_duration_ms",
            "run_count",
            "success_count",
            "failure_count"
          ],
          "properties": {
            "name": { "type": "string" },
            "last_started_at": { "type": "string", "nullable": true },
            "last_finished_at": { "type": "string", "nullable": true },
            "last_status": { "type": "string", "nullable": true },
            "last_message": { "type": "string", "nullable": true },
            "last_exit_code": { "type": "integer", "nullable": true },
            "last_duration_ms": { "type": "integer", "format": "uint64", "nullable": true },
            "run_count": { "type": "integer", "format": "uint64", "minimum": 0 },
            "success_count": { "type": "integer", "format": "uint64", "minimum": 0 },
            "failure_count": { "type": "integer", "format": "uint64", "minimum": 0 }
          }
        },
        "HealthResponse": {
          "type": "object",
          "required": ["online", "name"],
          "properties": {
            "online": { "type": "boolean" },
            "name": { "type": "string" }
          }
        },
        "LivenessResponse": {
          "type": "object",
          "required": ["alive"],
          "properties": {
            "alive": { "type": "boolean" }
          }
        },
        "ReadinessResponse": {
          "type": "object",
          "required": ["ready", "name"],
          "properties": {
            "ready": { "type": "boolean" },
            "name": { "type": "string" }
          }
        },
        "ModsResponse": {
          "type": "object",
          "required": ["installed_mods", "count"],
          "properties": {
            "installed_mods": {
              "type": "array",
              "items": { "$ref": "#/components/schemas/InstallableMod" }
            },
            "count": { "type": "integer", "format": "uint64", "minimum": 0 }
          }
        },
        "InstallableMod": {
          "type": "object",
          "required": ["name", "version", "dependencies"],
          "properties": {
            "name": { "type": "string" },
            "version": { "type": "string", "nullable": true },
            "dependencies": {
              "type": "array",
              "items": { "type": "string" },
              "nullable": true
            }
          }
        },
        "PlayersResponse": {
          "type": "object",
          "required": ["online", "players", "max_players", "names"],
          "properties": {
            "online": { "type": "boolean" },
            "players": { "type": "integer", "format": "uint8", "minimum": 0, "maximum": 255 },
            "max_players": { "type": "integer", "format": "uint8", "minimum": 0, "maximum": 255 },
            "names": {
              "type": "array",
              "items": { "type": "string" }
            }
          }
        },
        "MetadataResponse": {
          "type": "object",
          "required": ["service", "odin"],
          "properties": {
            "service": { "$ref": "#/components/schemas/ServiceMetadata" },
            "odin": { "$ref": "#/components/schemas/OdinMetadata" }
          }
        },
        "ServiceMetadata": {
          "type": "object",
          "required": [
            "name",
            "version",
            "http_bind",
            "http_port",
            "info_cache_ttl_secs",
            "uptime_seconds"
          ],
          "properties": {
            "name": { "type": "string" },
            "version": { "type": "string" },
            "http_bind": { "type": "string" },
            "http_port": { "type": "integer", "format": "uint16", "minimum": 0, "maximum": 65535 },
            "info_cache_ttl_secs": { "type": "integer", "format": "uint64", "minimum": 0 },
            "uptime_seconds": { "type": "integer", "format": "uint64", "minimum": 0 }
          }
        },
        "OdinMetadata": {
          "type": "object",
          "required": [
            "game_id",
            "game_port",
            "query_port",
            "query_address",
            "current_build_id",
            "public_server",
            "crossplay_enabled",
            "validate_on_install",
            "staged_updates",
            "clean_install",
            "beta",
            "jobs"
          ],
          "properties": {
            "game_id": { "type": "integer", "format": "int64" },
            "game_port": { "type": "integer", "format": "uint16", "minimum": 0, "maximum": 65535 },
            "query_port": { "type": "integer", "format": "uint16", "minimum": 0, "maximum": 65535 },
            "query_address": { "type": "string" },
            "current_build_id": { "type": "string", "nullable": true },
            "public_server": { "type": "boolean" },
            "crossplay_enabled": { "type": "boolean" },
            "validate_on_install": { "type": "boolean" },
            "staged_updates": { "type": "boolean" },
            "clean_install": { "type": "boolean" },
            "beta": { "$ref": "#/components/schemas/BetaMetadata" },
            "jobs": { "$ref": "#/components/schemas/JobsMetadata" }
          }
        },
        "BetaMetadata": {
          "type": "object",
          "required": ["use_public_beta", "branch", "backwards_compatible_branch"],
          "properties": {
            "use_public_beta": { "type": "boolean" },
            "branch": { "type": "string" },
            "backwards_compatible_branch": { "type": "boolean" }
          }
        },
        "JobsMetadata": {
          "type": "object",
          "required": [
            "auto_update_enabled",
            "auto_update_schedule",
            "auto_backup_enabled",
            "auto_backup_schedule",
            "scheduled_restart_enabled",
            "scheduled_restart_schedule"
          ],
          "properties": {
            "auto_update_enabled": { "type": "boolean" },
            "auto_update_schedule": { "type": "string" },
            "auto_backup_enabled": { "type": "boolean" },
            "auto_backup_schedule": { "type": "string" },
            "scheduled_restart_enabled": { "type": "boolean" },
            "scheduled_restart_schedule": { "type": "string" }
          }
        },
        "ConnectUrlResponse": {
          "type": "object",
          "required": ["steam_url", "host", "port", "redirect"],
          "properties": {
            "steam_url": { "type": "string" },
            "host": { "type": "string" },
            "port": { "type": "integer", "format": "uint16", "minimum": 0, "maximum": 65535 },
            "redirect": { "type": "boolean" }
          }
        }
      }
    },
    "paths": {
      "/status": {
        "get": {
          "summary": "Server status",
          "description": "Get detailed server status information",
          "responses": {
            "200": {
              "description": "Current server status",
              "content": {
                "application/json": {
                  "schema": { "$ref": "#/components/schemas/StatusResponse" }
                }
              }
            }
          }
        }
      },
      "/connect/local": {
        "get": {
          "summary": "Steam local connect redirect",
          "description": "Redirects to steam://run/<CONNECT_STEAM_APP_ID>//+connect%20127.0.0.1:<PORT>. Browser CORS fetch clients receive JSON payload instead.",
          "responses": {
            "200": {
              "description": "Connect URL payload for browser CORS clients",
              "content": {
                "application/json": {
                  "schema": { "$ref": "#/components/schemas/ConnectUrlResponse" }
                }
              }
            },
            "302": {
              "description": "Redirect to local Steam connect URL"
            }
          }
        }
      },
      "/connect/remote": {
        "get": {
          "summary": "Steam remote connect redirect",
          "description": "Redirects to steam://run/<CONNECT_STEAM_APP_ID>//+connect%20<PUBLIC_HOST>:<PORT>. Browser CORS fetch clients receive JSON payload instead.",
          "responses": {
            "200": {
              "description": "Connect URL payload for browser CORS clients",
              "content": {
                "application/json": {
                  "schema": { "$ref": "#/components/schemas/ConnectUrlResponse" }
                }
              }
            },
            "302": {
              "description": "Redirect to remote Steam connect URL"
            }
          }
        }
      },
      "/metrics": {
        "get": {
          "summary": "Prometheus metrics",
          "description": "Get metrics in Prometheus format",
          "responses": {
            "200": {
              "description": "Prometheus metrics payload",
              "content": {
                "text/plain": {
                  "schema": { "type": "string" }
                }
              }
            }
          }
        }
      },
      "/health": {
        "get": {
          "summary": "Health status",
          "description": "Health check endpoint",
          "responses": {
            "200": {
              "description": "Server is online",
              "content": {
                "application/json": {
                  "schema": { "$ref": "#/components/schemas/HealthResponse" }
                }
              }
            },
            "503": {
              "description": "Server is offline",
              "content": {
                "application/json": {
                  "schema": { "$ref": "#/components/schemas/HealthResponse" }
                }
              }
            }
          }
        }
      },
      "/readiness": {
        "get": {
          "summary": "Kubernetes readiness probe",
          "description": "Returns 200 only if the Valheim server is actually online",
          "responses": {
            "200": {
              "description": "Server is ready",
              "content": {
                "application/json": {
                  "schema": { "$ref": "#/components/schemas/ReadinessResponse" }
                }
              }
            },
            "503": {
              "description": "Server is not ready",
              "content": {
                "application/json": {
                  "schema": { "$ref": "#/components/schemas/ReadinessResponse" }
                }
              }
            }
          }
        }
      },
      "/liveness": {
        "get": {
          "summary": "Kubernetes liveness probe",
          "description": "Returns 200 if the API service is alive",
          "responses": {
            "200": {
              "description": "Service is alive",
              "content": {
                "application/json": {
                  "schema": { "$ref": "#/components/schemas/LivenessResponse" }
                }
              }
            }
          }
        }
      },
      "/mods": {
        "get": {
          "summary": "Installed mods",
          "description": "Get information about installed mods",
          "responses": {
            "200": {
              "description": "Installed mods and count",
              "content": {
                "application/json": {
                  "schema": { "$ref": "#/components/schemas/ModsResponse" }
                }
              }
            }
          }
        }
      },
      "/players": {
        "get": {
          "summary": "Players info",
          "description": "Get information about players on the server",
          "responses": {
            "200": {
              "description": "Current player summary",
              "content": {
                "application/json": {
                  "schema": { "$ref": "#/components/schemas/PlayersResponse" }
                }
              }
            }
          }
        }
      },
      "/metadata": {
        "get": {
          "summary": "Safe runtime metadata",
          "description": "Get safe Odin/Huginn runtime metadata and job mode toggles",
          "responses": {
            "200": {
              "description": "Safe runtime metadata",
              "content": {
                "application/json": {
                  "schema": { "$ref": "#/components/schemas/MetadataResponse" }
                }
              }
            }
          }
        }
      },
      "/openapi.json": {
        "get": {
          "summary": "OpenAPI spec",
          "description": "Get the OpenAPI specification in JSON format",
          "responses": {"200": {"description": "OK"}}
        }
      },
      "/docs": {
        "get": {
          "summary": "API documentation",
          "description": "Interactive Swagger UI documentation",
          "responses": {"200": {"description": "OK"}}
        }
      }
    }
  });

  json(&spec)
}
