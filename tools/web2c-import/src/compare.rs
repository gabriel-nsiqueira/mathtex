//! Body comparison and classification for each function.

use std::collections::BTreeMap;

use crate::select::{normalized_source_for_hash, source_hash, unsupported_external_boundaries};

/// Classification of the difference between patched base and XeTeX bodies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BodyDiff {
    /// Bodies are identical after whitespace normalization.
    IdenticalAfterNormalization,
    /// Bodies differ only in numeric/constant literal tokens.
    OnlyConstantsDiffer,
    /// A small, locally guarded behavioral delta (few tokens changed).
    SmallGuardedBehavior,
    /// XeTeX body references unadapted host boundary identifiers.
    HostBoundaryDifference,
    /// Difference concentrated in state layout and table address access.
    StateLayoutDifference,
    /// Large semantic divergence between the two bodies.
    LargeSemanticDivergence,
    /// Difference that could not be confidently classified.
    Unknown,
}

impl BodyDiff {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::IdenticalAfterNormalization => "IdenticalAfterNormalization",
            Self::OnlyConstantsDiffer => "OnlyConstantsDiffer",
            Self::SmallGuardedBehavior => "SmallGuardedBehavior",
            Self::HostBoundaryDifference => "HostBoundaryDifference",
            Self::StateLayoutDifference => "StateLayoutDifference",
            Self::LargeSemanticDivergence => "LargeSemanticDivergence",
            Self::Unknown => "Unknown",
        }
    }

    pub(crate) fn is_unknown(self) -> bool {
        matches!(self, Self::Unknown)
    }
}

/// Body selection strategy for a comparison record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SelectedStrategy {
    /// Use the base body when bodies are identical or the TeX when available policy applies.
    UseBase,
    /// Use the XeTeX body for host boundary and widened differences.
    UseXetex,
    SingleProfile,
}

#[derive(Clone, Debug)]
pub(crate) struct ComparisonRecord {
    pub(crate) symbol: String,
    /// Profiles present for this symbol, in `tex`/`etex`/`xetex` order.
    pub(crate) profiles_present: Vec<&'static str>,
    /// Hash of each profile body after normalization.
    pub(crate) profile_body_hashes: BTreeMap<&'static str, u64>,
    /// Classified difference between base and xetex bodies (when both present).
    pub(crate) diff: BodyDiff,
    pub(crate) selected_strategy: SelectedStrategy,
    /// Host boundary identifiers required by the selected body, if any.
    pub(crate) required_boundaries: Vec<&'static str>,
    pub(crate) reason_if_not_importable: Option<String>,
}

/// Classify the difference between an already patched base body and XeTeX body.
pub(crate) fn classify_body_diff(base_patched: &str, xetex_patched: &str) -> BodyDiff {
    let base_norm = normalized_source_for_hash(base_patched);
    let xetex_norm = normalized_source_for_hash(xetex_patched);
    if base_norm == xetex_norm {
        return BodyDiff::IdenticalAfterNormalization;
    }

    // XeTeX body references a host boundary that the base body does not.
    let xetex_boundaries = unsupported_external_boundaries(xetex_patched);
    let base_boundaries = unsupported_external_boundaries(base_patched);
    if !xetex_boundaries.is_empty() && xetex_boundaries != base_boundaries {
        return BodyDiff::HostBoundaryDifference;
    }

    let base_tokens = body_tokens(&base_norm);
    let xetex_tokens = body_tokens(&xetex_norm);
    let delta = TokenDelta::between(&base_tokens, &xetex_tokens);

    // Shared meaningful vocabulary determines whether bodies are variants of the same function.
    let base_vocab = meaningful_vocabulary(&base_tokens);
    let xetex_vocab = meaningful_vocabulary(&xetex_tokens);
    let smaller_vocab = base_vocab.len().min(xetex_vocab.len());
    let shared_vocab = base_vocab.intersection(&xetex_vocab).count();
    let shared_ratio = if smaller_vocab == 0 {
        0.0
    } else {
        shared_vocab as f64 / smaller_vocab as f64
    };
    if shared_vocab < MIN_SHARED_ANCHORS && shared_ratio < MIN_VARIANT_SHARED_RATIO {
        return BodyDiff::Unknown;
    }

    let larger = base_tokens.len().max(xetex_tokens.len());
    let changed_ratio = if larger == 0 {
        0.0
    } else {
        delta.changed_tokens as f64 / larger as f64
    };

    if delta.changed_tokens > 0 && delta.changed_non_numeric == 0 {
        return BodyDiff::OnlyConstantsDiffer;
    }

    if delta.changed_non_numeric > 0
        && delta.state_layout_tokens > 0
        && delta.changed_non_numeric <= delta.state_layout_tokens
    {
        return BodyDiff::StateLayoutDifference;
    }

    if changed_ratio <= SMALL_GUARDED_RATIO {
        return BodyDiff::SmallGuardedBehavior;
    }

    BodyDiff::LargeSemanticDivergence
}

/// Minimum shared meaningful anchor tokens to treat a pair as function variants.
const MIN_SHARED_ANCHORS: usize = 4;
/// Minimum shared fraction of the smaller body vocabulary when the anchor floor is not met.
const MIN_VARIANT_SHARED_RATIO: f64 = 0.20;
const SMALL_GUARDED_RATIO: f64 = 0.15;

pub(crate) fn single_profile_record(
    symbol: &str,
    profile: &'static str,
    body_patched: &str,
    required_boundaries: Vec<&'static str>,
) -> ComparisonRecord {
    let mut profile_body_hashes = BTreeMap::new();
    profile_body_hashes.insert(profile, source_hash(body_patched));
    ComparisonRecord {
        symbol: symbol.to_string(),
        profiles_present: vec![profile],
        profile_body_hashes,
        diff: BodyDiff::IdenticalAfterNormalization,
        selected_strategy: SelectedStrategy::SingleProfile,
        required_boundaries,
        reason_if_not_importable: None,
    }
}

pub(crate) fn dual_profile_record(
    symbol: &str,
    base_profile: &'static str,
    base_patched: &str,
    xetex_patched: &str,
    selected_strategy: SelectedStrategy,
) -> ComparisonRecord {
    let diff = classify_body_diff(base_patched, xetex_patched);
    let mut profile_body_hashes = BTreeMap::new();
    profile_body_hashes.insert(base_profile, source_hash(base_patched));
    profile_body_hashes.insert("xetex", source_hash(xetex_patched));
    let required_boundaries = unsupported_external_boundaries(xetex_patched);
    let reason_if_not_importable = if diff.is_unknown() {
        Some(format!(
            "body diff for `{symbol}` could not be classified (base vs xetex are not a defensible variant pair)"
        ))
    } else {
        None
    };
    ComparisonRecord {
        symbol: symbol.to_string(),
        profiles_present: vec![base_profile, "xetex"],
        profile_body_hashes,
        diff,
        selected_strategy,
        required_boundaries,
        reason_if_not_importable,
    }
}

fn body_tokens(normalized: &str) -> Vec<&str> {
    normalized.split(' ').filter(|token| !token.is_empty()).collect()
}

fn meaningful_vocabulary<'a>(tokens: &[&'a str]) -> std::collections::BTreeSet<&'a str> {
    tokens
        .iter()
        .copied()
        .filter(|token| is_meaningful_token(token))
        .collect()
}

/// True for tokens that are not scaffold keywords, punctuation, or numeric literals.
fn is_meaningful_token(token: &str) -> bool {
    if token.len() <= 1 {
        return false;
    }
    if is_numeric_token(token) {
        return false;
    }
    if TRIVIAL_TOKENS.contains(&token) {
        return false;
    }
    // Operator and punctuation clusters like `=>`, `::`, and `&mut` do not count as shared vocabulary.
    token.chars().any(|c| c.is_ascii_alphabetic() || c == '_')
}

const TRIVIAL_TOKENS: &[&str] = &[
    "let", "mut", "as", "if", "else", "return", "match", "loop", "while", "for", "in", "fn",
    "self", "Self", "unsafe", "pub", "crate", "i32", "i64", "u8", "u32", "u64", "usize", "isize",
    "integer", "c_int", "c_long", "core", "ffi",
];

struct TokenDelta {
    changed_tokens: usize,
    changed_non_numeric: usize,
    state_layout_tokens: usize,
}

impl TokenDelta {
    fn between(base: &[&str], xetex: &[&str]) -> Self {
        let mut base_counts: BTreeMap<&str, isize> = BTreeMap::new();
        for token in base {
            *base_counts.entry(token).or_insert(0) += 1;
        }
        let mut xetex_counts: BTreeMap<&str, isize> = BTreeMap::new();
        for token in xetex {
            *xetex_counts.entry(token).or_insert(0) += 1;
        }

        let mut changed_tokens = 0usize;
        let mut changed_non_numeric = 0usize;
        let mut state_layout_tokens = 0usize;

        let mut keys: Vec<&str> = base_counts.keys().copied().collect();
        for key in xetex_counts.keys() {
            if !base_counts.contains_key(key) {
                keys.push(*key);
            }
        }

        for key in keys {
            let in_base = base_counts.get(key).copied().unwrap_or(0);
            let in_xetex = xetex_counts.get(key).copied().unwrap_or(0);
            let delta = (in_base - in_xetex).unsigned_abs();
            if delta == 0 {
                continue;
            }
            changed_tokens += delta;
            if !is_numeric_token(key) {
                changed_non_numeric += delta;
            }
            if is_state_layout_token(key) {
                state_layout_tokens += delta;
            }
        }

        Self {
            changed_tokens,
            changed_non_numeric,
            state_layout_tokens,
        }
    }
}

fn is_numeric_token(token: &str) -> bool {
    let trimmed = token.trim_end_matches(|c: char| matches!(c, ',' | ';' | ')' | '('));
    !trimmed.is_empty()
        && trimmed
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '_' | 'i' | '3' | '2' | '6' | '4' | '8'))
        && trimmed.chars().any(|c| c.is_ascii_digit())
}

fn is_state_layout_token(token: &str) -> bool {
    token.contains("self.state.") || token.contains(".offset") || token.contains("as_mut_ptr")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_bodies_classify_identical() {
        let body = "{ let a = call_thing(); other_call(a); return; }";
        assert_eq!(
            classify_body_diff(body, body),
            BodyDiff::IdenticalAfterNormalization
        );
    }

    #[test]
    fn whitespace_only_difference_classifies_identical() {
        let base = "{ let a = b;\n    return a; }";
        let xetex = "{   let a = b;  return a;  }";
        assert_eq!(
            classify_body_diff(base, xetex),
            BodyDiff::IdenticalAfterNormalization
        );
    }

    #[test]
    fn constant_only_difference_classifies_only_constants() {
        let base = "{ let a = foo bar baz qux limit set value 100 ; return a ; }";
        let xetex = "{ let a = foo bar baz qux limit set value 200 ; return a ; }";
        assert_eq!(classify_body_diff(base, xetex), BodyDiff::OnlyConstantsDiffer);
    }

    #[test]
    fn host_boundary_identifier_classifies_host_boundary() {
        let base = "{ let a = foo bar baz qux measure char width ; return a ; }";
        let xetex = "{ let a = foo bar baz qux getnativecharwd char width ; return a ; }";
        assert_eq!(
            classify_body_diff(base, xetex),
            BodyDiff::HostBoundaryDifference
        );
    }

    #[test]
    fn small_local_delta_classifies_small_guarded() {
        let common: String = (0..40).map(|i| format!("tok{i} ")).collect();
        let base = format!("{{ {common} alpha beta gamma }}");
        let xetex = format!("{{ {common} alpha beta delta }}");
        assert_eq!(
            classify_body_diff(&base, &xetex),
            BodyDiff::SmallGuardedBehavior
        );
    }

    #[test]
    fn related_but_divergent_bodies_classify_large_divergence() {
        let core: String = (0..20).map(|i| format!("shared{i} ")).collect();
        let base = format!("{{ {core} basea baseb basec based basee basef baseg baseh }}");
        let xetex = format!("{{ {core} xa xb xc xd xe xf xg xh xi xj xk xl xm xn xo xp }}");
        assert_eq!(
            classify_body_diff(&base, &xetex),
            BodyDiff::LargeSemanticDivergence
        );
    }

    #[test]
    fn disjoint_bodies_classify_unknown() {
        let base: String = (0..40).map(|i| format!("alpha{i} ")).collect();
        let xetex: String = (0..40).map(|i| format!("omega{i} ")).collect();
        let base = format!("{{ {base} }}");
        let xetex = format!("{{ {xetex} }}");
        assert_eq!(classify_body_diff(&base, &xetex), BodyDiff::Unknown);
    }

    #[test]
    fn unknown_diff_marks_record_not_importable() {
        let base: String = (0..40).map(|i| format!("alpha{i} ")).collect();
        let xetex: String = (0..40).map(|i| format!("omega{i} ")).collect();
        let record = dual_profile_record(
            "synthetic_divergent",
            "tex",
            &format!("{{ {base} }}"),
            &format!("{{ {xetex} }}"),
            SelectedStrategy::UseXetex,
        );
        assert!(record.diff.is_unknown());
        assert!(record.reason_if_not_importable.is_some());
    }
}
