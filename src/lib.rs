pub mod util;
use aws_config::BehaviorVersion;
use metrics::counter;
use rmcp::{Error as McpError, ServerHandler, const_string, model::*, schemars, tool};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone, schemars::JsonSchema, Serialize, Deserialize)]
pub struct ListDatabasesResult {
    pub databases: Vec<String>,
}

#[derive(Clone, schemars::JsonSchema, Serialize, Deserialize)]
pub struct DatabaseMetadata {
    pub name: String,
    pub tables: Vec<String>,
}

#[derive(Clone, schemars::JsonSchema, Serialize, Deserialize)]
pub struct TableMetadata {
    pub name: String,
    pub columns: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GlueDataCatalog {
    client: aws_sdk_glue::Client,
}

#[tool(tool_box)]
impl GlueDataCatalog {
    #[allow(dead_code)]
    pub fn new(client: aws_sdk_glue::Client) -> Self {
        Self { client }
    }

    /// Creates a new GlueDataCatalog using the default AWS configuration from environment
    #[allow(dead_code)]
    pub async fn from_env() -> Self {
        let config = aws_config::defaults(BehaviorVersion::latest()).load().await;
        let client = aws_sdk_glue::Client::new(&config);
        client
            .get_databases()
            .send()
            .await
            .expect("Couldn't connect to AWS");
        Self { client }
    }

    #[tool(description = "List the databases in an AWS Glue Data Catalog")]
    async fn list_databases(&self) -> Result<CallToolResult, McpError> {
        log::info!(
            "Listing databases in {}",
            self.client.config().region().unwrap()
        );
        counter!("calls.list_databases").increment(1);

        let response = self.client.get_databases().send().await.map_err(|e| {
            counter!("errors.list_databases.aws_call_error").increment(1);
            McpError::internal_error(
                "Failed to list databases",
                Some(json!({"error": e.to_string()})),
            )
        })?;

        let databases = response
            .database_list()
            .iter()
            .map(|db| db.name().into())
            .collect::<Vec<String>>();

        let result = ListDatabasesResult { databases };
        let json_result = serde_json::to_value(result).map_err(|e| {
            counter!("errors.list_databases.serde_error").increment(1);
            McpError::internal_error(
                "Failed to serialize result",
                Some(json!({"error": e.to_string()})),
            )
        })?;

        Ok(CallToolResult::success(vec![Content::json(json_result)?]))
    }

    #[tool(
        description = "Get database metadata from an AWS Glue Data Catalog, including the tables in the database"
    )]
    async fn get_database_metadata(
        &self,
        #[tool(param)]
        #[schemars(description = "The database name")]
        database_name: String,
    ) -> Result<CallToolResult, McpError> {
        log::info!("Getting tables for database {}", database_name);
        counter!("calls.get_database_metadata").increment(1);

        let response = self
            .client
            .get_tables()
            .database_name(database_name.clone())
            .send()
            .await
            .map_err(|e| {
                counter!("errors.get_database_metadata.aws_call_error").increment(1);
                McpError::internal_error(
                    "Failed to get tables",
                    Some(json!({"error": e.to_string()})),
                )
            })?;

        let tables = response
            .table_list()
            .iter()
            .map(|table| table.name().into())
            .collect::<Vec<String>>();

        let result = DatabaseMetadata {
            name: database_name,
            tables,
        };

        let json_result = serde_json::to_value(result).map_err(|e| {
            counter!("errors.get_database_metadata.serde_error").increment(1);
            McpError::internal_error(
                "Failed to serialize result",
                Some(json!({"error": e.to_string()})),
            )
        })?;

        Ok(CallToolResult::success(vec![Content::json(json_result)?]))
    }

    #[tool(
        description = "Get table metadata from an AWS Glue Data Catalog, including the columns in the table"
    )]
    async fn get_table_metadata(
        &self,
        #[tool(param)]
        #[schemars(description = "The database name")]
        database_name: String,
        #[tool(param)]
        #[schemars(description = "The table name")]
        table_name: String,
    ) -> Result<CallToolResult, McpError> {
        log::info!("Getting columns for table {}", table_name);
        counter!("calls.get_table_metadata").increment(1);

        let response = self
            .client
            .get_table()
            .database_name(database_name)
            .name(table_name.clone())
            .send()
            .await
            .map_err(|e| {
                counter!("errors.get_table_metadata.aws_call_error").increment(1);
                McpError::internal_error(
                    "Failed to get table metadata",
                    Some(json!({"error": e.to_string()})),
                )
            })?;

        let columns = response
            .table()
            .and_then(|table| table.storage_descriptor())
            .map(|sd| sd.columns())
            .unwrap_or_default()
            .iter()
            .map(|col| col.name().into())
            .collect::<Vec<String>>();

        log::info!("Got {} columns for table {}", columns.len(), table_name);

        let result = TableMetadata {
            name: table_name,
            columns,
        };

        let json_result = serde_json::to_value(result).map_err(|e| {
            counter!("errors.get_table_metadata.serde_error").increment(1);
            McpError::internal_error(
                "Failed to serialize result",
                Some(json!({"error": e.to_string()})),
            )
        })?;

        Ok(CallToolResult::success(vec![Content::json(json_result)?]))
    }
}

const_string!(Echo = "echo");
#[tool(tool_box)]
impl ServerHandler for GlueDataCatalog {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("This server provides a glue data catalog tool that can be used to get database and table metadata from an AWS Glue Data Catalog".to_string()),
        }
    }
}
