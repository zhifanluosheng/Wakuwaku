// Verification codes in a new letter: the words a website or an operator
// uses around one, and the code itself. Only a letter that really has both is
// worth a notice in the corner; ordinary mail never is.
//
// The body is the letter's text as the Mail page reads it (letters.rs), not
// the snippet kept with its headers: a code is often past the first line.
use std::sync::{Arc, Mutex};

use super::imap::Session;
use super::letters;
use super::store::{Folder, Store};
use crate::Shared;

// How many of the newest letters one look reads whole. A burst larger than
// this is mostly not codes; the rest are read when the page opens them.
const MOST: usize = 5;

// A code is up to eight characters; longer numbers are order numbers,
// tracking numbers, phone numbers and the like.
const SHORTEST: usize = 4;
const LONGEST: usize = 8;
// The nearest a code may sit to the word that names it, in characters.
const AFTER: usize = 80;
const BEFORE: usize = 40;
// Below this, the words are not about a code.
const ENOUGH: i32 = 55;

// The words that say a code is coming. The weight says how sure the word is
// by itself: "verification code" is a code; "code" alone may be a promo code
// or an order code.
const KEYWORDS: &[(&str, i32)] = &[
    ("短信验证码", 110),
    ("验证码", 100),
    ("校验码", 100),
    ("动态密码", 100),
    ("动态码", 95),
    ("一次性密码", 105),
    ("确认码", 90),
    ("安全码", 90),
    ("授权码", 75),
    ("登录码", 85),
    ("verification code", 100),
    ("one-time password", 105),
    ("one time password", 105),
    ("one-time code", 100),
    ("one time code", 100),
    ("security code", 95),
    ("confirmation code", 95),
    ("authentication code", 95),
    ("passcode", 90),
    ("otp", 85),
    ("pin code", 80),
    ("pin", 75),
    ("code", 40),
];

// Things a number near a code word may also be. A code next to one of these
// is less likely to be the code, though a real one still wins.
const NOT_A_CODE: &[&str] = &[
    "order", "invoice", "phone", "mobile", "telephone", "fax", "zip", "postal", "tracking", "coupon", "promo", "discount",
    "price", "amount", "total", "balance", "cvv", "cvc", "referral", "invite", "invitation", "voucher", "gift", "qr",
    "area code", "code of conduct", "订单", "单号", "编号", "电话", "手机", "传真", "邮编", "快递",
    "物流", "运单", "优惠", "折扣", "价格", "金额", "余额", "发票", "卡号", "账号", "座位", "航班", "$", "¥", "￥", "usd",
    "cny", "checkout", "redeem", "claim", "offer", "bonus", "%",
];

#[derive(Clone)]
struct Candidate {
    text: String,
    at: usize,
    end: usize,
    digits: usize,
    letters: usize,
}

// A new letter's code, if it has one: the subject first (short and to the
// point), then the text with words of its own, then the subject's words over
// a bare number at the top of the text (the strict distance keeps an order
// number further down from passing as the code).
pub fn find_code(subject: &str, body: &str) -> Option<String> {
    if let Some(code) = detect(subject, true) {
        return Some(code);
    }
    if let Some(code) = detect(body, false) {
        return Some(code);
    }
    if subject.trim().is_empty() {
        return None;
    }
    detect_with(&format!("{}\n{}", subject.trim(), body), false, 8)
}

// Read the newest letters whole and notice the codes among them. A body that
// cannot be read is left to the next look; one letter's trouble does not stop
// the sync.
pub fn new_mail(sh: &Arc<Shared>, conn: &mut Session, st: &Mutex<Store>, uids: &[u32], since: u32) {
    for &uid in uids.iter().rev().filter(|uid| **uid > since).take(MOST) {
        let entry = st.lock().unwrap().inbox.letters.get(&uid).cloned();
        let Some(entry) = entry else { continue };
        if entry.seen || entry.from.address.trim().is_empty() {
            continue;
        }
        // The sync's download has usually brought the body already; asking
        // the server again would only cost a round trip.
        let kept = st.lock().unwrap().body(Folder::Inbox, uid);
        let raw = match kept {
            Some(raw) => raw,
            None => match conn.letter(uid) {
                Ok(Some((raw, _))) => {
                    st.lock().unwrap().keep_body(Folder::Inbox, uid, &raw, false);
                    raw
                }
                Ok(None) => continue,
                // The connection itself is in trouble: the sync's own retry
                // takes over rather than waiting on letter after letter.
                Err(_) => break,
            },
        };
        let Some(message) = letters::parse(&raw) else { continue };
        let body = letters::text(&message);
        if let Some(code) = find_code(&entry.subject, &body) {
            crate::toast::mail_code(sh, &entry.from.name, &entry.from.address, &code);
            sh.log(&format!("mail: a verification code in letter {uid}"));
        }
    }
}

fn detect(raw: &str, subject: bool) -> Option<String> {
    detect_with(raw, subject, AFTER)
}

fn detect_with(raw: &str, subject: bool, after: usize) -> Option<String> {
    let text = normalize(raw);
    if text.is_empty() {
        return None;
    }
    let chars: Vec<char> = text.chars().collect();
    let lower: Vec<char> = text.to_lowercase().chars().collect();
    let hits = keyword_hits(&lower);
    if hits.is_empty() {
        return None;
    }
    let mut candidates = candidates(&chars);
    candidates.sort_by_key(|c| (c.at, c.end));
    candidates.dedup_by(|a, b| a.at == b.at && a.end == b.end);
    let mut best: Option<(i32, String)> = None;
    for candidate in candidates {
        if beside_address(&chars, &candidate) {
            continue;
        }
        // A number in a link or a tracking address is the link's, not a code.
        if in_link(&chars, &candidate) {
            continue;
        }
        // A year is not a verification code, however near the words it is.
        if is_year(&candidate) {
            continue;
        }
        let mut score: Option<i32> = None;
        for &(from, to, weight) in &hits {
            // A code is not part of the word that named it (code123).
            if candidate.at < to && candidate.end > from {
                continue;
            }
            let distance = if candidate.at >= to {
                if candidate.at - to > after {
                    continue;
                }
                candidate.at - to
            } else if candidate.end <= from {
                if from - candidate.end > BEFORE {
                    continue;
                }
                from - candidate.end
            } else {
                0
            };
            let mut value = weight * 2 + length_bonus(&candidate) - distance as i32;
            if candidate.end <= from {
                value -= 15;
            }
            if subject {
                value += 10;
            }
            value -= not_a_code(&chars, &candidate);
            score = Some(score.map_or(value, |old: i32| old.max(value)));
        }
        if let Some(value) = score.filter(|v| *v >= ENOUGH) {
            if best.as_ref().is_none_or(|(old, _)| value > *old) {
                best = Some((value, candidate.text));
            }
        }
    }
    best.map(|(_, code)| code)
}

// Full-width digits and punctuation as their plain forms, so a code typed
// with a Chinese keyboard reads the same.
fn normalize(raw: &str) -> String {
    raw.chars()
        .map(|c| match c {
            '０'..='９' => char::from_u32('0' as u32 + (c as u32 - '０' as u32)).unwrap_or(c),
            'Ａ'..='Ｚ' => char::from_u32('A' as u32 + (c as u32 - 'Ａ' as u32)).unwrap_or(c),
            'ａ'..='ｚ' => char::from_u32('a' as u32 + (c as u32 - 'ａ' as u32)).unwrap_or(c),
            '：' => ':',
            '　' => ' ',
            _ => c,
        })
        .collect()
}

fn keyword_hits(lower: &[char]) -> Vec<(usize, usize, i32)> {
    let mut hits = Vec::new();
    for &(word, weight) in KEYWORDS {
        let wanted: Vec<char> = word.to_lowercase().chars().collect();
        if wanted.is_empty() || wanted.len() > lower.len() {
            continue;
        }
        for at in 0..=lower.len() - wanted.len() {
            if lower[at..at + wanted.len()] != wanted[..] {
                continue;
            }
            // Plain letters only count as a word: "code" in "barcode" does
            // not name a code.
            if wanted[0].is_ascii_alphanumeric() && at > 0 && lower[at - 1].is_ascii_alphanumeric() {
                continue;
            }
            let end = at + wanted.len();
            if wanted[wanted.len() - 1].is_ascii_alphanumeric() && end < lower.len() && lower[end].is_ascii_alphanumeric() {
                continue;
            }
            hits.push((at, end, weight));
        }
    }
    hits
}

fn candidates(chars: &[char]) -> Vec<Candidate> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            if (SHORTEST..=LONGEST).contains(&(i - start)) {
                out.push(Candidate {
                    text: chars[start..i].iter().collect(),
                    at: start,
                    end: i,
                    digits: i - start,
                    letters: 0,
                });
            }
            continue;
        }
        if c.is_ascii_alphanumeric() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_alphanumeric() {
                i += 1;
            }
            let letters = chars[start..i].iter().filter(|c| c.is_ascii_alphabetic()).count();
            let digits = chars[start..i].iter().filter(|c| c.is_ascii_digit()).count();
            let length = i - start;
            if (SHORTEST..=LONGEST).contains(&length) && letters >= 2 && digits >= 2 {
                out.push(Candidate {
                    text: chars[start..i].iter().collect(),
                    at: start,
                    end: i,
                    digits,
                    letters,
                });
            }
            continue;
        }
        i += 1;
    }
    // "123 456" and "123-456": one code written in groups.
    for start in 0..chars.len() {
        if !chars[start].is_ascii_digit() {
            continue;
        }
        let mut groups: Vec<(usize, usize)> = Vec::new();
        let mut at = start;
        while groups.len() < 4 {
            let from = at;
            while at < chars.len() && chars[at].is_ascii_digit() {
                at += 1;
            }
            if at == from {
                break;
            }
            groups.push((from, at));
            if at < chars.len() && (chars[at] == ' ' || chars[at] == '-') {
                at += 1;
            } else {
                break;
            }
        }
        if groups.len() < 2 {
            continue;
        }
        let total: usize = groups.iter().map(|(from, to)| to - from).sum();
        let first = groups[0].1 - groups[0].0;
        // A date (2026-10-21), not a code.
        if total == 8 && groups.len() == 3 && first == 4 {
            continue;
        }
        if !(SHORTEST..=LONGEST).contains(&total) {
            continue;
        }
        let end = groups.last().map(|(_, to)| *to).unwrap_or(at);
        let text: String = groups.iter().flat_map(|(from, to)| chars[*from..*to].iter().copied()).collect();
        out.push(Candidate { text, at: start, end, digits: total, letters: 0 });
    }
    out
}

fn beside_address(chars: &[char], candidate: &Candidate) -> bool {
    (candidate.at > 0 && chars[candidate.at - 1] == '@') || (candidate.end < chars.len() && chars[candidate.end] == '@')
}

fn in_link(chars: &[char], candidate: &Candidate) -> bool {
    let edge = |c: char| c.is_whitespace() || matches!(c, '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | '"' | '\'' | '，' | '。' | '、');
    let mut from = candidate.at;
    while from > 0 && !edge(chars[from - 1]) {
        from -= 1;
    }
    let mut to = candidate.end;
    while to < chars.len() && !edge(chars[to]) {
        to += 1;
    }
    let token: String = chars[from..to].iter().collect::<String>().to_lowercase();
    token.contains("://")
        || token.starts_with("www.")
        || (token.contains('/') && token.contains('.'))
        || (token.contains('?') && token.contains('='))
        || [".png", ".jpg", ".jpeg", ".gif", ".webp", ".svg", ".html", ".php"].iter().any(|ext| token.contains(ext))
}

fn is_year(candidate: &Candidate) -> bool {
    candidate.letters == 0
        && candidate.digits == 4
        && candidate.text.parse::<i32>().is_ok_and(|year| (1900..=2100).contains(&year))
}

fn length_bonus(candidate: &Candidate) -> i32 {
    match candidate.digits + candidate.letters {
        4 => 16,
        5 => 22,
        6 => 26,
        7 => 24,
        8 => 22,
        _ => 0,
    }
}

fn not_a_code(chars: &[char], candidate: &Candidate) -> i32 {
    let from = candidate.at.saturating_sub(24);
    let to = (candidate.end + 24).min(chars.len());
    let around: String = chars[from..to].iter().collect::<String>().to_lowercase();
    let mut penalty = 0;
    for word in NOT_A_CODE {
        if around.contains(word) {
            penalty += 60;
        }
    }
    if around.contains("http://") || around.contains("https://") || around.contains("www.") {
        penalty += 35;
    }
    if candidate.text.len() == 4 {
        if let Ok(year) = candidate.text.parse::<i32>() {
            if (1900..=2100).contains(&year) {
                penalty += 15;
            }
        }
    }
    penalty.min(120)
}

#[cfg(test)]
mod tests {
    use super::find_code;

    #[test]
    fn a_chinese_code_is_the_number_after_the_words() {
        assert_eq!(find_code("", "您的验证码为 123456，5 分钟内有效").as_deref(), Some("123456"));
        assert_eq!(find_code("", "校验码：６５４３２１").as_deref(), Some("654321"));
        assert_eq!(find_code("", "动态密码 123 456，请勿告诉他人").as_deref(), Some("123456"));
    }

    #[test]
    fn an_english_code_may_be_letters_and_digits() {
        assert_eq!(find_code("", "Your verification code is A1B2C3.").as_deref(), Some("A1B2C3"));
        assert_eq!(find_code("", "Your security code: 654321").as_deref(), Some("654321"));
        assert_eq!(find_code("", "One-time password: 4821").as_deref(), Some("4821"));
    }

    #[test]
    fn a_code_before_the_words_is_still_a_code() {
        assert_eq!(find_code("", "123456 is your verification code").as_deref(), Some("123456"));
        assert_eq!(find_code("123456 is your verification code", "").as_deref(), Some("123456"));
        // The subject can carry the words and the text a bare number at its top.
        assert_eq!(find_code("Your verification code", "123456").as_deref(), Some("123456"));
        assert_eq!(find_code("Your verification code", "Hi,\n\n123456").as_deref(), Some("123456"));
        // A number further down, after an order word, is not that code.
        assert_eq!(find_code("Your verification code", "Your order 12345678 has shipped"), None);
    }

    #[test]
    fn ordinary_mail_has_no_code() {
        assert_eq!(find_code("", "您的订单号 12345678 已发货"), None);
        assert_eq!(find_code("", "Your order 20261021123456 has shipped"), None);
        assert_eq!(find_code("", "Call me on 13800138000"), None);
        assert_eq!(find_code("", "The date is 2026-10-21"), None);
        assert_eq!(find_code("", "Your invoice total is $1,234.56"), None);
    }

    #[test]
    fn offers_and_order_codes_are_not_verification_codes() {
        assert_eq!(find_code("", "Promo code SAVE20 for your next order"), None);
        assert_eq!(find_code("", "Your order code is 123456"), None);
        assert_eq!(find_code("", "Postal code: 123456"), None);
        assert_eq!(find_code("", "Our code of conduct for 2026 is here"), None);
        assert_eq!(find_code("", "The verification code era in 2026 is here"), None);
        assert_eq!(find_code("", "Use referral code 482913"), None);
        assert_eq!(find_code("", "Enter code MICA10 at checkout to claim the additional 10% discount."), None);
        assert_eq!(find_code("", "Claude Code hacks: https://magic.beehiiv.com/v1/5f7ce6e3-9a71-416b-99a7"), None);
    }

    #[test]
    fn an_address_is_not_a_code() {
        assert_eq!(find_code("", "Your verification code was sent to user1234@example.com"), None);
    }
}
