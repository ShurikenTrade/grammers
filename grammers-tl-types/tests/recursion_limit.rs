//! Regression: deeply nested recursive boxed types (e.g. `RichText`, which wraps itself via
//! `text:RichText` in most of its 16 variants) must not be deserialized without bound. Telegram
//! delivers `RichText` inside webpage / Instant-View previews; a pathologically deep value would
//! otherwise recurse one stack frame per level through the generated enum `deserialize` and
//! overflow the worker thread's stack (observed in production as a SIGSEGV crash-loop). The
//! deserializer must instead reject excessive nesting with a recoverable error.

use grammers_tl_types::{Deserializable, enums::RichText};

/// `depth` levels of `textBold#6724abc4 text:RichText` wrapping a terminal `textEmpty#dc3d824f`.
/// Every byte sequence here is a *valid* boxed `RichText`, so absent a depth limit it deserializes
/// successfully — which is precisely the unbounded recursion we must prevent.
fn nested_rich_text(depth: usize) -> Vec<u8> {
    const TEXT_BOLD: u32 = 0x6724abc4;
    const TEXT_EMPTY: u32 = 0xdc3d824f;

    let mut buf = TEXT_EMPTY.to_le_bytes().to_vec();
    for _ in 0..depth {
        let mut outer = TEXT_BOLD.to_le_bytes().to_vec();
        outer.extend_from_slice(&buf);
        buf = outer;
    }
    buf
}

#[test]
fn deeply_nested_rich_text_is_rejected_not_recursed() {
    // 300 levels is shallow enough that the *unguarded* deserializer recurses through it without
    // overflowing the test thread (so this fails cleanly as `Ok` today, rather than crashing the
    // runner), yet far deeper than any legitimate Telegram value and past the recursion limit.
    let bytes = nested_rich_text(300);

    let result = RichText::from_bytes(&bytes);

    assert!(
        result.is_err(),
        "deeply nested RichText must be rejected by the recursion limit, got: {result:?}"
    );
}

#[test]
fn shallow_rich_text_still_deserializes() {
    // A normally-nested value (well within the limit) must still round-trip.
    let bytes = nested_rich_text(8);
    assert!(RichText::from_bytes(&bytes).is_ok());
}
