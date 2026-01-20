# rag-rs

> [!NOTE]
>
> _This software is released in `alpha`, so you can and should expect bugs. Please take a look at the [Roadmap](#roadmap) to see the plan to reach the first stable version_

A Rust-native implementation of the RAG stack, using:

- [PdfExtract](https://github.com/jrmuizel/pdf-extract) for PDF extraction
- [memchunk](https://github.com/chonkie-inc/memchunk) for chunking
- [BM25](https://github.com/Michael-JB/bm25) for embedding
- [Qdrant](https://qdrant.tech) for storing
- [async-openai](https://github.com/64bit/async-openai) for LLM generation

Moreover, it can be served as an API server, usin:

- [Axum](https://github.com/tokio-rs/axum) as a web server framework
- [tower-governor](https://github.com/benwis/tower-governor) for rate limiting
- [tower-http](https://github.com/tower-rs/tower-http) for CORS
- [tracing and tracing_subscriber](https://github.com/tokio-rs/tracing) for logging

## Installation and Usage

Install with cargo:

```bash
cargo install rag-rs
```

### `load` command

Parse, chunk and embed the documents in a given directory, and upload them to a vector store.

**Usage**

```bash
rag-rs load [OPTIONS] --directory <DIRECTORY> --qdrant-url <QDRANT_URL> --collection-name <COLLECTION_NAME>
```

**Options**

- `-d, --directory <DIRECTORY>`  
  The path to the directory containing the files for the RAG pipeline. (required)
- `--qdrant-url <QDRANT_URL>`  
  URL for a Qdrant vector store instance. If your Qdrant instance needs an API key, make sure that it is available as `QDRANT_API_KEY` in your environment. (required)
- `--collection-name <COLLECTION_NAME>`  
  Name of the collection for the Qdrant vector store. (required)
- `--chunk-size <CHUNK_SIZE>`  
  Chunking size. **Default:** `1024`
- `--cache-dir <CACHE_DIR>`
  Directory where to cache the parsed file. **Default:** `.rag-rs-cache/`
- `--cache-chunk-size <CACHE_CHUNK_SIZE>`
  Chunk size for cached writes. **Default:** `1024 bytes`
- `--no-cache`
  Deactivate read/write from cache. **Default:** active
- `-h, --help`  
  Print help information.

**Example**

```bash
rag-rs --directory data/ \
    --chunk-size 2048 \
    --qdrant-url http://localhost:6334 \
    --collection-name test-data \
    --cache-dir cache/ \
    --cache-chunk-size 1048576
```

### `serve` command

Serve the RAG application as an API server.

**Usage**

```bash
rag-rs serve [OPTIONS] --qdrant-url <QDRANT_URL> --collection-name <COLLECTION_NAME>
```

**Options**

- `--qdrant-url <QDRANT_URL>`  
  URL of your Qdrant instance. If your Qdrant instance needs an API key, make sure that it is available as `QDRANT_API_KEY` in your environment. (required)
- `--collection-name <COLLECTION_NAME>`  
  Name of the collection for the Qdrant vector store. (required)
- `--openai-api-key <OPENAI_API_KEY>`  
  OpenAI API key. It is not advised to pass the key as an option to the CLI command: you should set it as the `OPENAI_API_KEY` environment variable.
- `-p, --port <PORT>`  
  Port for the server to run on. **Default:** `8000`
- `--host <HOST>`  
  Host for the server to run on. **Default:** `0.0.0.0`
- `--rate-limit-per-minute <RATE_LIMIT_PER_MINUTE>`  
  Request rate limit per minute. **Default:** `100`
- `--cors <CORS>`  
  Allowed CORS origin (e.g. `https://mydomain.com`). **Default:** `*` (all origins allowed). While this argument has no effect for local development, it is advisable to set it for production deployments.
- `--log-level <LOG_LEVEL>`  
  Logging level. **Default:** `info`  
  **Available values:** `info`, `debug`, `error`, `warning`, `trace`
- `--log-json`  
  Whether or not to activate JSON logging. **Default:** `false` (uses compact logging by default)
- `-h, --help`  
  Print help information.

**Example**

```bash
rag-rs --qdrant-url http://localhost:6334 \
    --collection-name test-data \
    --host 127.0.0.1 \
    --port 3000 \
    --rate-limit-per-minute 30 \
    --cors "http://mydomain.com" \
    --log-leve info \
    --log-json
```

## Limitations

- Currently supports only `.pdf`, `.txt` and `.md` files
- Does not go through the data directory recursively
- PDF extraction accounts only for text

## Roadmap

To reach the first stable version, this software will first:

- [X] Add a caching layer (v0.2.0-alpha)
- [ ] Introduce thorough testing
- [ ] Add a programmatic API along with the CLI app, possibly both in Rust and Python
- [ ] Add an NPM-installable version
- [ ] Add support for more text-based file formats, and possibly for more unstructured file formats
