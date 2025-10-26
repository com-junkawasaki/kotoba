use kotoba_lsp::{Backend, LspServiceBuilder};

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    // Create a dummy socket parameter - this will be replaced by the actual socket in build_server
    let service = LspServiceBuilder::new(|client| Backend::new(client));
    LspServiceBuilder::build_server(stdin, stdout, service).await.unwrap();
}
