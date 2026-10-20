// What is playing on this computer, as Windows' own media sessions know it
// (GlobalSystemMediaTransportControlsSessionManager): the player's name, the
// track and its artist, whether it plays, and where it is. A browser playing
// a page (YouTube, Spotify's web player, Bilibili) reports itself here as one
// more player, so it shows the same way. The words of the track come from
// LRCLIB (no key, no account), asked when the track changes; the synced
// lines give the one it is on, and the card shows those around it.
//
// Shown as a built-in widget (widgets.rs, the island): 音乐 · Spotify · 歌名,
// the lyric lines in its card. Windows only; elsewhere there is nothing to
// read and no widget.

use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Default, PartialEq)]
pub struct Song {
    pub app: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub playing: bool,
    pub position_ms: u64,
    pub duration_ms: u64,
}

#[derive(Clone, Default)]
struct Lyrics {
    plain: Vec<String>,
    synced: Vec<(u64, String)>,
}

impl Lyrics {
    // The synced line at that moment (the last stamp at or before it), else
    // the first plain line.
    fn at(&self, ms: u64) -> Option<&str> {
        if !self.synced.is_empty() {
            let i = self.synced.partition_point(|(t, _)| *t <= ms);
            return self.synced.get(i.saturating_sub(1)).map(|(_, l)| l.as_str());
        }
        self.plain.first().map(String::as_str)
    }

    // The lines around it, for the card: the one it is on and those after.
    fn around(&self, ms: u64, count: usize) -> Vec<String> {
        if !self.synced.is_empty() {
            let i = self.synced.partition_point(|(t, _)| *t <= ms).saturating_sub(1);
            return self.synced.iter().skip(i).take(count).map(|(_, l)| l.clone()).collect();
        }
        self.plain.iter().take(count).cloned().collect()
    }
}

pub struct Music {
    lyrics: Arc<Mutex<Option<(String, Lyrics)>>>,
    // The track the fetch was asked for (artist, title), so one ask a track.
    asked: Option<String>,
}

impl Default for Music {
    fn default() -> Self {
        Self { lyrics: Arc::new(Mutex::new(None)), asked: None }
    }
}

impl Music {
    // What to show now: the widget's words, and its card's (the artist and
    // the lyric lines). None when nothing plays (nothing is hidden by that:
    // the widget goes, as any built-in with nothing to say).
    pub fn turn(&mut self) -> Option<(Value, Value)> {
        let song = read()?;
        let key = format!("{}\n{}", song.artist, song.title);
        if self.asked.as_deref() != Some(key.as_str()) {
            self.asked = Some(key.clone());
            let slot = self.lyrics.clone();
            let ask = song.clone();
            let asked = key.clone();
            std::thread::spawn(move || {
                let got = fetch(&ask);
                if let Ok(mut slot) = slot.lock() {
                    *slot = got.map(|l| (asked, l));
                }
            });
        }
        let lyrics = self
            .lyrics
            .lock()
            .ok()
            .and_then(|g| g.as_ref().filter(|(k, _)| *k == key).map(|(_, l)| l.clone()));
        let words = json!({
            "key": "widget.musicValue",
            "vars": { "player": song.app, "song": song.title },
        });
        let mut detail = json!({
            "sizes": ["s", "m", "l"],
            "extra": if song.artist.is_empty() { song.album.clone() } else { song.artist.clone() },
        });
        if let Some(lyrics) = lyrics {
            let rows: Vec<Value> = lyrics
                .around(song.position_ms, 6)
                .into_iter()
                .map(|line| json!({ "kind": "lyric", "text": line }))
                .collect();
            if !rows.is_empty() {
                detail["rows"] = Value::Array(rows);
            }
        }
        Some((words, detail))
    }

    pub fn reset(&mut self) {
        self.asked = None;
        if let Ok(mut slot) = self.lyrics.lock() {
            *slot = None;
        }
    }
}

// --- Windows' sessions -------------------------------------------------------

#[cfg(windows)]
fn read() -> Option<Song> {
    use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager as Manager;
    use windows::Media::Control::GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status;
    // This thread's own apartment (an error: it was taken already, which does).
    // SAFETY: a plain COM apartment call on this thread, its result let be.
    let _ = unsafe { windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED) };
    let manager = Manager::RequestAsync().ok()?.join().ok()?;
    let session = manager.GetCurrentSession().ok()?;
    let props = session.TryGetMediaPropertiesAsync().ok()?.join().ok()?;
    let title = props.Title().map(|s| s.to_string()).unwrap_or_default();
    if title.trim().is_empty() {
        return None;
    }
    let artist = props.Artist().map(|s| s.to_string()).unwrap_or_default();
    let album = props.AlbumTitle().map(|s| s.to_string()).unwrap_or_default();
    let app = session.SourceAppUserModelId().map(|s| s.to_string()).unwrap_or_default();
    let playing = session
        .GetPlaybackInfo()
        .ok()
        .and_then(|i| i.PlaybackStatus().ok())
        .is_some_and(|s| s == Status::Playing);
    let (position_ms, duration_ms) = match session.GetTimelineProperties() {
        Ok(t) => {
            let pos = t.Position().map(|s| s.Duration).unwrap_or(0).max(0) as u64 / 10_000;
            let end = t.EndTime().map(|s| s.Duration).unwrap_or(0).max(0) as u64 / 10_000;
            (pos, end)
        }
        Err(_) => (0, 0),
    };
    Some(Song { app: app_name(&app), title: title.trim().to_string(), artist, album, playing, position_ms, duration_ms })
}

#[cfg(not(windows))]
fn read() -> Option<Song> {
    None
}

// A player's name from its id: "SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify"
// is Spotify, "MSEdge" a browser; anything else as its own process's name.
fn app_name(id: &str) -> String {
    let lower = id.to_lowercase();
    for (key, name) in [
        ("spotify", "Spotify"),
        ("zunemusic", "媒体播放器"),
        ("mediaplayer", "媒体播放器"),
        ("msedge", "Edge"),
        ("chrome", "Chrome"),
        ("firefox", "Firefox"),
        ("brave", "Brave"),
        ("opera", "Opera"),
        ("vivaldi", "Vivaldi"),
        ("foobar2000", "foobar2000"),
        ("aimp", "AIMP"),
        ("vlc", "VLC"),
        ("potplayer", "PotPlayer"),
        ("qqmusic", "QQ音乐"),
        ("cloudmusic", "网易云音乐"),
        ("kugou", "酷狗音乐"),
        ("kuwo", "酷我音乐"),
        ("migumusic", "咪咕音乐"),
        ("groove", "Groove"),
    ] {
        if lower.contains(key) {
            return name.to_string();
        }
    }
    let tail = id.rsplit('!').next().unwrap_or(id);
    let tail = tail.trim_end_matches(".exe");
    let name = tail.rsplit(|c| c == '.' || c == '_').next().unwrap_or(tail).trim();
    if name.is_empty() {
        id.trim().to_string()
    } else {
        name.to_string()
    }
}

// --- The words of a track (LRCLIB) ------------------------------------------

fn fetch(song: &Song) -> Option<Lyrics> {
    let mut url = format!(
        "https://lrclib.net/api/get?artist_name={}&track_name={}",
        urlencode(&song.artist),
        urlencode(&song.title)
    );
    if !song.album.is_empty() {
        url.push_str(&format!("&album_name={}", urlencode(&song.album)));
    }
    if song.duration_ms > 5000 {
        url.push_str(&format!("&duration={}", song.duration_ms / 1000));
    }
    let agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(12))).build().new_agent();
    let mut res = agent
        .get(&url)
        .header("user-agent", "wakuwaku (https://github.com/zhoupengjie/wakuwaku)")
        .header("accept", "application/json")
        .call()
        .ok()?;
    let body: Value = res.body_mut().read_json().ok()?;
    let synced = body["syncedLyrics"].as_str().map(parse_synced).unwrap_or_default();
    let plain: Vec<String> = body["plainLyrics"]
        .as_str()
        .unwrap_or("")
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect();
    if synced.is_empty() && plain.is_empty() {
        return None;
    }
    Some(Lyrics { plain, synced })
}

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(*b as char),
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

// LRC: "[mm:ss.xx] words", more than one stamp on a line sometimes.
fn parse_synced(text: &str) -> Vec<(u64, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let mut rest = line;
        let mut stamps = Vec::new();
        while let Some(open) = rest.find('[') {
            let Some(close) = rest[open..].find(']') else { break };
            if let Some(ms) = stamp(&rest[open + 1..open + close]) {
                stamps.push(ms);
            }
            rest = &rest[open + close + 1..];
        }
        let words = rest.trim();
        if words.is_empty() {
            continue;
        }
        for ms in stamps {
            out.push((ms, words.to_string()));
        }
    }
    out.sort_by_key(|(t, _)| *t);
    out
}

fn stamp(s: &str) -> Option<u64> {
    let (m, rest) = s.split_once(':')?;
    let minutes: u64 = m.trim().parse().ok()?;
    let seconds: f64 = rest.trim().replace(',', ".").parse().ok()?;
    Some(minutes * 60_000 + (seconds * 1000.0).round() as u64)
}

// One reading as JSON, for trying it by hand beside her
// (wakuwaku.exe --wakuwaku-music).
pub fn dump() -> Value {
    match read() {
        Some(song) => {
            let lyrics = fetch(&song);
            json!({
                "app": song.app,
                "title": song.title,
                "artist": song.artist,
                "album": song.album,
                "playing": song.playing,
                "positionMs": song.position_ms,
                "durationMs": song.duration_ms,
                "line": lyrics.as_ref().and_then(|l| l.at(song.position_ms)),
                "lines": lyrics.as_ref().map(|l| l.synced.len()).unwrap_or(0),
            })
        }
        None => json!({ "playing": false }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lrc_lines_are_read_and_sorted() {
        let lrc = "[00:12.50] hello\n[00:10.00][00:20.00] twice\n[01:02.25] late\n";
        assert_eq!(
            parse_synced(lrc),
            vec![
                (10_000, "twice".to_string()),
                (12_500, "hello".to_string()),
                (20_000, "twice".to_string()),
                (62_250, "late".to_string()),
            ]
        );
        assert_eq!(parse_synced("[ar: x]\n[00:01,5] comma"), vec![(1_500, "comma".to_string())]);
    }

    #[test]
    fn the_line_is_the_last_one_started() {
        let l = Lyrics { plain: vec![], synced: parse_synced("[00:01.00] a\n[00:05.00] b\n[00:09.00] c") };
        // The first line until its stamp has come (the intro says something).
        assert_eq!(l.at(0), Some("a"));
        assert_eq!(l.at(4_999), Some("a"));
        assert_eq!(l.at(5_000), Some("b"));
        assert_eq!(l.at(60_000), Some("c"));
        assert_eq!(l.around(5_000, 2), vec!["b".to_string(), "c".to_string()]);
    }

    #[test]
    fn players_are_named_as_one_says_them() {
        assert_eq!(app_name("SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify"), "Spotify");
        assert_eq!(app_name("MSEdge"), "Edge");
        assert_eq!(app_name("chrome.exe"), "Chrome");
        assert_eq!(app_name("C:\\apps\\vlc.exe"), "VLC");
        assert_eq!(app_name("Some.Player_123!App"), "App");
    }

    #[test]
    fn the_lyric_url_is_escaped() {
        assert_eq!(urlencode("A B&c"), "A%20B%26c");
    }
}
