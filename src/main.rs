use anyhow::Result;
use std::env;
use std::future::Future;
use std::str::FromStr;
use std::sync::Arc;

use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::*,
    schemars, tool, tool_handler, tool_router,
};
use solana_pubkey::Pubkey;
use solana_rpc_client::rpc_client::RpcClient;
use solana_signature::Signature;
use solana_transaction_status::UiTransactionEncoding;

const DEFAULT_RPC_ENDPOINT: &str = "https://api.mainnet-beta.solana.com";
const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

#[tokio::main]
async fn main() -> Result<()> {
    let rpc_endpoint =
        env::var("SOLANA_RPC_ENDPOINT").unwrap_or_else(|_| DEFAULT_RPC_ENDPOINT.to_string());

    let transport = (tokio::io::stdin(), tokio::io::stdout());
    let service = SolanaMcp::new(rpc_endpoint).serve(transport).await?;
    service.waiting().await?;
    Ok(())
}

// ============================================================================
// Tool Request Schemas
// ============================================================================

#[derive(schemars::JsonSchema, serde::Deserialize)]
pub struct GetAccountInfoRequest {
    #[schemars(description = "Solana public key (32 byte base58 encoded address)")]
    public_key: String,
}

#[derive(schemars::JsonSchema, serde::Deserialize)]
pub struct GetBalanceRequest {
    #[schemars(description = "Solana public key (32 byte base58 encoded address)")]
    public_key: String,
}

#[derive(schemars::JsonSchema, serde::Deserialize)]
pub struct GetMinimumBalanceForRentExemptionRequest {
    #[schemars(description = "Data size in bytes")]
    data_size: u64,
}

#[derive(schemars::JsonSchema, serde::Deserialize)]
pub struct GetTransactionRequest {
    #[schemars(description = "Transaction signature (64 byte base58 encoded string)")]
    signature: String,
}

// ============================================================================
// Main Server Implementation
// ============================================================================

pub struct SolanaMcp {
    tool_router: ToolRouter<SolanaMcp>,
    rpc_client: Arc<RpcClient>,
}

#[tool_router]
impl SolanaMcp {
    fn new(rpc_endpoint: String) -> Self {
        Self {
            tool_router: Self::tool_router(),
            rpc_client: Arc::new(RpcClient::new(rpc_endpoint)),
        }
    }

    // ========================================================================
    // Tools
    // ========================================================================

    #[tool(
        description = "Used to look up account info by public key (32 byte base58 encoded address)"
    )]
    async fn get_account_info(
        &self,
        Parameters(GetAccountInfoRequest { public_key }): Parameters<GetAccountInfoRequest>,
    ) -> String {
        let pubkey = match Pubkey::from_str(&public_key) {
            Ok(pk) => pk,
            Err(e) => return format!("Error: {}", e),
        };

        match self.rpc_client.get_account(&pubkey) {
            Ok(account) => serde_json::to_string_pretty(&account)
                .unwrap_or_else(|e| format!("Error serializing account: {}", e)),
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Used to look up balance by public key (32 byte base58 encoded address)")]
    async fn get_balance(
        &self,
        Parameters(GetBalanceRequest { public_key }): Parameters<GetBalanceRequest>,
    ) -> String {
        let pubkey = match Pubkey::from_str(&public_key) {
            Ok(pk) => pk,
            Err(e) => return format!("Error: {}", e),
        };

        match self.rpc_client.get_balance(&pubkey) {
            Ok(lamports) => {
                let sol_balance = lamports as f64 / LAMPORTS_PER_SOL as f64;
                format!("{} SOL ({} lamports)", sol_balance, lamports)
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(
        description = "Used to look up minimum balance required for rent exemption by data size"
    )]
    async fn get_minimum_balance_for_rent_exemption(
        &self,
        Parameters(GetMinimumBalanceForRentExemptionRequest { data_size }): Parameters<
            GetMinimumBalanceForRentExemptionRequest,
        >,
    ) -> String {
        match self
            .rpc_client
            .get_minimum_balance_for_rent_exemption(data_size as usize)
        {
            Ok(lamports) => {
                let sol_balance = lamports as f64 / LAMPORTS_PER_SOL as f64;
                format!("{} SOL ({} lamports)", sol_balance, lamports)
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(
        description = "Used to look up transaction by signature (64 byte base58 encoded string)"
    )]
    async fn get_transaction(
        &self,
        Parameters(GetTransactionRequest { signature }): Parameters<GetTransactionRequest>,
    ) -> String {
        let sig = match Signature::from_str(&signature) {
            Ok(s) => s,
            Err(e) => return format!("Error: Invalid signature - {}", e),
        };

        match self
            .rpc_client
            .get_transaction(&sig, UiTransactionEncoding::JsonParsed)
        {
            Ok(tx) => serde_json::to_string_pretty(&tx)
                .unwrap_or_else(|e| format!("Error serializing transaction: {}", e)),
            Err(e) => format!("Error: {}", e),
        }
    }
}

// ============================================================================
// Server Handler with manual prompt implementations
// ============================================================================

#[tool_handler]
impl ServerHandler for SolanaMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_prompts()
                .build(),
            server_info: Implementation {
                name: "Solana RPC Tools".to_string(),
                version: "1.0.0".to_string(),
            },
            instructions: Some(
                "Solana MCP server providing RPC tools and helpful prompts for Solana development"
                    .to_string(),
            ),
        }
    }

    // ========================================================================
    // Prompts
    // ========================================================================

    fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> impl Future<Output = Result<ListPromptsResult, ErrorData>> + Send + '_ {
        async move {
            Ok(ListPromptsResult {
                prompts: vec![
                    Prompt {
                        name: "calculate-storage-deposit".to_string(),
                        description: Some(
                            "Calculate storage deposit for a specified number of bytes".to_string(),
                        ),
                        arguments: Some(vec![PromptArgument {
                            name: "bytes".to_string(),
                            description: Some("Number of bytes to store".to_string()),
                            required: Some(true),
                        }]),
                    },
                    Prompt {
                        name: "minimum-amount-of-sol-for-storage".to_string(),
                        description: Some(
                            "Calculate the minimum amount of SOL needed for storing 0 bytes on-chain"
                                .to_string(),
                        ),
                        arguments: None,
                    },
                    Prompt {
                        name: "why-did-my-transaction-fail".to_string(),
                        description: Some(
                            "Look up the given transaction and inspect its logs to figure out why it failed"
                                .to_string(),
                        ),
                        arguments: Some(vec![PromptArgument {
                            name: "signature".to_string(),
                            description: Some("Transaction signature".to_string()),
                            required: Some(true),
                        }]),
                    },
                    Prompt {
                        name: "how-much-did-this-transaction-cost".to_string(),
                        description: Some(
                            "Fetch the transaction by signature, and break down cost & priority fees"
                                .to_string(),
                        ),
                        arguments: Some(vec![PromptArgument {
                            name: "signature".to_string(),
                            description: Some("Transaction signature".to_string()),
                            required: Some(true),
                        }]),
                    },
                    Prompt {
                        name: "what-happened-in-transaction".to_string(),
                        description: Some(
                            "Look up the given transaction and inspect its logs & instructions to figure out what happened"
                                .to_string(),
                        ),
                        arguments: Some(vec![PromptArgument {
                            name: "signature".to_string(),
                            description: Some("Transaction signature".to_string()),
                            required: Some(true),
                        }]),
                    },
                ],
                next_cursor: None,
            })
        }
    }

    fn get_prompt(
        &self,
        request: GetPromptRequestParam,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> impl Future<Output = Result<GetPromptResult, ErrorData>> + Send + '_ {
        async move {
            let args = request.arguments.unwrap_or_default();

            match request.name.as_str() {
                "calculate-storage-deposit" => {
                    let bytes = args.get("bytes").and_then(|v| v.as_str()).unwrap_or("0");
                    Ok(GetPromptResult {
                        description: Some(
                            "Calculate storage deposit for a specified number of bytes".to_string(),
                        ),
                        messages: vec![PromptMessage::new_text(
                            PromptMessageRole::User,
                            format!(
                                "Calculate the SOL amount needed to store {} bytes of data on Solana using getMinimumBalanceForRentExemption.",
                                bytes
                            ),
                        )],
                    })
                }
                "minimum-amount-of-sol-for-storage" => Ok(GetPromptResult {
                    description: Some(
                        "Calculate the minimum amount of SOL needed for storing 0 bytes on-chain"
                            .to_string(),
                    ),
                    messages: vec![PromptMessage::new_text(
                        PromptMessageRole::User,
                        "Calculate the amount of SOL needed to store 0 bytes of data on Solana using getMinimumBalanceForRentExemption & present it to the user as the minimum cost for storing any data on Solana.",
                    )],
                }),
                "why-did-my-transaction-fail" => {
                    let signature = args.get("signature").and_then(|v| v.as_str()).unwrap_or("");
                    Ok(GetPromptResult {
                        description: Some(
                            "Look up the given transaction and inspect its logs to figure out why it failed"
                                .to_string(),
                        ),
                        messages: vec![PromptMessage::new_text(
                            PromptMessageRole::User,
                            format!(
                                "Look up the transaction with signature {} and inspect its logs to figure out why it failed.",
                                signature
                            ),
                        )],
                    })
                }
                "how-much-did-this-transaction-cost" => {
                    let signature = args.get("signature").and_then(|v| v.as_str()).unwrap_or("");
                    Ok(GetPromptResult {
                        description: Some(
                            "Fetch the transaction by signature, and break down cost & priority fees"
                                .to_string(),
                        ),
                        messages: vec![PromptMessage::new_text(
                            PromptMessageRole::User,
                            format!(
                                "Calculate the network fee for the transaction with signature {} by fetching it and inspecting the 'fee' field in 'meta'. Base fee is 0.000005 sol per signature (also provided as array at the end). So priority fee is fee - (numSignatures * 0.000005). Please provide the base fee and the priority fee.",
                                signature
                            ),
                        )],
                    })
                }
                "what-happened-in-transaction" => {
                    let signature = args.get("signature").and_then(|v| v.as_str()).unwrap_or("");
                    Ok(GetPromptResult {
                        description: Some(
                            "Look up the given transaction and inspect its logs & instructions to figure out what happened"
                                .to_string(),
                        ),
                        messages: vec![PromptMessage::new_text(
                            PromptMessageRole::User,
                            format!(
                                "Look up the transaction with signature {} and inspect its logs & instructions to figure out what happened.",
                                signature
                            ),
                        )],
                    })
                }
                _ => Err(ErrorData::invalid_params(
                    format!("Unknown prompt: {}", request.name),
                    None,
                )),
            }
        }
    }
}
