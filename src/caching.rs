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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_correct_cache_init() {
        let cache = Cache::new(None, None);
        assert_eq!(cache.chunk_size, DEFAULT_CHUNK_SIZE);
        assert_eq!(cache.directory, DEFAULT_CACHE_DIR);
        let cache_1 = Cache::new(Some("data/cache".to_string()), Some(1024_usize));
        assert_eq!(cache_1.directory, "data/cache".to_string());
        assert_eq!(cache_1.chunk_size, 1024_usize);
    }

    #[tokio::test]
    async fn test_write_and_read_file() {
        let cache = Cache::new(None, None);
        let file_path = "test.txt";
        let file_content = "this is a test".to_string();
        let res = cache.write_file_content(file_path, file_content).await;
        assert!(res.is_ok());
        let content = cache.read_file_content(file_path).await;
        match content {
            Ok(buf) => {
                assert_eq!(buf, "this is a test".to_string());
            }
            Err(e) => {
                println!(
                    "An error occurred while testing cache reading: {}",
                    e.to_string()
                );
                assert!(false);
            }
        }
    }
}
