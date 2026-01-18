use bm25::Embedding;
use memchunk::chunk;

#[derive(Debug)]
pub struct Chunk {
    pub content: String,
    pub embedding: Option<Embedding>,
}

impl Chunk {
    fn from_content(content: String) -> Self {
        Self {
            content: content,
            embedding: None,
        }
    }
}

pub fn chunk_text(text: String, size: usize) -> Vec<Chunk> {
    let text_bytes = text.as_bytes();
    let chunks: Vec<&[u8]> = chunk(text_bytes).size(size).collect();
    let string_chunks: Vec<String> = chunks
        .iter()
        .map(|&chunk| String::from_utf8_lossy(chunk).to_string())
        .collect();
    let mut struct_chunks: Vec<Chunk> = vec![];
    for c in string_chunks {
        let chunk_struct = Chunk::from_content(c);
        struct_chunks.push(chunk_struct);
    }
    println!("Created {:?} chunks", struct_chunks.len());
    struct_chunks
}
