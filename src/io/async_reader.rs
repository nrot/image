use std::path::Path;
use tokio::io::AsyncRead;

use crate::image::ImageFormat;

pub struct AsyncReader<R: AsyncRead> {
    /// The reader. Should be buffered.
    inner: R,
    /// The format, if one has been set or deduced.
    format: Option<ImageFormat>,
    /// Decoding limits
    limits: super::Limits,
}

impl AsyncReader<tokio::io::BufReader<tokio::fs::File>>{
    /// Open a file to async read, format will be guessed from path.
    ///
    /// This will not attempt any io operation on the opened file.
    ///
    /// If you want to inspect the content for a better guess on the format, which does not depend
    /// on file extensions, follow this call with a call to [`with_guessed_format`].
    ///
    /// [`with_guessed_format`]: #method.with_guessed_format
    pub async fn open_async<P>(path: P) -> tokio::io::Result<Self>
    where 
        P: AsRef<Path>,
    {
        Self::open_impl_async(path.as_ref()).await
    }
    async fn open_impl_async(path: &Path)->tokio::io::Result<Self>{
        Ok(AsyncReader{
            inner: tokio::io::BufReader::new(tokio::fs::File::open(path).await?),
            format: ImageFormat::from_path(path).ok(),
            limits: super::Limits::default(),
        })
    }
}