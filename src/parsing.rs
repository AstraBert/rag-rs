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

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_extract_from_pdf() {
        let parser = Parser::new("testfiles/".to_string(), true, None, None);
        let now = tokio::time::Instant::now();
        let result = parser
            .extract_text_from_pdf(PathBuf::from("testfiles/sample.pdf"))
            .await;
        let first_elapsed = now.elapsed();
        match result {
            Ok(s) => {
                // should contain some text from the file
                println!("{}", s);
                assert!(s.contains("Sample PDF"));
            }
            Err(e) => {
                println!("An error occurred during the extraction: {}", e.to_string());
                assert!(false);
            }
        }
        let now1 = tokio::time::Instant::now();
        let result1 = parser
            .extract_text_from_pdf(PathBuf::from("testfiles/sample.pdf"))
            .await;
        let second_elapsed = now1.elapsed();
        assert!(result1.is_ok());
        // cache access should make the extraction from the PDF file faster the second time
        assert!(second_elapsed < first_elapsed);
    }

    #[tokio::test]
    async fn test_read_file() {
        let parser = Parser::new("testfiles/".to_string(), true, None, None);
        let result = parser.read_file(PathBuf::from("testfiles/test.txt")).await;
        match result {
            Ok(s) => {
                assert!(s.contains("This is a test!"));
            }
            Err(e) => {
                println!(
                    "An error occurred while reading the file: {}",
                    e.to_string()
                );
                assert!(false);
            }
        }
    }

    #[tokio::test]
    async fn test_parse() {
        let parser = Parser::new("testfiles/".to_string(), true, None, None);
        let results = parser.parse().await;
        match results {
            Ok(v) => {
                assert_eq!(v.len(), 2);
            }
            Err(e) => {
                println!(
                    "An error occurred while parsing testfiles/: {}",
                    e.to_string()
                );
                assert!(false);
            }
        }
    }
}
