//! Split outbound text into Telegram-sized pages (parity with `tg_agent_relay.send.paginate`).

/// Split `text` into pages on line boundaries; hard-split overlong lines.
///
/// A message within `page_size` is a single unmodified page (no `[k/n]` prefix).
/// Empty text yields an empty list. Non-positive `page_size` returns the whole text as one page.
pub fn paginate(text: &str, page_size: usize) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    if page_size == 0 {
        return vec![text.to_string()];
    }
    if text.len() <= page_size {
        return vec![text.to_string()];
    }

    let mut pages: Vec<String> = Vec::new();
    let mut cur = String::new();

    for line in text.split('\n') {
        let cand = if cur.is_empty() {
            line.to_string()
        } else {
            format!("{cur}\n{line}")
        };

        if cand.len() <= page_size {
            cur = cand;
            continue;
        }

        if !cur.is_empty() {
            pages.push(std::mem::take(&mut cur));
        }

        if line.len() <= page_size {
            cur = line.to_string();
        } else {
            let mut rest = line;
            while rest.len() > page_size {
                let (chunk, remainder) = rest.split_at(page_size);
                pages.push(chunk.to_string());
                rest = remainder;
            }
            cur = rest.to_string();
        }
    }

    if !cur.is_empty() {
        pages.push(cur);
    }

    if pages.is_empty() {
        vec![text.to_string()]
    } else {
        pages
    }
}

/// Apply `[k/n]` prefixes when there is more than one page (tg-send / `build_page_payloads` shape).
pub fn format_page_payloads(pages: &[String]) -> Vec<String> {
    let total = pages.len();
    if total <= 1 {
        return pages.to_vec();
    }
    pages
        .iter()
        .enumerate()
        .map(|(idx, page)| format!("[{}/{total}]\n{page}", idx + 1))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{format_page_payloads, paginate};

    #[test]
    fn short_single_page() {
        assert_eq!(paginate("hello", 3500), vec!["hello".to_string()]);
    }

    #[test]
    fn empty_yields_none() {
        assert!(paginate("", 10).is_empty());
    }

    #[test]
    fn multi_page_rejoins_on_newlines() {
        let long_lines: String = (0..50)
            .map(|i| format!("line-{i:04}-{}", "x".repeat(20)))
            .collect::<Vec<_>>()
            .join("\n");
        let pages = paginate(&long_lines, 200);
        assert!(pages.len() > 1);
        assert!(pages.iter().all(|p| p.len() <= 200));
        assert_eq!(pages.join("\n"), long_lines);
    }

    #[test]
    fn hard_split_single_line() {
        let hard = "a".repeat(50);
        let hp = paginate(&hard, 20);
        assert_eq!(hp.len(), 3);
        assert_eq!(hp.join(""), hard);
    }

    #[test]
    fn format_payloads_single_unchanged() {
        let pages = vec!["only".to_string()];
        assert_eq!(format_page_payloads(&pages), pages);
    }

    #[test]
    fn format_payloads_adds_prefix() {
        let pages = vec!["a".to_string(), "b".to_string()];
        let out = format_page_payloads(&pages);
        assert_eq!(out[0], "[1/2]\na");
        assert_eq!(out[1], "[2/2]\nb");
    }
}