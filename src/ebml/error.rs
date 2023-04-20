#[derive(PartialEq, Eq)]
pub struct Error {
    /// The Element ID where the error occurred. 0 if not available.
    ///
    /// For an overview of Element IDs, see the list of
    /// [EBML Element IDs] or [Matroska Element IDs].
    ///
    /// [EBML Element IDs]: https://www.rfc-editor.org/rfc/rfc8794.html#name-ebml-element-ids-registry
    /// [Matroska Element IDs]: https://www.ietf.org/archive/id/draft-ietf-cellar-matroska-15.html#section-27.1-11
    pub id: u32,

    /// See [ErrorKind] for more information.
    pub kind: ErrorKind,
}

/// Describes what went wrong.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// nom returned an error.
    Nom(nom::error::ErrorKind),

    /// The Element Data Size did not fit within a [usize].
    /// The current parsing code cannot handle an element of this size.
    ElementTooLarge,

    /// A required Element was not found by the parser.
    MissingElement,

    /// One of the segment element types was discovered more than once in the input.
    DuplicateSegment,

    /// The VINT_WIDTH is 8 or more, which means that the resulting variable-size
    /// integer is more than 8 octets wide. This is currently not supported.
    VintTooWide,

    /// The VINT_WIDTH of this Element ID is 4 or more, which is not allowed as
    /// per the Matroska specification (Element IDs can be 1 to 4 octets long,
    /// except for the EBML Header which is also limited to 4 octets here).
    IDTooWide,

    /// A signed integer element has declared a length of more than 8 octets,
    /// which is not allowed.
    IntTooWide,

    /// An unsigned integer with a maximum length of 4 octets has declared a
    /// length of more than 4 octets, which is not allowed.
    U32TooWide,

    /// An unsigned integer element has declared a length of more than 8 octets,
    /// which is not allowed.
    UintTooWide,

    /// A float element has declared a length of 0 octets, which needs to be
    /// converted to some default value (0.0, if not otherwise specified).
    EmptyFloat,

    /// A float element has declared a length that is not 0, 4 or 8 octets,
    /// which is not allowed.
    FloatWidthIncorrect,

    /// A string element contains non-UTF-8 data, which is not allowed.
    StringNotUtf8,

    /// A binary element does not adhere to the length declared in the
    /// specification. The enclosed [u16] is the actual length of the data.
    BinaryWidthIncorrect(u16),

    /// A CRC-32 element was found, but the checksum did not match.
    Crc32Mismatch,
}

/// Create an error with the given ID and [ErrorKind].
pub fn ebml_err<'a, T>(id: u32, kind: ErrorKind) -> nom::IResult<&'a [u8], T, Error> {
    Err(nom::Err::Error(Error { id, kind }))
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Error")
            .field("id", &format!("{:#0X}", self.id))
            .field("kind", &self.kind)
            .finish()
    }
}

impl<'a> nom::error::ParseError<&'a [u8]> for Error {
    fn from_error_kind(_input: &'a [u8], kind: nom::error::ErrorKind) -> Self {
        Self {
            id: 0,
            kind: ErrorKind::Nom(kind),
        }
    }

    fn append(_input: &'a [u8], _kind: nom::error::ErrorKind, other: Self) -> Self {
        other
    }

    fn or(self, other: Self) -> Self {
        match other.kind {
            // "Complete" overrides some EBML errors, so discard it
            ErrorKind::Nom(nom::error::ErrorKind::Complete) => self,
            _ => other,
        }
    }
}

impl<I> nom::error::FromExternalError<I, Error> for Error {
    fn from_external_error(_input: I, _kind: nom::error::ErrorKind, e: Error) -> Self {
        e
    }
}
