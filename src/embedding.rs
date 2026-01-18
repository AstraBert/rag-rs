use bm25::{Embedder, EmbedderBuilder, Embedding, LanguageMode};

use crate::chunking::Chunk;

const DEFAULT_AVGDL: f32 = 5.75;

pub fn embed_chunks(mut chunks: Vec<Chunk>) -> Vec<Chunk> {
    println!("Starting to embed {:?} chunks", chunks.len());
    let embedder: Embedder = EmbedderBuilder::with_avgdl(DEFAULT_AVGDL)
        .language_mode(LanguageMode::Detect)
        .build();
    let mut i = 0;
    while i < chunks.len() {
        let embedding = embedder.embed(&chunks[i].content);
        chunks[i].embedding = Some(embedding);
        i += 1;
        if i % 10 == 0 {
            println!("Progress: {:?}/{:?}", i, chunks.len())
        }
    }
    chunks
}

pub fn embed_text(text: String) -> Embedding {
    let embedder: Embedder = EmbedderBuilder::with_avgdl(DEFAULT_AVGDL)
        .language_mode(LanguageMode::Detect)
        .build();
    let embd = embedder.embed(&text);
    embd
}
