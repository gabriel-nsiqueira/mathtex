//! All three engine modes (tex / etex / xetex) must boot through the optimized engine.
use mathtex_engine::portable_engine::{self as pe, EmptyResourceProvider, EngineProfile};
use mathtex_engine::GeneratedFormatCache;

fn boot(profile: EngineProfile) {
    let cache = GeneratedFormatCache::initialized(profile);
    let mut engine = cache.instantiate(profile, EmptyResourceProvider::default());
    engine.finalize_trie();
    let img = engine.into_format();
    assert!(img.state_array_bytes() > 0);
}

#[test]
fn tex_mode_boots() {
    boot(EngineProfile::tex());
}
#[test]
fn etex_mode_boots() {
    boot(EngineProfile::etex());
}
#[test]
fn xetex_mode_boots() {
    boot(EngineProfile::xetex());
}
#[test]
fn memory_word_is_8_bytes() {
    assert_eq!(pe::memory_word_bytes(), 8);
}
