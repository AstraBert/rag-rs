use std::path::PathBuf;

use anyhow;
use tokio::fs;

pub struct Parser {
    pub directory_path: String,
}

impl Parser {
    pub fn new(directory_path: String) -> Self {
        Self {
            directory_path: directory_path,
        }
    }

    async fn extract_text_from_pdf(&self, file_path: PathBuf) -> anyhow::Result<String> {
        let bytes = fs::read(file_path).await?;
        let out = pdf_extract::extract_text_from_mem(&bytes)?;
        Ok(out)
    }

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
