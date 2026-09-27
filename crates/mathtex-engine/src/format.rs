use std::fmt;

use mathtex_font::FontLoader;
use mathtex_portable_engine_generated as pe;

use crate::adapter::{FontTable, NativeFontTable, ProviderFiles};
use crate::resource::{ResourceError, ResourceKind, ResourceProvider};

/// Magic of the packaged format container, followed by a version and a compression byte.
const MAGIC: &[u8; 6] = b"MTXPKG";
const VERSION: u8 = 2;
const STORED: u8 = 0;
const DEFLATE: u8 = 1;
/// DEFLATE level for packaging, only the builder pays for it, not the reader.
const DEFLATE_LEVEL: u8 = 7;

/// Defines `\hostbox` when the preamble left it undefined, with its own catcodes so any regime works.
const HOST_BOX_DEFINITION: &str = r"\ifdefined\hostbox\else\begingroup\catcode123=1 \catcode125=2 \catcode35=6 \gdef\hostbox#1{\Uhostbox#1\relax}\endgroup\fi";

/// A validated packaged XeTeX format and its native fonts, the same bytes load on every little endian target.
#[derive(Clone)]
pub struct Format {
    pub(crate) image: pe::PortableFormatImage,
    pub(crate) fonts: NativeFontTable,
}

impl Format {
    /// Loads a format [`FormatBuilder::build`] wrote, such as one included with `include_bytes!`.
    pub fn from_static(bytes: &'static [u8]) -> Result<Format, FormatError> {
        Self::decode(bytes)
    }

    /// Loads a format [`FormatBuilder::build`] wrote.
    pub fn from_owned(bytes: Vec<u8>) -> Result<Format, FormatError> {
        Self::decode(&bytes)
    }

    fn decode(bytes: &[u8]) -> Result<Format, FormatError> {
        let (image, fonts) = unpack(bytes)?;
        let image =
            pe::PortableFormatImage::from_bytes(&image).map_err(|error| FormatError::Image {
                message: error.to_string(),
            })?;
        Ok(Self { image, fonts })
    }
}

impl fmt::Debug for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Format")
            .field("native_fonts", &self.fonts.len())
            .finish_non_exhaustive()
    }
}

/// Why bytes are not a usable format.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FormatError {
    /// The bytes do not start with the packaged format header.
    NotAFormat,
    /// The container was written by a version of this crate with another layout.
    Version {
        /// Version byte found.
        found: u8,
    },
    /// The payload does not inflate or its parts do not add up.
    Corrupt,
    /// The format image was dumped by another engine build or on a big endian target, or fails validation.
    Image {
        /// Why the image was refused.
        message: String,
    },
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAFormat => f.write_str("not a packaged mathtex format"),
            Self::Version { found } => {
                write!(
                    f,
                    "packaged format version {found}, this build reads {VERSION}"
                )
            }
            Self::Corrupt => f.write_str("packaged format is corrupt"),
            Self::Image { message } => write!(f, "{message}, bake the format again"),
        }
    }
}

impl std::error::Error for FormatError {}

/// Bakes a packaged format: INITEX runs an optional kernel file, then the preamble, then dumps.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct FormatBuilder {
    preamble: String,
    kernel: Option<String>,
}

impl FormatBuilder {
    /// A format whose INITEX run reads `preamble`, which should not `\dump` itself.
    pub fn new(preamble: impl Into<String>) -> Self {
        Self {
            preamble: preamble.into(),
            kernel: None,
        }
    }

    /// Runs `file`, such as `latex.ltx`, through INITEX to its own `\dump` before the preamble.
    pub fn kernel(mut self, file: impl Into<String>) -> Self {
        self.kernel = Some(file.into());
        self
    }

    /// Runs the kernel and preamble, defines `\hostbox` if still undefined, and returns the packaged format.
    pub fn build(
        self,
        resources: &impl ResourceProvider,
        fonts: &impl FontLoader,
    ) -> Result<Vec<u8>, BuildError> {
        let mut table = FontTable::default();
        let empty = pe::PortableFormatImage::empty();
        let mut engine = pe::PortableTexEngine::from_format(&empty, ProviderFiles(resources))
            .with_font_platform(table.platform(fonts));
        if !engine.initialize_format_state() {
            return Err(BuildError::Initialization);
        }
        if let Some(kernel) = &self.kernel {
            let bytes = resources
                .read(kernel, ResourceKind::TexInput)
                .map_err(|error| BuildError::Kernel {
                    name: kernel.clone(),
                    error,
                })?
                .bytes;
            run_input(&mut engine, kernel, bytes)?;
        }
        // The newline keeps a trailing comment in the preamble from swallowing the definitions after it.
        let preamble = format!("{}\n{HOST_BOX_DEFINITION}\\dump", self.preamble);
        run_input(&mut engine, "preamble", preamble.into_bytes())?;
        if !engine.finalize_trie() {
            return Err(BuildError::Trie);
        }
        let image = engine.into_format().to_bytes();
        Ok(pack(&image, &table.snapshot()))
    }
}

/// Runs one INITEX input to its `\dump` or `\end`.
fn run_input(
    engine: &mut pe::PortableTexEngine<'_>,
    input: &str,
    bytes: Vec<u8>,
) -> Result<(), BuildError> {
    if engine.begin_primary_input(input, bytes) && engine.run_format_initialization() {
        return Ok(());
    }
    Err(match engine.last_error() {
        Some(error) => BuildError::Tex {
            input: input.into(),
            message: error.message.clone(),
            line: error.line,
        },
        None => BuildError::Aborted {
            input: input.into(),
            status: engine.last_abort_status(),
        },
    })
}

/// Why a format could not be baked.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BuildError {
    /// INITEX could not initialize its tables.
    Initialization,
    /// The kernel file could not be read.
    Kernel {
        /// The kernel's name.
        name: String,
        /// Why it could not be read.
        error: ResourceError,
    },
    /// TeX reported an error, which ends a format build.
    Tex {
        /// The kernel's name or `preamble`.
        input: String,
        /// TeX's error message.
        message: String,
        /// Line of the innermost file TeX was reading.
        line: i32,
    },
    /// The run stopped without an error message.
    Aborted {
        /// The kernel's name or `preamble`.
        input: String,
        /// The engine's exit status, when it set one.
        status: Option<i32>,
    },
    /// The hyphenation patterns overflow the trie.
    Trie,
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Initialization => f.write_str("INITEX initialization failed"),
            Self::Kernel { name, error } => write!(f, "cannot read the kernel {name}: {error}"),
            Self::Tex {
                input,
                message,
                line,
            } => write!(f, "{input}, line {line}: {message}"),
            Self::Aborted { input, status } => match status {
                Some(status) => write!(f, "{input}: TeX stopped with status {status}"),
                None => write!(f, "{input}: TeX stopped"),
            },
            Self::Trie => f.write_str("the hyphenation patterns overflow the trie"),
        }
    }
}

impl std::error::Error for BuildError {}

/// The container: magic, version, compression, payload length, then the payload, deflated.
fn pack(image: &[u8], fonts: &[(pe::PortableFontHandle, String, i32)]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(image.len() + 64);
    payload.extend_from_slice(&(image.len() as u64).to_le_bytes());
    payload.extend_from_slice(image);
    payload.extend_from_slice(&(fonts.len() as u32).to_le_bytes());
    for (handle, spec, size) in fonts {
        payload.extend_from_slice(&(*handle as u64).to_le_bytes());
        payload.extend_from_slice(&size.to_le_bytes());
        payload.extend_from_slice(&(spec.len() as u32).to_le_bytes());
        payload.extend_from_slice(spec.as_bytes());
    }
    let compressed = miniz_oxide::deflate::compress_to_vec(&payload, DEFLATE_LEVEL);
    let mut out = Vec::with_capacity(compressed.len() + 16);
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    out.push(DEFLATE);
    out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    out.extend_from_slice(&compressed);
    out
}

/// Cursor over container bytes whose reads fail instead of running past the end.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, len: usize) -> Result<&'a [u8], FormatError> {
        let end = self.at.checked_add(len).ok_or(FormatError::Corrupt)?;
        let slice = self.bytes.get(self.at..end).ok_or(FormatError::Corrupt)?;
        self.at = end;
        Ok(slice)
    }

    fn u32(&mut self) -> Result<u32, FormatError> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn u64(&mut self) -> Result<u64, FormatError> {
        let mut word = [0; 8];
        word.copy_from_slice(self.take(8)?);
        Ok(u64::from_le_bytes(word))
    }

    fn len(&mut self) -> Result<usize, FormatError> {
        usize::try_from(self.u64()?).map_err(|_| FormatError::Corrupt)
    }
}

/// Splits a container into its format image bytes and native font table.
fn unpack(bytes: &[u8]) -> Result<(Vec<u8>, NativeFontTable), FormatError> {
    if bytes.get(..MAGIC.len()) != Some(&MAGIC[..]) {
        return Err(FormatError::NotAFormat);
    }
    let mut header = Reader {
        bytes,
        at: MAGIC.len(),
    };
    let version = header.take(1)?[0];
    if version != VERSION {
        return Err(FormatError::Version { found: version });
    }
    let compression = header.take(1)?[0];
    let payload_len = header.len()?;
    let body = &bytes[header.at..];
    let payload = match compression {
        DEFLATE => miniz_oxide::inflate::decompress_to_vec_with_limit(body, payload_len)
            .map_err(|_| FormatError::Corrupt)?,
        STORED => body.to_vec(),
        _ => return Err(FormatError::Corrupt),
    };
    if payload.len() != payload_len {
        return Err(FormatError::Corrupt);
    }
    let mut payload = Reader {
        bytes: &payload,
        at: 0,
    };
    let image_len = payload.len()?;
    let image = payload.take(image_len)?.to_vec();
    let count = payload.u32()? as usize;
    let mut fonts = Vec::with_capacity(count.min(1024));
    for _ in 0..count {
        let handle = usize::try_from(payload.u64()?).map_err(|_| FormatError::Corrupt)?;
        let size = payload.u32()? as i32;
        let spec_len = payload.u32()? as usize;
        let spec = std::str::from_utf8(payload.take(spec_len)?)
            .map_err(|_| FormatError::Corrupt)?
            .to_string();
        fonts.push((handle, spec, size));
    }
    if payload.at != payload.bytes.len() {
        return Err(FormatError::Corrupt);
    }
    Ok((image, fonts))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::InMemoryResourceProvider;
    use mathtex_font::InMemoryFontLoader;

    fn plain_format() -> Vec<u8> {
        FormatBuilder::new(r"\catcode`\{=1 \catcode`\}=2 \catcode`\$=3 \def\hello{hi}")
            .build(&InMemoryResourceProvider::new(), &InMemoryFontLoader::new())
            .expect("a catcode preamble bakes")
    }

    #[test]
    fn containers_round_trip_their_image_and_font_table() {
        let fonts = vec![(3, "[latinmodern-math.otf]:script=math".to_string(), 655_360)];
        let packed = pack(b"image", &fonts);
        assert_eq!(unpack(&packed), Ok((b"image".to_vec(), fonts)));
    }

    #[test]
    fn foreign_and_damaged_bytes_are_refused() {
        let packed = plain_format();
        assert!(Format::from_owned(packed.clone()).is_ok());
        assert_eq!(
            Format::from_owned(vec![1, 2, 3]).err(),
            Some(FormatError::NotAFormat)
        );
        let mut other_version = packed.clone();
        other_version[MAGIC.len()] = VERSION + 1;
        assert_eq!(
            Format::from_owned(other_version).err(),
            Some(FormatError::Version { found: VERSION + 1 })
        );
        let truncated = packed[..packed.len() / 2].to_vec();
        assert_eq!(
            Format::from_owned(truncated).err(),
            Some(FormatError::Corrupt)
        );
        let stale = pack(b"not an image", &[]);
        assert!(matches!(
            Format::from_owned(stale),
            Err(FormatError::Image { .. })
        ));
    }

    #[test]
    fn a_preamble_error_names_its_input_and_line() {
        let error = FormatBuilder::new("\\relax\n\\undefinedmacro")
            .build(&InMemoryResourceProvider::new(), &InMemoryFontLoader::new())
            .expect_err("an undefined control sequence fails the build");
        assert!(
            matches!(&error, BuildError::Tex { input, line: 2, .. } if input == "preamble"),
            "{error:?}"
        );
        let error = FormatBuilder::new("")
            .kernel("missing.ltx")
            .build(&InMemoryResourceProvider::new(), &InMemoryFontLoader::new())
            .expect_err("a missing kernel fails the build");
        assert!(matches!(error, BuildError::Kernel { .. }), "{error:?}");
    }
}
