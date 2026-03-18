# solana-mcp

A Solana MCP server that provides tools for querying the Solana blockchain.

## Tools

- `get_balance` - Get SOL balance for a public key
- `get_account_info` - Get account info for a public key
- `get_transaction` - Get transaction details by signature
- `get_minimum_balance_for_rent_exemption` - Get minimum balance for rent exemption

## Usage

```bash
cargo build --release
```

## Configuration

Add to your MCP config file:

```json
{
  "mcpServers": {
    "solana-mcp": {
      "command": "/path/to/solana-mcp/target/release/solana-mcp",
      "env": {
        "SOLANA_RPC_ENDPOINT": "https://api.mainnet-beta.solana.com"
      }
    }
  }
}
```

