mod logger;
mod server;

use std::error::Error;

use server::LanguageServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    logger::init();
    let mut server = LanguageServer::new(tokio::io::stdin(), tokio::io::stdout());
    server.run().await?;
    Ok(())
}
