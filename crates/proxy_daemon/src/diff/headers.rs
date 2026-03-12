use std::collections::{BTreeMap, BTreeSet};

use super::HeaderDiff;

/// Compare two header maps and return a list of diffs.
/// Supports duplicate header keys by grouping values into Vec<String>.
pub fn diff_headers(
    old_headers: &[(String, String)],
    new_headers: &[(String, String)],
) -> Vec<HeaderDiff> {
    let mut old_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (k, v) in old_headers {
        old_map.entry(k.to_lowercase()).or_default().push(v.clone());
    }
    let mut new_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (k, v) in new_headers {
        new_map.entry(k.to_lowercase()).or_default().push(v.clone());
    }

    let all_keys: BTreeSet<&String> = old_map.keys().chain(new_map.keys()).collect();

    let mut diffs = Vec::new();
    for key in all_keys {
        match (old_map.get(key), new_map.get(key)) {
            (Some(old_vals), Some(new_vals)) => {
                if old_vals != new_vals {
                    diffs.push(HeaderDiff::Modified {
                        key: key.clone(),
                        old_value: old_vals.join(", "),
                        new_value: new_vals.join(", "),
                    });
                }
            }
            (Some(old_vals), None) => {
                diffs.push(HeaderDiff::Removed {
                    key: key.clone(),
                    value: old_vals.join(", "),
                });
            }
            (None, Some(new_vals)) => {
                diffs.push(HeaderDiff::Added {
                    key: key.clone(),
                    value: new_vals.join(", "),
                });
            }
            (None, None) => unreachable!(),
        }
    }

    diffs
}
