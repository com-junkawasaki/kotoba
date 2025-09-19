use kotoba_lsp::{Backend, LspServiceBuilder};
use tower_lsp::{LspService, Server};

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspServiceBuilder::new(|client| Backend::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
}
