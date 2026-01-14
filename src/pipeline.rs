use crate::{chunking::chunk_text, embedding::embed_chunks, parsing::Parser, vectordb::VectorDB};

struct Pipeline {
    // Parsing options
    pub directory_path: String,
    pub directory_description: Option<String>,
    pub use_eu: bool,
    llama_cloud_api_key: Option<String>,
    pub max_polling_attempts: Option<u64>,
    pub polling_interval: Option<u64>,
    // Chunking options
    pub chunk_size: usize,
    // VectorDB options
    qdrant_url: String,
    pub collection_name: String,
}

impl Pipeline {
    fn new(
        directory_path: String,
        directory_description: Option<String>,
        use_eu: bool,
        llama_cloud_api_key: Option<String>,
        max_polling_attempts: Option<u64>,
        polling_interval: Option<u64>,
        chunk_size: usize,
        qdrant_url: String,
        collection_name: String,
    ) -> Self {
        return Self {
            directory_path: directory_path,
            directory_description: directory_description,
            use_eu: use_eu,
            chunk_size: chunk_size,
            llama_cloud_api_key: llama_cloud_api_key,
            max_polling_attempts: max_polling_attempts,
            polling_interval: polling_interval,
            qdrant_url: qdrant_url,
            collection_name: collection_name,
        };
    }

    async fn run(&self) -> anyhow::Result<()> {
        let mut parser = Parser::new(
            self.directory_path.clone(),
            self.directory_description.clone(),
            self.use_eu,
            self.llama_cloud_api_key.clone(),
            self.max_polling_attempts,
            self.polling_interval,
        );
        let vectordb = VectorDB::new(self.qdrant_url.clone(), self.collection_name.clone());
        parser.create_directory().await?;
        parser.upload_files_to_directory().await?;
        parser.create_batch_job().await?;
        let job_ok = parser.poll_job_for_completion().await?;
        if !job_ok {
            return Err(anyhow::anyhow!("Parsing job was not successfull"));
        }
        let results = parser.get_parsed_results().await?;
        vectordb.create_collection().await?;
        for result in results {
            let mut chunks = chunk_text(result, self.chunk_size);
            chunks = embed_chunks(chunks);
            vectordb.upload_embeddings(chunks).await?;
        }
        Ok(())
    }
}
