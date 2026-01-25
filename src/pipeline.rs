use crate::{chunking::chunk_text, embedding::embed_chunks, parsing::Parser, vectordb::VectorDB};

pub struct Pipeline {
    // Parsing options
    pub directory_path: String,
    pub cached: bool,
    pub cache_directory: Option<String>,
    pub cache_chunk_size: Option<usize>,
    // Chunking options
    pub chunk_size: usize,
    // VectorDB options
    qdrant_url: String,
    pub collection_name: String,
}

impl Pipeline {
    pub fn new(
        directory_path: String,
        chunk_size: usize,
        qdrant_url: String,
        collection_name: String,
        cached: bool,
        cache_directory: Option<String>,
        cache_chunk_size: Option<usize>,
    ) -> Self {
        Self {
            directory_path,
            chunk_size,
            qdrant_url,
            collection_name,
            cache_directory,
            cache_chunk_size,
            cached,
        }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let parser = Parser::new(
            self.directory_path.clone(),
            self.cached,
            self.cache_directory.clone(),
            self.cache_chunk_size,
        );
        let vectordb = VectorDB::new(self.qdrant_url.clone(), self.collection_name.clone());
        let results = parser.parse().await?;
        vectordb.create_collection().await?;
        for result in results {
            let mut chunks = chunk_text(result, self.chunk_size);
            chunks = embed_chunks(chunks);
            vectordb.upload_embeddings(chunks).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::pipeline::Pipeline;

    #[tokio::test]
    async fn test_pipeline_run() {
        let qdrant_url_var = std::env::var("QDRANT_URL");
        let qdrant_url = match qdrant_url_var {
            Ok(s) => s.to_string(),
            Err(_) => {
                println!("Skipping test because Qdrant is not available");
                return;
            }
        };
        let pipeline = Pipeline::new(
            "testfiles/".to_string(),
            1024_usize,
            qdrant_url,
            "test-collection".to_string(),
            true,
            None,
            None,
        );
        let result = pipeline.run().await;
        assert!(result.is_ok());
    }
}
