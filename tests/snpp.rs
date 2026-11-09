#![cfg(feature = "serde")]

use spacecrafts::{FrameType, Spacecraft, TimecodeConfig};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/snpp.json");

#[test]
fn parse_snpp_fixture() {
    let sc = Spacecraft::with_file(FIXTURE).expect("fixture should parse");

    assert_eq!(sc.scid, 157);
    assert_eq!(sc.name, "snpp");
    assert_eq!(sc.norad_catalog_id, 37849);
    assert_eq!(sc.aliases, vec!["npp".to_string()]);
    assert_eq!(sc.vcids.len(), 14);
    assert!(!sc.apids.is_empty());

    assert_eq!(sc.framing.length, 892);
    assert_eq!(sc.framing.frame_type, FrameType::CcsdsAos);
    assert!(sc.framing.pseudo_noise.is_some());
    let rs = sc.framing.reed_solomon.as_ref().expect("rs should be set");
    assert_eq!(rs.interleave, 4);
    assert_eq!(rs.virtual_fill_length, 0);
    assert_eq!(rs.num_correctable, 16);

    match sc.timecodes.get("cds").expect("cds timecode should be set") {
        TimecodeConfig::CDS {
            day_length,
            submillis_length,
            ..
        } => {
            assert_eq!(*day_length, 2);
            assert_eq!(*submillis_length, 2);
        }
        other => panic!("expected CDS timecode, got {other:?}"),
    }
}

#[test]
fn snpp_fixture_is_valid() {
    let sc = Spacecraft::with_file(FIXTURE).expect("fixture should parse");
    sc.validate().expect("fixture should validate");
}

#[test]
fn snpp_fixture_cadu_length() {
    let sc = Spacecraft::with_file(FIXTURE).expect("fixture should parse");
    // 4 byte ASM + 892 byte frame + RS parity (interleave 4 * 16 correctable * 2)
    assert_eq!(sc.cadu_length(), 4 + 892 + 4 * 32);
}
