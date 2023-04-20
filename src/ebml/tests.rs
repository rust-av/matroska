use std::path::Path;

use super::*;

#[test]
fn mkv_header() {
    for (f, expected) in mkv_headers() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("matroska_test_w1_1")
            .join(f);

        let bytes = std::fs::read(path).expect("can read file");
        let (_, header) = ebml_header(&bytes).expect("can parse header");
        assert_eq!(header, expected);
    }
}

#[test]
fn webm_header() {
    let f = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("big-buck-bunny_trailer.webm");

    let webm = std::fs::read(f).expect("can read file");

    let expected = EbmlHeader {
        doc_type: "webm".into(),
        doc_type_version: 1,
        doc_type_read_version: 1,
        ..default_header()
    };

    let (_, header) = ebml_header(&webm[..100]).unwrap();
    assert_eq!(header, expected);
}

#[test]
fn floats() {
    #[rustfmt::skip]
    let tests: Vec<(&[u8], Option<f64>)> = vec![
        // wrong lengths
        (&[], None),
        (&[0xFF, 0xFF], None),
        (&[0xAB, 0xCD, 0xEF, 0x12, 0x56], None),
        (&[0xAB, 0xCD, 0xEF, 0x12, 0x56, 0x78, 0x90, 0xFF, 0xFF], None),

        // f32
        (&[0x42, 0x02, 0x2F, 0x07], Some(32.545_925_140_380_86)),
        (&[0x4A, 0xBE, 0x2F, 0x00], Some(6_231_936.0)),
        (&[0xA6, 0xAA, 0x3B, 0x0C], Some(-1.181_212_432_418_065_8e-15)),

        // f64
        (&[0x40, 0x84, 0x0F, 0x47, 0xAE, 0x14, 0x7A, 0xE1], Some(641.91)),
        (&[0x46, 0x69, 0x23, 0x6B, 0xD6, 0xA3, 0xBE, 0x04], Some(1.593_333_125_495_994_4e31)),
        (&[0xA6, 0x6C, 0x81, 0x8A, 0x63, 0x94, 0x55, 0x6F], Some(-1.347_560_723_113_483_4e-123)),

        // subnormals
        (&[0x00, 0x30, 0x08, 0x51], Some(4.411_087_180_014_126e-39)),
        (&[0x80, 0x07, 0x21, 0xC3, 0x51, 0x96, 0x4B, 0x50], Some(-9.918_108_989_973_327e-309)),

        // Infinities
        (&[0x7F, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], Some(f64::INFINITY)),
        (&[0xFF, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], Some(f64::NEG_INFINITY)),

        // f32 infinity should stay infinite, even as f64
        (&[0x7F, 0x80, 0x00, 0x00], Some(f64::INFINITY)),
        (&[0xFF, 0x80, 0x00, 0x00], Some(f64::NEG_INFINITY)),
    ];

    for (bytes, expected) in tests {
        assert_eq!(expected, f64::try_parse(bytes).ok());
    }

    // EBML doesn't specify how to encode qNaN and sNaN. So we can't
    // reliably test this, as platforms differ (e.g. x86 vs. MIPS).
    let nans: [(&[u8], bool); 3] = [
        (&[0x7F, 0x96, 0x93, 0x4D], false),
        (&[0xFF, 0xE4, 0xA6, 0x6D], true),
        (&[0xFF, 0xFF, 0x21, 0xC3, 0x51, 0x96, 0x4B, 0x50], true),
    ];

    for (bytes, neg) in nans {
        let res = f64::try_parse(bytes).unwrap();
        assert!(res.is_nan());
        assert_eq!(res.is_sign_negative(), neg);
    }
}

#[test]
fn variable_integer() {
    let val01 = [0b10000000];

    match vint(&val01) {
        Ok((_, v)) => assert!(0 == v),
        _ => panic!(),
    }
}

fn mkv_headers() -> Vec<(&'static str, EbmlHeader)> {
    vec![
        ("test1.mkv", default_header()), // basic
        ("test2.mkv", default_header()), // includes CRC-32
        (
            // some non-default values
            "test4.mkv",
            EbmlHeader {
                doc_type_version: 1,
                doc_type_read_version: 1,
                ..default_header()
            },
        ),
    ]
}

fn default_header() -> EbmlHeader {
    EbmlHeader {
        version: 1,
        read_version: 1,
        max_id_length: 4,
        max_size_length: 8,
        doc_type: "matroska".into(),
        doc_type_version: 2,
        doc_type_read_version: 2,
    }
}
