# Files, uploads and media

## Uploads

Every file parameter takes an `impl Into<timvw_rust_runtime::Upload>`:

- a `PathBuf` or `&Path`, streamed from disk (named after its last component, its media type
  guessed from the extension; native only);
- bytes (`Vec<u8>`, `Bytes`, `&'static [u8]`): name them with `Upload::file_name`, since the API
  uses the extension;
- a reader or a stream, with its length.

A `&str` is not taken as a path: write `Path::new("audio.wav")`. A missing file is an
`Error::Http` whose `is_io()` is `true`, raised before anything is sent; `Error::io_error()` gives
the `std::io::Error` (with its `kind()`). An upload is sent again on a retry when it can be (a
path or bytes can, a one-shot stream cannot).

```rust,no_run
use openai_ergonomic::builders::files::FilePurpose;
use openai_ergonomic::Client;
use std::path::{Path, PathBuf};
use timvw_rust_runtime::Upload;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?.build();

    // A file from disk, streamed (no temporary copy)
    let file = client
        .files()
        .upload_file(PathBuf::from("data/batch.jsonl"), FilePurpose::Batch)
        .await?;
    println!("uploaded {}", file.id);

    // Bytes from memory
    let notes = Upload::bytes(b"hello".to_vec()).file_name("notes.txt").mime("text/plain");
    client.files().upload_file(notes, FilePurpose::Assistants).await?;

    // Speech to text, and an image edit
    let transcription = client
        .audio()
        .transcription(Path::new("meeting.wav"), "gpt-4o-mini-transcribe")
        .language("en");
    let text = client.audio().create_transcription(transcription).await?;
    println!("{text:?}");

    let edit = client
        .images()
        .edit(PathBuf::from("photo.png"), "Add a rainbow")
        .model("gpt-image-1");
    let edited = client.images().create_edit(edit).await?;
    client.images().save_all(&edited, ".", "rainbow").await?;
    Ok(())
}
```

## Large files

`Client::upload_large_file` sends a file of up to 8 GB through the Uploads API: it creates an
upload, sends parts of at most 64 MB several at a time (4 by default), each read straight from
its offset in the file and retried on its own, and completes the upload. The file name defaults
to the upload's (a path's last component) and the media type to its own (guessed from a path's
extension); set them with `.filename(..)` and `.mime_type(..)`.

```rust,no_run
use openai_ergonomic::{Client, UploadPurpose};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?.build();
    let upload = client
        .upload_large_file(PathBuf::from("training.jsonl"), UploadPurpose::FineTune)
        .mime_type("text/jsonl")
        .concurrency(8)
        .await?;
    println!("uploaded: {:?}", upload.file);
    Ok(())
}
```

## Generated media and downloads

```rust,no_run
use openai_ergonomic::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?.build();

    let images = client
        .images()
        .create(client.images().generate("A lighthouse at dusk").model("gpt-image-1"))
        .await?;
    let paths = client.images().save_all(&images, ".", "lighthouse").await?;
    println!("saved {paths:?}");

    let speech = client.audio().speech("gpt-4o-mini-tts", "Hello there", "alloy");
    client.audio().speech_to_file(speech, "hello.mp3").await?;

    client.files().download_to("file-abc123", "results.jsonl").await?;
    Ok(())
}
```

`ImagesClient::save` decodes `b64_json` or downloads the image's `url` (without the provider's
credentials); `AudioClient::speech_to_file` and `FilesClient::download_to` stream the body to a
temporary file that replaces the target once complete. These are native only: WebAssembly has
no file system (see [WebAssembly](webassembly.md)).

See also [vector store operations](vector_store_operations.md) and the examples
[`files.rs`](../examples/files.rs), [`uploads.rs`](../examples/uploads.rs),
[`images_comprehensive.rs`](../examples/images_comprehensive.rs),
[`audio_speech.rs`](../examples/audio_speech.rs) and
[`audio_transcription.rs`](../examples/audio_transcription.rs).
