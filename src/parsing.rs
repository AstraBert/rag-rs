use reqwest;
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

const LLAMA_CLOUD_BASE_URL: &str = "https://api.cloud.llamaindex.ai";
const LLAMA_CLOUD_EU_BASE_URL: &str = "https://api.cloud.eu.llamaindex.ai";
const DEFAULT_TIMEOUT: u64 = 180;
const DEFAULT_PAGE_SIZE: i32 = 100;
const DEFAULT_CONTINUE_AS_NEW_THRESHOLD: i32 = 10;
const DEFAULT_MAX_POLLING_ATTEMPTS: u64 = 180;
const DEFAULT_POLLING_INTERVAL: u64 = 10;

#[derive(Serialize, Deserialize, Debug)]
struct CreateDirectoryResponse {
    created_at: Option<String>,
    data_source_id: Option<String>,
    deleted_at: Option<String>,
    description: Option<String>,
    id: String,
    name: String,
    project_id: String,
    updated_at: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct CreateBatchJobResponse {
    created_at: Option<String>,
    id: String,
    dorectory_id: Option<String>,
    project_id: String,
    started_at: Option<String>,
    updated_at: Option<String>,
    completed_at: Option<String>,
    effective_at: String,
    error_message: Option<String>,
    failed_items: i32,
    job_record_id: Option<String>,
    job_type: String,
    processed_items: i32,
    skipped_items: i32,
    status: String,
    total_items: i32,
    workflow_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetBatchJobResponse {
    job: CreateBatchJobResponse,
    progress_percentage: i32,
}

pub struct Parser {
    api_key: String,
    base_url: String,
    directory_id: Option<String>,
    batch_job_id: Option<String>,
    pub directory_path: String,
    pub direcory_description: Option<String>,
    pub max_polling_attempts: u64,
    pub polling_interval: u64,
}

impl Parser {
    fn new(
        directory_path: String,
        directory_description: Option<String>,
        eu: bool,
        api_key: Option<String>,
        max_polling_attempts: Option<u64>,
        polling_interval: Option<u64>,
    ) -> Self {
        let llama_cloud_api_key = match api_key {
            Some(s) => s,
            None => std::env::var("LLAMA_CLOUD_API_KEY")
                .expect("If API key is not provided, should be able to load it from enviroment"),
        };
        let llama_cloud_url = if eu {
            LLAMA_CLOUD_EU_BASE_URL
        } else {
            LLAMA_CLOUD_BASE_URL
        };
        let pollings = match max_polling_attempts {
            Some(p) => p,
            None => DEFAULT_MAX_POLLING_ATTEMPTS,
        };
        let polling_int = match polling_interval {
            Some(p) => p,
            None => DEFAULT_POLLING_INTERVAL,
        };
        Self {
            directory_path: directory_path,
            direcory_description: directory_description,
            api_key: llama_cloud_api_key,
            base_url: llama_cloud_url.to_string(),
            batch_job_id: None,
            directory_id: None,
            max_polling_attempts: pollings,
            polling_interval: polling_int,
        }
    }

    async fn create_directory(&mut self) -> anyhow::Result<()> {
        println!(
            "Creating a directory on LlamaCloud from {}",
            self.directory_path
        );
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/api/v1/beta/directories", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "name": &self.directory_path,
                "description": &self.direcory_description,
            }))
            .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT))
            .send()
            .await?;
        if response.status().is_success() {
            let response_json = response.json::<CreateDirectoryResponse>().await?;
            self.directory_id = Some(response_json.id);
            println!("Successfully created directory");
            Ok(())
        } else {
            let detail = response.text().await?;
            eprintln!("An error occurred while creating the directory: {}", detail);
            return Err(anyhow::anyhow!(
                "Could not create a directory on LlamaCloud"
            ));
        }
    }

    async fn upload_files_to_directory(&self) -> anyhow::Result<()> {
        let dir_id = match &self.directory_id {
            Some(s) => s,
            None => {
                eprintln!(
                    "A directory ID is needed for the file upload to take place. Run `create_directory` first"
                );
                return Err(anyhow::anyhow!(
                    "A directory ID is needed for the file upload to take place. Run `create_directory` first"
                ));
            }
        };
        println!("Starting to upload files from {}", self.directory_path);
        let mut entries = tokio::fs::read_dir(&self.directory_path).await?;
        let client = reqwest::Client::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            println!("Starting to upload: {:?}", path);
            let mut file = File::open(&path).await?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).await?;
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown.pdf");

            let part = multipart::Part::bytes(buffer)
                .file_name(filename.to_string())
                .mime_str("application/pdf")?;
            let form = multipart::Form::new().part("upload_file", part);

            let response = client
                .post(format!(
                    "{}/api/v1/beta/directories/{}/files/upload",
                    self.base_url, &dir_id
                ))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .multipart(form)
                .timeout(std::time::Duration::from_secs(60))
                .send()
                .await?;

            if !response.status().is_success() {
                eprintln!("Failed to upload {}: {}", filename, response.status());
            } else {
                println!("Uploaded: {}", filename);
            }
        }
        Ok(())
    }

    async fn create_batch_job(&mut self) -> anyhow::Result<()> {
        let dir_id = match &self.directory_id {
            Some(s) => s,
            None => {
                eprintln!(
                    "A directory ID is needed for the file upload to take place. Run `create_directory` first"
                );
                return Err(anyhow::anyhow!(
                    "A directory ID is needed for the file upload to take place. Run `create_directory` first"
                ));
            }
        };
        println!("Starting to create batch job for {}", self.directory_path);
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/api/v1/beta/batch-processing", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "directory_id": dir_id,
                "job_config": {
                    "job_name": "parse_raw_file_job",
                    "partitions": {},
                    "parameters": {
                        "type": "parse",
                        "lang": "en",
                        "fast_mode": true,
                    },
                },
                "page_size": DEFAULT_PAGE_SIZE,
                "continue_as_new_threshold": DEFAULT_CONTINUE_AS_NEW_THRESHOLD,
            }))
            .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT))
            .send()
            .await?;
        if !response.status().is_success() {
            let detail = response.text().await?;
            eprintln!(
                "Could not successfully create batch job because of: {}, retry soon!",
                detail
            );
            return Err(anyhow::anyhow!(
                "Could not successfully create batch job, retry soon!"
            ));
        } else {
            let response_json = response.json::<CreateBatchJobResponse>().await?;
            self.batch_job_id = Some(response_json.id);
            println!("Successfully created batch job");
        }
        Ok(())
    }

    async fn poll_job_for_completion(&self) -> anyhow::Result<bool> {
        let job_id = match &self.batch_job_id {
            Some(s) => s,
            None => {
                eprintln!(
                    "A Job ID is needed for the polling to take place. Run `create_batch_job` first"
                );
                return Err(anyhow::anyhow!(
                    "A directory ID is needed for the polling to take place. Run `create_batch_job` first"
                ));
            }
        };
        let mut i = 0;
        let client = reqwest::Client::new();
        while i < self.max_polling_attempts {
            let response = client
                .get(format!(
                    "{}/api/v1/beta/batch-processing/{}",
                    self.base_url, job_id
                ))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT))
                .send()
                .await?;
            if !response.status().is_success() {
                let detail = response.text().await?;
                eprintln!(
                    "An error occurred while polling for the job: {}. Retrying...",
                    detail,
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(self.polling_interval)).await;
            } else {
                let response_json = response.json::<GetBatchJobResponse>().await?;
                if response_json.job.status == "completed"
                    || response_json.job.status == "failed"
                    || response_json.job.status == "cancelled"
                {
                    println!("Job completed with status: {}", response_json.job.status);
                    return Ok(true);
                } else {
                    if i < (self.max_polling_attempts - 1) {
                        tokio::time::sleep(tokio::time::Duration::from_secs(self.polling_interval))
                            .await;
                    }
                }
            }
            i += 1;
        }
        eprintln!("Maximum retries exceeded, job never completed...");
        Ok(false)
    }
}
