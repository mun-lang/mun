use lsp_types::{ClientCapabilities, ServerCapabilities};

/// Returns the capabilities of this LSP server implementation given the capabilities of the client.
pub fn server_capabilities(_client_caps: &ClientCapabilities) -> ServerCapabilities {
    ServerCapabilities {
        ..Default::default()
    }
}
