//! Fixed display spellings used in this kit (`BTF_NAME`, `DMVPN_LABEL`).
//! Prefer these constants; probe via `to_ascii_lowercase()` instead of hardcoding
//! alternate-case string literals in tests.

use rand::Rng;

/// Remark / display token (ASCII uppercase).
pub const BTF_NAME: &str = "BTF";

/// Export-folder label (ASCII uppercase).
pub const DMVPN_LABEL: &str = "DMVPN";

/// Alphabet for batch tags (no 0/O/1/l/I — readable in phone lists).
const BATCH_ALPHABET: &[u8] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";

fn needle_lower(canon: &str) -> String {
    canon.to_ascii_lowercase()
}

/// Rewrite every wrong-case run of `canon` → `canon` (exact). Does **not** prepend if absent.
pub fn uppercase_token(s: &str, canon: &str) -> String {
    let needle = needle_lower(canon);
    let nlen = needle.len();
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    loop {
        let lower = rest.to_ascii_lowercase();
        match lower.find(needle.as_str()) {
            Some(pos) => {
                out.push_str(&rest[..pos]);
                out.push_str(canon);
                rest = &rest[pos + nlen..];
            }
            None => {
                out.push_str(rest);
                break;
            }
        }
    }
    out
}

pub fn uppercase_person_name(s: &str) -> String {
    uppercase_token(s, BTF_NAME)
}

pub fn uppercase_dmvpn_label(s: &str) -> String {
    uppercase_token(s, DMVPN_LABEL)
}

/// Normalize person-name runs, then if still no [`BTF_NAME`], prefix it
/// (or return only the name when empty). For remarks / NetMod `ps` / SlipNet names.
pub fn ensure_person_name(s: &str) -> String {
    let out = uppercase_person_name(s);
    if out.contains(BTF_NAME) {
        return out;
    }
    let t = out.trim();
    if t.is_empty() {
        BTF_NAME.to_string()
    } else {
        format!("{BTF_NAME} {t}")
    }
}

/// Apply both person-name and DMVPN label uppercase fixes (no forced prefixes for DMVPN).
pub fn normalize_display_text(s: &str) -> String {
    uppercase_dmvpn_label(&uppercase_person_name(s))
}

/// Short random tag shared by one generate run (e.g. `K7HM`) so same-day batches don't collide.
pub fn new_batch_tag() -> String {
    let mut rng = rand::thread_rng();
    (0..4)
        .map(|_| {
            let i = rng.gen_range(0..BATCH_ALPHABET.len());
            BATCH_ALPHABET[i] as char
        })
        .collect()
}

/// Display name for one link in a batch: `{base}-{tag}-01`
pub fn batch_item_label(base: &str, batch: &str, index_1based: usize) -> String {
    let base = ensure_person_name(base);
    format!("{base}-{batch}-{index_1based:02}")
}

/// Display name for the combined “all resolvers” link: `{base}-{tag}-all`
pub fn batch_all_label(base: &str, batch: &str) -> String {
    let base = ensure_person_name(base);
    format!("{base}-{batch}-all")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mangled(canon: &str, lower_mask: &[bool], suffix: &str) -> String {
        assert_eq!(lower_mask.len(), canon.len());
        let mut s: String = canon
            .chars()
            .zip(lower_mask.iter())
            .map(|(c, low)| if *low { c.to_ascii_lowercase() } else { c })
            .collect();
        s.push_str(suffix);
        s
    }

    #[test]
    fn person_name_wrong_case() {
        let all_lower = format!("{}jang", needle_lower(BTF_NAME));
        assert_eq!(uppercase_person_name(&all_lower), format!("{BTF_NAME}jang"));
        assert_eq!(
            uppercase_person_name(&mangled(BTF_NAME, &[false, true, true], "Jang")),
            format!("{BTF_NAME}Jang")
        );
        assert_eq!(ensure_person_name(&all_lower), format!("{BTF_NAME}jang"));
    }

    #[test]
    fn person_name_prefix_when_missing() {
        assert_eq!(
            ensure_person_name("My DNSTT+SSH"),
            format!("{BTF_NAME} My DNSTT+SSH")
        );
        assert_eq!(ensure_person_name(""), BTF_NAME);
    }

    #[test]
    fn dmvpn_label_wrong_case() {
        let mixed = mangled(DMVPN_LABEL, &[true, true, true, true, true], "");
        assert_eq!(uppercase_dmvpn_label(&mixed), DMVPN_LABEL);
        assert_eq!(
            uppercase_dmvpn_label(&format!("out/{}/x", needle_lower(DMVPN_LABEL))),
            format!("out/{DMVPN_LABEL}/x")
        );
    }

    #[test]
    fn keys_without_person_name_unchanged() {
        assert_eq!(uppercase_person_name("demo"), "demo");
    }

    #[test]
    fn normalize_display_applies_both() {
        let s = format!("{} / {}", needle_lower(BTF_NAME), needle_lower(DMVPN_LABEL));
        assert_eq!(
            normalize_display_text(&s),
            format!("{BTF_NAME} / {DMVPN_LABEL}")
        );
    }

    #[test]
    fn batch_labels_include_tag_and_index() {
        let tag = "K7HM";
        assert_eq!(batch_item_label("BTFJang891", tag, 1), "BTFJang891-K7HM-01");
        assert_eq!(
            batch_item_label("BTFJang891", tag, 50),
            "BTFJang891-K7HM-50"
        );
        assert_eq!(batch_all_label("BTFJang891", tag), "BTFJang891-K7HM-all");
        let a = new_batch_tag();
        let b = new_batch_tag();
        assert_eq!(a.len(), 4);
        assert!(a.chars().all(|c| BATCH_ALPHABET.contains(&(c as u8))));
        assert_ne!(a, b);
    }
}
