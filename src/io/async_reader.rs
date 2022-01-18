use std::path::Path;
use tokio::io::AsyncReadExt;
use tokio::io;

use crate::image::ImageFormat;
use super::free_functions;
use crate::{ImageError, ImageResult};
use crate::dynimage::DynamicImage;
use crate::error::{ImageFormatHint, UnsupportedError, UnsupportedErrorKind};

pub struct AsyncReader<R: AsyncReadExt> {
    /// The reader. Should be buffered.
    inner: R,
    /// The format, if one has been set or deduced.
    format: Option<ImageFormat>,
    /// Decoding limits
    limits: super::Limits,
}

impl<R: AsyncReadExt> AsyncReader<R> {
    /// Create a new image reader without a preset format.
    ///
    /// Assumes the reader is already buffered. For optimal performance,
    /// consider wrapping the reader with a `BufReader::new()`.
    ///
    /// It is possible to guess the format based on the content of the read object with
    /// [`with_guessed_format`], or to set the format directly with [`set_format`].
    ///
    /// [`with_guessed_format`]: #method.with_guessed_format
    /// [`set_format`]: method.set_format
    pub fn new(buffered_reader: R) -> Self {
        AsyncReader {
            inner: buffered_reader,
            format: None,
            limits: super::Limits::default(),
        }
    }

    /// Construct a reader with specified format.
    ///
    /// Assumes the reader is already buffered. For optimal performance,
    /// consider wrapping the reader with a `BufReader::new()`.
    pub fn with_format(buffered_reader: R, format: ImageFormat) -> Self {
        AsyncReader {
            inner: buffered_reader,
            format: Some(format),
            limits: super::Limits::default(),
        }
    }

    /// Get the currently determined format.
    pub fn format(&self) -> Option<ImageFormat> {
        self.format
    }

    /// Supply the format as which to interpret the read image.
    pub fn set_format(&mut self, format: ImageFormat) {
        self.format = Some(format);
    }

    /// Remove the current information on the image format.
    ///
    /// Note that many operations require format information to be present and will return e.g. an
    /// `ImageError::Unsupported` when the image format has not been set.
    pub fn clear_format(&mut self) {
        self.format = None;
    }

    /// Disable all decoding limits.
    pub fn no_limits(&mut self) {
        self.limits = super::Limits::no_limits();
    }

    /// Set a custom set of decoding limits.
    pub fn limits(&mut self, limits: super::Limits) {
        self.limits = limits;
    }

    /// Unwrap the reader.
    pub fn into_inner(self) -> R {
        self.inner
    }
}


impl AsyncReader<io::BufReader<tokio::fs::File>>{
    /// Open a file to async read, format will be guessed from path.
    ///
    /// This will not attempt any io operation on the opened file.
    ///
    /// If you want to inspect the content for a better guess on the format, which does not depend
    /// on file extensions, follow this call with a call to [`with_guessed_format`].
    ///
    /// [`with_guessed_format`]: #method.with_guessed_format
    pub async fn open<P>(path: P) -> io::Result<Self>
    where 
        P: AsRef<Path>,
    {
        Self::open_impl(path.as_ref()).await
    }
    async fn open_impl(path: &Path)->io::Result<Self>{
        Ok(AsyncReader{
            inner: io::BufReader::new(tokio::fs::File::open(path).await?),
            format: ImageFormat::from_path(path).ok(),
            limits: super::Limits::default(),
        })
    }
}


impl<R> AsyncReader<R> 
    where R:tokio::io::AsyncBufReadExt + tokio::io::AsyncBufRead + tokio::io::AsyncRead + tokio::io::AsyncSeekExt + std::marker::Unpin
    {
    /// Make a format guess based on the content, replacing it on success.
    ///
    /// Returns `Ok` with the guess if no io error occurs. Additionally, replaces the current
    /// format if the guess was successful. If the guess was unable to determine a format then
    /// the current format of the reader is unchanged.
    ///
    /// Returns an error if the underlying reader fails. The format is unchanged. The error is a
    /// `std::io::Error` and not `ImageError` since the only error case is an error when the
    /// underlying reader seeks.
    ///
    /// When an error occurs, the reader may not have been properly reset and it is potentially
    /// hazardous to continue with more io.
    ///
    /// ## Usage
    ///
    /// This supplements the path based type deduction from [`open`](Reader::open) with content based deduction.
    /// This is more common in Linux and UNIX operating systems and also helpful if the path can
    /// not be directly controlled.
    ///
    /// ```no_run
    /// # use image::ImageError;
    /// # use image::io::Reader;
    /// # fn main() -> Result<(), ImageError> {
    /// let image = Reader::open("image.unknown")?
    ///     .with_guessed_format()?
    ///     .decode()?;
    /// # Ok(()) }
    /// ```
    pub async fn with_guessed_format(mut self) -> io::Result<Self> {
        let format = self.guess_format().await?;
        // Replace format if found, keep current state if not.
        self.format = format.or(self.format);
        Ok(self)
    }

    async fn guess_format(&mut self) -> io::Result<Option<ImageFormat>> {
        // Save current offset, read start, restore offset.
        let cur = self.inner.seek(std::io::SeekFrom::Current(0)).await?;
        let mut start = [0u8; 16];
        let len = self.inner.read_exact(&mut start).await? as u64;

        self.inner.seek(io::SeekFrom::Start(cur)).await?;

        Ok(free_functions::guess_format_impl(&start[..len as usize]))
    }

    /// Read the image dimensions.
    ///
    /// Uses the current format to construct the correct reader for the format.
    ///
    /// If no format was determined, returns an `ImageError::Unsupported`.
    pub async fn into_dimensions(mut self) -> ImageResult<(u32, u32)> {
        let format = self.require_format()?;
        free_functions::image_dimensions_with_format_impl_async(self.inner, format).await
    }

    /// Read the image (replaces `load`).
    ///
    /// Uses the current format to construct the correct reader for the format.
    ///
    /// If no format was determined, returns an `ImageError::Unsupported`.
    pub fn decode(mut self) -> ImageResult<DynamicImage> {
        let format = self.require_format()?;
        free_functions::load_inner(self.inner, self.limits, format)
    }

    fn require_format(&mut self) -> ImageResult<ImageFormat> {
        self.format.ok_or_else(|| {
            ImageError::Unsupported(UnsupportedError::from_format_and_kind(
                ImageFormatHint::Unknown,
                UnsupportedErrorKind::Format(ImageFormatHint::Unknown),
            ))
        })
    }
}