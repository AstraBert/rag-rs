mod chunking;
mod embedding;
mod parsing;
mod pipeline;
mod serving;
mod vectordb;

use clap::{Parser, Subcommand};

use crate::{pipeline::Pipeline, serving::RagServer};

#[derive(Parser)]
struct CliArgs {
    #[command(subcommand)]
    cmd: Commands,
}
#[derive(Subcommand, Debug)]
enum Commands {
    /// Parse, chunk and embed the documents in a given directory, and upload them to a
    /// vector store.
    /// Uses PdfExtract for parsing, memchunk for chunking, BM25 for embeddings and Qdrant as a vector database.
    Load {
        // Parser options
        /// The path to the directory containing the files for the RAG pipeline
        #[arg(short, long)]
        directory: String,

        // Chunking options
        /// Chunking size
        #[arg(long, default_value_t = 1024)]
        chunk_size: usize,

        // VectorDB options
        /// URL for a Qdrant vector store instance.
        /// If your Qdrant instance needs an API key, make sure that
        /// it is available as `QDRANT_API_KEY` in your environment
        #[arg(long)]
        qdrant_url: String,

        /// Name of the collection for the Qdrant vector store.
        #[arg(long)]
        collection_name: String,
    },
    /// Serve the RAG application as an API server.
    Serve {
        // URL for a Qdrant vector store instance.
        /// If your Qdrant instance needs an API key, make sure that
        /// it is available as `QDRANT_API_KEY` in your environment
        #[arg(long)]
        qdrant_url: String,

        /// Name of the collection for the Qdrant vector store.
        #[arg(long)]
        collection_name: String,

        /// OpenAI API key.
        /// It is not advised to pass the key as an option
        /// to the CLI command: you should set it
        /// as the `OPENAI_API_KEY` environment variable.
        #[arg(long, default_value = None)]
        openai_api_key: Option<String>,

        /// Port for the server to run on. Defaults to 8000.
        #[arg(short, long, default_value = None)]
        port: Option<u16>,

        /// Host for the server to run on. Defaults to '0.0.0.0'.
        #[arg(long, default_value = None)]
        host: Option<String>,

        /// Request rate limit per minute. Defaults to 100.
        #[arg(long, default_value = None)]
        rate_limit_per_minute: Option<u32>,

        /// Allowed CORS origin (e.g. 'https://mydomain.com'). Defaults to '*' (all origins allowed) if not provided.
        /// While this argument has no effect for local development, it is advisable to set it for production deployments.
        #[arg(long, default_value = None)]
        cors: Option<String>,

        // logging
        /// Logging level. Defaults to 'info'. Available values: 'info', 'debug', 'error', 'warning', 'trace'
        #[arg(long, default_value = None)]
        log_level: Option<String>,

        /// Wether or not to activate JSON logging. Defaults to false (uses compact logging by default).
        #[arg(long, default_value_t = false)]
        log_json: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = CliArgs::parse();
    match args.cmd {
        Commands::Load {
            directory,
            chunk_size,
            qdrant_url,
            collection_name,
        } => {
            let pipeline = Pipeline::new(directory, chunk_size, qdrant_url, collection_name);
            pipeline.run().await?;
        }
        Commands::Serve {
            qdrant_url,
            collection_name,
            openai_api_key,
            port,
            host,
            rate_limit_per_minute,
            cors,
            log_level,
            log_json,
        } => {
            let server = RagServer::new(
                qdrant_url,
                openai_api_key,
                collection_name,
                port,
                host,
                rate_limit_per_minute,
                cors,
                log_level,
                log_json,
            );
            server.serve().await?;
        }
    }
    Ok(())
}
