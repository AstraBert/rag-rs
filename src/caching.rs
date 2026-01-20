use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

const DEFAULT_CACHE_DIR: &str = "./.rag-rs-cache";
const DEFAULT_CHUNK_SIZE: usize = 1024;

pub struct Cache {
    pub directory: String,
    pub chunk_size: usize,
}

impl Cache {
    pub fn new(directory: Option<String>, chunk_size: Option<usize>) -> Self {
        let cache_dir = match directory {
            Some(s) => s,
            None => DEFAULT_CACHE_DIR.to_string(),
        };
        let cache_chunk_size = match chunk_size {
            Some(c) => c,
            None => DEFAULT_CHUNK_SIZE,
        };
        Self {
            directory: cache_dir,
            chunk_size: cache_chunk_size,
        }
    }

    pub async fn write_file_content(
        &self,
        file_path: &str,
        file_content: String,
    ) -> cacache::Result<()> {
        let to_cache = file_content.into_bytes();
        let mut fd = cacache::Writer::create(&self.directory, file_path).await?;
        for chunk in to_cache.chunks(self.chunk_size) {
            fd.write_all(chunk)
                .await
                .expect("Should be able to write to file");
        }
        fd.commit().await?;
        Ok(())
    }

    pub async fn read_file_content(&self, file_path: &str) -> cacache::Result<String> {
        let mut fd = cacache::Reader::open(&self.directory, file_path).await?;
        let mut buf = String::new();
        fd.read_to_string(&mut buf)
            .await
            .expect("Should be able to read from file");
        fd.check()?;
        Ok(buf)
    }
}
