//! Ranked, case-insensitive scope lookup, independent of the GUI.

/// Match ordered characters, preferring names over their enclosing scope paths.
pub fn search<'a>(scopes: &'a [Vec<String>], query: &str) -> Vec<&'a [String]> {
    ranked(
        scopes
            .iter()
            .map(|scope| (scope.as_slice(), scope.as_slice())),
        query,
    )
}

/// Use the same ranking for scope paths and individual signal names.
pub(super) fn ranked<'a, T>(
    candidates: impl IntoIterator<Item = (T, &'a [String])>,
    query: &str,
) -> Vec<T> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return candidates.into_iter().map(|(item, _)| item).collect();
    }
    let characters: Vec<_> = query.chars().collect();
    let mut matches: Vec<_> = candidates
        .into_iter()
        .filter_map(|(item, scope)| {
            let name = scope.last()?.to_lowercase();
            let path = scope.join(".").to_lowercase();
            let (priority, span, start) = if path == query || name == query {
                (0, characters.len(), 0)
            } else if let Some((span, start)) = subsequence_span(&name, &characters) {
                let priority = if name.starts_with(&query) {
                    1
                } else if name.contains(&query) {
                    2
                } else {
                    3
                };
                (priority, span, start)
            } else {
                let (span, start) = subsequence_span(&path, &characters)?;
                (4, span, start)
            };
            Some((item, (priority, span, start, path.chars().count()), scope))
        })
        .collect();
    matches.sort_by(|(_, a, left), (_, b, right)| a.cmp(b).then_with(|| left.cmp(right)));
    matches.into_iter().map(|(item, _, _)| item).collect()
}

fn subsequence_span(text: &str, query: &[char]) -> Option<(usize, usize)> {
    let mut starts = vec![None; query.len()];
    let mut best = None;
    for (position, character) in text.chars().enumerate() {
        // Work backwards so repeated query characters cannot reuse one letter.
        for index in (0..query.len()).rev() {
            if character != query[index] {
                continue;
            }
            let start = if index == 0 {
                Some(position)
            } else {
                starts[index - 1]
            };
            if let Some(start) = start {
                starts[index] = Some(start);
                if index + 1 == query.len() {
                    let score = (position - start + 1, start);
                    best = Some(best.map_or(score, |old: (usize, usize)| old.min(score)));
                }
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(names: &[&str]) -> Vec<Vec<String>> {
        names
            .iter()
            .map(|name| name.split('.').map(str::to_owned).collect())
            .collect()
    }

    fn found(scopes: &[Vec<String>], query: &str) -> Vec<String> {
        search(scopes, query)
            .iter()
            .map(|path| path.join("."))
            .collect()
    }

    #[test]
    fn finds_skipped_characters_across_scope_boundaries_ignoring_case() {
        let scopes = paths(&["tb.clock", "tb.dut", "tb.other"]);
        assert_eq!(found(&scopes, "TD"), ["tb.dut"]);
        assert_eq!(found(&scopes, "dt"), ["tb.dut"]);
        assert!(found(&scopes, "db").is_empty());
        assert_eq!(found(&scopes, "tt"), ["tb.other", "tb.dut"]);
        assert!(found(&scopes, "ttt").is_empty());
        assert!(found(&scopes, "missing").is_empty());
    }

    #[test]
    fn ranks_exact_leaf_names_before_prefix_substring_and_fuzzy_matches() {
        let scopes = paths(&[
            "top.a_uart",
            "top.u___a___r___t",
            "top.uart",
            "top.uart_rx",
            "top.bus.uart",
        ]);
        assert_eq!(
            found(&scopes, "UART"),
            [
                "top.uart",
                "top.bus.uart",
                "top.uart_rx",
                "top.a_uart",
                "top.u___a___r___t",
            ]
        );
        assert_eq!(found(&scopes, "TOP.BU"), ["top.bus.uart"]);
    }

    #[test]
    fn prefers_the_most_compact_subsequence_including_later_occurrences() {
        let scopes = paths(&["top.a___b_c", "top.a___a_bc"]);
        assert_eq!(found(&scopes, "abc"), ["top.a___a_bc", "top.a___b_c"]);
    }

    #[test]
    fn keeps_equally_named_scopes_distinct_and_orders_ties_by_path() {
        let scopes = paths(&["tb.b.uart", "tb.a.uart"]);
        assert_eq!(found(&scopes, "uart"), ["tb.a.uart", "tb.b.uart"]);
    }

    #[test]
    fn supports_unicode_scope_names_and_trimmed_queries() {
        let scopes = paths(&["tb.Üart_接收", "tb.other"]);
        assert_eq!(found(&scopes, "  Ü接  "), ["tb.Üart_接收"]);
        assert_eq!(found(&scopes, "ü接"), ["tb.Üart_接收"]);
    }

    #[test]
    fn empty_queries_restore_all_scopes_including_root() {
        let mut scopes = vec![Vec::new()];
        scopes.extend(paths(&["tb", "tb.dut"]));
        assert_eq!(found(&scopes, ""), ["", "tb", "tb.dut"]);
        assert_eq!(found(&scopes, "  "), ["", "tb", "tb.dut"]);
        assert!(search(&[], "dut").is_empty());
    }
}
