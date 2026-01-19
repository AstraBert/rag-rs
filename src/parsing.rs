use std::path::PathBuf;

use anyhow;
use tokio::fs;

use crate::caching::Cache;

pub struct Parser {
    pub directory_path: String,
    pub cached: bool,
    pub cache_directory: Option<String>,
    pub cache_chunk_size: Option<usize>,
}

impl Parser {
    pub fn new(
        directory_path: String,
        cached: bool,
        cache_directory: Option<String>,
        cache_chunk_size: Option<usize>,
    ) -> Self {
        Self {
            directory_path: directory_path,
            cache_directory: cache_directory,
            cache_chunk_size: cache_chunk_size,
            cached: cached,
        }
    }

    async fn extract_text_from_pdf(&self, file_path: PathBuf) -> anyhow::Result<String> {
        if self.cached {
            let cache = Cache::new(self.cache_directory.clone(), self.cache_chunk_size);
            match cache
                .read_file_content(
                    file_path
                        .to_str()
                        .expect("Should be able to convert path to string"),
                )
                .await
            {
                Ok(s) => {
                    return Ok(s);
                }
                Err(_) => {}
            };
        }
        let bytes = fs::read(file_path.clone()).await?;
        let out = pdf_extract::extract_text_from_mem(&bytes)?;
        if self.cached {
            let cache = Cache::new(self.cache_directory.clone(), self.cache_chunk_size);
            cache
                .write_file_content(
                    file_path
                        .to_str()
                        .expect("Should be able to convert path to string"),
                    out.clone(),
                )
                .await?;
        }
        Ok(out)
    }

    // This is as expensive as reading/writing from cache, no need for caching here
    async fn read_file(&self, file_path: PathBuf) -> anyhow::Result<String> {
        let content = fs::read_to_string(file_path).await?;
        Ok(content)
    }

    pub async fn parse(&self) -> anyhow::Result<Vec<String>> {
        let mut entries = fs::read_dir(&self.directory_path).await?;
        let mut results: Vec<String> = vec![];
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let result = if path
                .extension()
                .expect("Should be able to get file extension")
                == "pdf"
            {
                println!("Extracting text from {:?}", path);
                self.extract_text_from_pdf(path).await?
            } else {
                if path
                    .extension()
                    .expect("Should be able to get file extension")
                    == "md"
                    || path
                        .extension()
                        .expect("Should be able to get file extension")
                        == "txt"
                {
                    println!("Reading text from {:?}", path);
                    self.read_file(path).await?
                } else {
                    eprintln!(
                        "Unsupported file format: {:?}. Supported file formats are: .pdf, .txt and .md",
                        path
                    );
                    continue;
                }
            };
            println!("Text size: {:?} chars", result.len());
            results.push(result);
        }

        Ok(results)
    }
}
