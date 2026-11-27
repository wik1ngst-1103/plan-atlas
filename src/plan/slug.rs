use unicode_normalization::UnicodeNormalization;

/// Lowercase ASCII slug: text is NFD-decomposed and combining marks dropped
/// (so NFC and NFD input agree), đ -> d, any run of non-alphanumerics collapsed
/// into a single '-', trimmed of leading/trailing '-'.
pub fn slugify(s: &str) -> String {
    let mut out = String::new();
    let mut pending_dash = false;
    for c in s.nfd() {
        if ('\u{0300}'..='\u{036f}').contains(&c) {
            continue;
        }
        let c = match c {
            'đ' | 'Đ' => 'd',
            other => other,
        };
        if c.is_ascii_alphanumeric() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.push(c.to_ascii_lowercase());
        } else {
            pending_dash = true;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::slugify;

    #[test]
    fn slugs() {
        assert_eq!(slugify("01. Overview"), "01-overview");
        assert_eq!(slugify("Thiết kế đăng nhập!"), "thiet-ke-dang-nhap");
        assert_eq!(slugify("  --A & B--  "), "a-b");
    }

    #[test]
    fn slug_nfd_equals_nfc() {
        use unicode_normalization::UnicodeNormalization;
        let nfc: String = "Thiết kế đăng nhập".nfc().collect();
        let nfd: String = "Thiết kế đăng nhập".nfd().collect();
        assert_ne!(nfc, nfd);
        assert_eq!(slugify(&nfc), "thiet-ke-dang-nhap");
        assert_eq!(slugify(&nfd), "thiet-ke-dang-nhap");
    }

    #[test]
    fn slug_latin_diacritics() {
        assert_eq!(slugify("café"), "cafe");
    }
}
