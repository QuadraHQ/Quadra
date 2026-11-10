const SECRET_PLACEHOLDER: &str = "<redacted>";

fn is_quote_or_space(character: char) -> bool {
    character.is_whitespace() || character == '"' || character == '\''
}

const RULES: [(&str, fn(char) -> bool); 8] = [
    ("ticket=", |c| is_quote_or_space(c) || c == '&'),
    ("\"ticket\":\"", |c| c == '"'),
    ("\"ticket\": \"", |c| c == '"'),
    ("gameinfo:", |c| is_quote_or_space(c) || c == '+'),
    (".roblosecurity=", |c| is_quote_or_space(c) || c == ';'),
    (".roblosecurity:", |c| is_quote_or_space(c) || c == ';'),
    ("roblosecurity=", |c| is_quote_or_space(c) || c == ';'),
    ("_|warning:-do-not-share-this", |c| {
        is_quote_or_space(c) || c == ';'
    }),
];

pub fn redact(text: &str) -> String {
    RULES
        .iter()
        .fold(text.to_owned(), |text, (marker, ends_secret)| {
            redact_after(&text, marker, ends_secret)
        })
}

fn redact_after(text: &str, marker: &str, ends_secret: &fn(char) -> bool) -> String {
    let lowercase = text.to_ascii_lowercase();
    let keeps_marker = !marker.starts_with('_');
    let mut redacted = String::with_capacity(text.len());
    let mut copied_up_to = 0;

    while let Some(found) = lowercase[copied_up_to..].find(marker) {
        let marker_start = copied_up_to + found;
        let marker_end = marker_start + marker.len();
        let secret_end = text[marker_end..]
            .find(ends_secret)
            .map_or(text.len(), |length| marker_end + length);

        let kept_until = if keeps_marker {
            marker_end
        } else {
            marker_start
        };
        redacted.push_str(&text[copied_up_to..kept_until]);
        redacted.push_str(SECRET_PLACEHOLDER);
        copied_up_to = secret_end;
    }

    redacted.push_str(&text[copied_up_to..]);
    redacted
}

#[cfg(test)]
mod tests {
    use super::redact;

    #[test]
    fn hides_join_tickets() {
        assert_eq!(
            redact("https://x/join?ticket=ABC123&place=1"),
            "https://x/join?ticket=<redacted>&place=1"
        );
        assert_eq!(
            redact(r#"{"Ticket": "ABC", "x": 1}"#),
            r#"{"Ticket": "<redacted>", "x": 1}"#
        );
        assert_eq!(redact("gameinfo:SECRET+rest"), "gameinfo:<redacted>+rest");
    }

    #[test]
    fn hides_the_roblox_cookie_in_all_its_forms() {
        assert_eq!(
            redact("Cookie: .ROBLOSECURITY=abc; other=1"),
            "Cookie: .ROBLOSECURITY=<redacted>; other=1"
        );
        assert_eq!(
            redact("value _|WARNING:-DO-NOT-SHARE-THIS.--Sharing-this|_abc123 end"),
            "value <redacted> end"
        );
    }

    #[test]
    fn leaves_other_text_alone() {
        let text = "Joining game 'abc' place 1 at 10.0.0.1 ünïcode";
        assert_eq!(redact(text), text);
    }
}
