use rmcp::{
    Error as McpError, RoleServer, ServerHandler, const_string, model::*, schemars,
    service::RequestContext, tool,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use aws_config::BehaviorVersion;
use aws_config;
use aws_sdk_glue;

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

#[derive(Clone)]
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
        client.get_databases().send().await.expect("Couldn't connect to AWS");
        Self { client }
    }

    #[tool(description = "List the databases in an AWS Glue Data Catalog")]
    async fn list_databases(&self) -> Result<CallToolResult, McpError> {
        log::info!("Listing databases in {}", self.client.config().region().unwrap());
        let response = self.client.get_databases()
            .send()
            .await
            .map_err(|e| {
                McpError::internal_error(
                    "Failed to list databases",
                    Some(json!({"error": e.to_string()})),
                )
            })?;
        
        let databases = response.database_list()
            .iter()
            .map(|db| db.name().into())
            .collect::<Vec<String>>();
        
        let result = ListDatabasesResult { databases };
        let json_result = serde_json::to_value(result).map_err(|e| {
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
        let response = self.client.get_tables()
            .database_name(database_name.clone())
            .send()
            .await
            .map_err(|e| {
                McpError::internal_error(
                    "Failed to get tables",
                    Some(json!({"error": e.to_string()})),
                )
            })?;
        
        let tables = response.table_list()
            .iter()
            .map(|table| table.name().into())
            .collect::<Vec<String>>();
        
        let result = DatabaseMetadata {
            name: database_name,
            tables,
        };
        
        let json_result = serde_json::to_value(result).map_err(|e| {
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
        let response = self.client.get_table()
            .database_name(database_name)
            .name(table_name.clone())
            .send()
            .await
            .map_err(|e| {
                McpError::internal_error(
                    "Failed to get table metadata",
                    Some(json!({"error": e.to_string()})),
                )
            })?;
        
        let columns = response.table()
            .and_then(|table| table.storage_descriptor())
            .and_then(|sd| Some(sd.columns()))
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
            McpError::internal_error(
                "Failed to serialize result",
                Some(json!({"error": e.to_string()})),
            )
        })?;
        
        Ok(CallToolResult::success(vec![Content::json(json_result)?]))
    }

    // #[tool(description = "Calculate the sum of two numbers")]
    // fn sum(
    //     &self,
    //     #[tool(aggr)] StructRequest { a, b }: StructRequest,
    // ) -> Result<CallToolResult, McpError> {
    //     Ok(CallToolResult::success(vec![Content::text(
    //         (a + b).to_string(),
    //     )]))
    // }
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

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult {
            resources: vec![
                // self._create_resource_text("str:////Users/to/some/path/", "cwd"),
                // self._create_resource_text("memo://insights", "memo-name"),
            ],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        ReadResourceRequestParam { uri }: ReadResourceRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        match uri.as_str() {
            "str:////Users/to/some/path/" => {
                let cwd = "/Users/to/some/path/";
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(cwd, uri)],
                })
            }
            "memo://insights" => {
                let memo = "Business Intelligence Memo\n\nAnalysis has revealed 5 key insights ...";
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(memo, uri)],
                })
            }
            _ => Err(McpError::resource_not_found(
                "resource_not_found",
                Some(json!({
                    "uri": uri
                })),
            )),
        }
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        Ok(ListPromptsResult {
            next_cursor: None,
            prompts: vec![Prompt::new(
                "example_prompt",
                Some("This is an example prompt that takes one required argument, message"),
                Some(vec![PromptArgument {
                    name: "message".to_string(),
                    description: Some("A message to put in the prompt".to_string()),
                    required: Some(true),
                }]),
            )],
        })
    }

    async fn get_prompt(
        &self,
        GetPromptRequestParam { name, arguments }: GetPromptRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        match name.as_str() {
            "example_prompt" => {
                let message = arguments
                    .and_then(|json| json.get("message")?.as_str().map(|s| s.to_string()))
                    .ok_or_else(|| {
                        McpError::invalid_params("No message provided to example_prompt", None)
                    })?;

                let prompt =
                    format!("This is an example prompt with your message here: '{message}'");
                Ok(GetPromptResult {
                    description: None,
                    messages: vec![PromptMessage {
                        role: PromptMessageRole::User,
                        content: PromptMessageContent::text(prompt),
                    }],
                })
            }
            _ => Err(McpError::invalid_params("prompt not found", None)),
        }
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(ListResourceTemplatesResult {
            next_cursor: None,
            resource_templates: Vec::new(),
        })
    }
}
