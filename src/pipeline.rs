use crate::{chunking::chunk_text, embedding::embed_chunks, parsing::Parser, vectordb::VectorDB};

pub struct Pipeline {
    // Parsing options
    pub directory_path: String,
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
    ) -> Self {
        return Self {
            directory_path: directory_path,
            chunk_size: chunk_size,
            qdrant_url: qdrant_url,
            collection_name: collection_name,
        };
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let parser = Parser::new(self.directory_path.clone());
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
