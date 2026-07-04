use serde::{Deserialize, Serialize};

const EXTM3U: &str = "#EXTM3U";
const EXTINF: &str = "#EXTINF";
const EXTVLCOPT: &str = "#EXTVLCOPT";

// ---------------------------------------------------------------------------
// Entry — a single playlist entry (media or stream variant)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Entry {
    pub title: Option<String>,
    pub url: String,
    pub time: Option<i32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vlc_opt: Vec<(String, String)>,
    /// HLS tags that precede this entry, e.g.
    /// `("EXT-X-STREAM-INF", [("BANDWIDTH","1280000"), ...])`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hls_tags: Vec<(String, Vec<(String, String)>)>,
}

impl Entry {
    /// Get the first value of a named attribute from any HLS tag.
    pub fn get_attr(&self, key: &str) -> Option<&str> {
        for (_, attrs) in &self.hls_tags {
            for (k, v) in attrs {
                if k == key {
                    return Some(v.as_str());
                }
            }
        }
        None
    }
}

/// Quote a value if it contains characters that need quoting (comma, space, quote).
fn quote_val(v: &str) -> String {
    if v.is_empty() || v.contains(',') || v.contains(' ') || v.contains('"') {
        format!("\"{}\"", v.replace('"', "\\\""))
    } else {
        v.to_string()
    }
}

impl core::fmt::Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // HLS tags (e.g. #EXT-X-STREAM-INF, #EXT-X-KEY, #EXT-X-MAP, ...)
        for (tag, attrs) in &self.hls_tags {
            let attrs_str: Vec<String> = attrs
                .iter()
                .map(|(k, v)| format!("{k}={}", quote_val(v)))
                .collect();
            writeln!(f, "#{tag}:{}", attrs_str.join(","))?;
        }

        // EXTINF (before VLC options in standard M3U)
        match (self.time, &self.title) {
            (Some(d), Some(t)) if !t.is_empty() => {
                writeln!(f, "{EXTINF}:{d},{t}")?;
            }
            (Some(d), _) => {
                writeln!(f, "{EXTINF}:{d}")?;
            }
            (None, Some(t)) => {
                // Empty title still emits #EXTINF:0
                write!(f, "{EXTINF}:0")?;
                if !t.is_empty() {
                    write!(f, ",{t}")?;
                }
                writeln!(f)?;
            }
            _ => {}
        }

        // VLC options
        for (k, v) in &self.vlc_opt {
            writeln!(f, "{EXTVLCOPT}:{k}={v}")?;
        }

        // URL
        write!(f, "{}", self.url)
    }
}

// ---------------------------------------------------------------------------
// Playlist
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Playlist {
    pub list: Vec<Entry>,
    /// `#EXT-X-MEDIA` entries (subtitles, audio, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub media: Vec<Vec<(String, String)>>,
}

impl Playlist {
    /// Find `#EXT-X-MEDIA` entries matching a predicate.
    pub fn find_media(
        &self,
        predicate: impl Fn(&[(String, String)]) -> bool,
    ) -> Vec<&[(String, String)]> {
        self.media
            .iter()
            .map(|m| m.as_slice())
            .filter(|m| predicate(m))
            .collect()
    }

    /// Get the first value of a named attribute from a media entry.
    pub fn get_media_attr<'a>(media: &'a [(String, String)], key: &str) -> Option<&'a str> {
        media
            .iter()
            .find_map(|(k, v)| if k == key { Some(v.as_str()) } else { None })
    }
}

impl core::fmt::Display for Playlist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{EXTM3U}")?;

        // EXT-X-MEDIA entries
        for attrs in &self.media {
            let s: Vec<String> = attrs
                .iter()
                .map(|(k, v)| format!("{k}={}", quote_val(v)))
                .collect();
            write!(f, "\n#EXT-X-MEDIA:{}", s.join(","))?;
        }

        // Regular entries
        for entry in &self.list {
            write!(f, "\n{}", entry)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Parser helpers
// ---------------------------------------------------------------------------

/// Parse a key=value attribute line (without the leading `#TAG:` prefix).
fn parse_attrs(s: &str) -> Vec<(String, String)> {
    let mut attrs = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;

    for ch in s.chars() {
        match ch {
            '"' => {
                in_quote = !in_quote;
                current.push(ch);
            }
            ',' if !in_quote => {
                if let Some((k, v)) = split_kv(&current) {
                    attrs.push((k, v));
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    if let Some((k, v)) = split_kv(&current) {
        attrs.push((k, v));
    }
    attrs
}

fn split_kv(s: &str) -> Option<(String, String)> {
    let s = s.trim();
    if let Some(eq) = s.find('=') {
        let k = s[..eq].trim().to_string();
        let v = s[eq + 1..].trim().to_string();
        // Unquote
        let v = if v.starts_with('"') && v.ends_with('"') && v.len() >= 2 {
            v[1..v.len() - 1].to_string()
        } else {
            v
        };
        Some((k, v))
    } else if let Some(at) = s.find('@') {
        // BYTERANGE: n@offset
        let n = s[..at].trim().to_string();
        let offset = s[at + 1..].trim().to_string();
        Some((n, offset))
    } else {
        // Tag with no value (e.g. DISCONTINUITY, ENDLIST, INDEPENDENT-SEGMENTS)
        Some((s.to_string(), String::new()))
    }
}

// ---------------------------------------------------------------------------
// From<&str> for Playlist — the main parser
// ---------------------------------------------------------------------------

impl<'a> From<&'a str> for Playlist {
    fn from(value: &'a str) -> Self {
        let mut list: Vec<Entry> = vec![];
        let mut media: Vec<Vec<(String, String)>> = vec![];
        let mut lines = value.lines();

        let first = lines.next();
        if let Some(s) = first {
            if s != EXTM3U {
                // Plain URL list (no #EXTM3U header)
                list.push(Entry {
                    url: s.to_string(),
                    ..Default::default()
                });
                for i in lines {
                    list.push(Entry {
                        url: i.to_string(),
                        ..Default::default()
                    });
                }
            } else {
                let mut entry = Entry::default();
                for line in lines {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    if let Some(body) = trimmed.strip_prefix("#EXT-X-MEDIA:") {
                        // --- #EXT-X-MEDIA:TYPE=SUBTITLES,... ---
                        media.push(parse_attrs(body));
                    } else if trimmed.starts_with("#EXT-X-") {
                        // --- Generic HLS tag (e.g. #EXT-X-STREAM-INF, #EXT-X-KEY, ...) ---
                        if let Some(colon) = trimmed.find(':') {
                            let tag_name = &trimmed[1..colon]; // skip '#'
                            let body = &trimmed[colon + 1..];
                            let attrs = parse_attrs(body);
                            let is_self_contained = tag_name == "EXT-X-I-FRAME-STREAM-INF";
                            entry.hls_tags.push((tag_name.to_string(), attrs));
                            // Self-contained entries: I-FRAME-STREAM-INF has URI
                            // in attributes, no URL line follows
                            if is_self_contained {
                                list.push(entry);
                                entry = Entry::default();
                            }
                        }
                    } else if trimmed.starts_with(EXTINF) {
                        // --- #EXTINF ---
                        let body = &trimmed[EXTINF.len() + 1..];
                        if let Some(comma) = body.find(',') {
                            let time: i32 = body[..comma]
                                .parse::<f64>()
                                .map(|f| f as i32)
                                .unwrap_or_default();
                            let title = body[comma + 1..].to_string();
                            entry.time = Some(time);
                            entry.title = Some(title);
                        } else {
                            entry.time =
                                Some(body.parse::<f64>().map(|f| f as i32).unwrap_or_default());
                        }
                    } else if trimmed.starts_with(EXTVLCOPT) {
                        // --- #EXTVLCOPT:key=value ---
                        let body = &trimmed[EXTVLCOPT.len() + 1..];
                        if let Some(eq) = body.find('=') {
                            let k = body[..eq].trim().to_owned();
                            let v = body[eq + 1..].trim().to_owned();
                            entry.vlc_opt.push((k, v));
                        }
                    } else {
                        // --- URL line — finalize current entry ---
                        entry.url = trimmed.to_owned();
                        list.push(entry);
                        entry = Entry::default();
                    }
                }
            }
        }
        Self { list, media }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use crate::{Entry, Playlist};

    #[test]
    pub fn base() {
        let s = r#"
#EXTM3U
#EXTINF:419,Alice in Chains - Rotten Apple
Alice in Chains_Jar of Flies_01_Rotten Apple.mp3
#EXTINF:260,Alice in Chains - Nutshell
Alice in Chains_Jar of Flies_02_Nutshell.mp3
"#
        .trim();

        let playlist = Playlist::from(s);

        assert_eq!(playlist.list.len(), 2);
        assert_eq!(playlist.to_string(), s);
    }

    #[test]
    pub fn sub() {
        let s = r#"
#EXTM3U
#EXTINF:-1,Abenobashi ED - Anata No Kokoro Ni (In Your Heart)
#EXTVLCOPT:sub-file=./Cyberpunk： Edgerunners — Ending Theme ｜ Let You Down by Dawid Podsiadło ｜ Netflix [BnnbP7pCIvQ].en
#EXTVLCOPT:subsdec-encoding=UTF-8
./Cyberpunk： Edgerunners — Ending Theme ｜ Let You Down by Dawid Podsiadło ｜ Netflix [BnnbP7pCIvQ].webm
"#
        .trim();

        let playlist = Playlist::from(s);

        assert_eq!(playlist.list.len(), 1);
        assert_eq!(playlist.to_string(), s);
    }

    #[test]
    pub fn test_empty_title() {
        let e = Entry {
            title: Some("".to_owned()),
            url: "abc".to_owned(),
            time: None,
            vlc_opt: Default::default(),
            hls_tags: Default::default(),
        };

        let s = e.to_string();
        assert_eq!(s, "#EXTINF:0\nabc")
    }

    // -----------------------------------------------------------------------
    // HLS tests
    // -----------------------------------------------------------------------

    #[test]
    pub fn hls_media_subtitles() {
        let s = r#"
#EXTM3U
#EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID="subs",NAME="English",LANGUAGE="en",URI="subs-en.m3u8",DEFAULT=YES,AUTOSELECT=YES,FORCED=NO
#EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID="subs",NAME="中文",LANGUAGE="zh",URI="subs-zh.m3u8",DEFAULT=NO,AUTOSELECT=YES
#EXT-X-STREAM-INF:BANDWIDTH=1280000,RESOLUTION=720x480,CODECS="avc1.42e01e,mp4a.40.2",SUBTITLES="subs"
video-720p.m3u8
"#
        .trim();

        let playlist = Playlist::from(s);

        // Media entries
        assert_eq!(playlist.media.len(), 2);
        assert_eq!(
            Playlist::get_media_attr(&playlist.media[0], "TYPE"),
            Some("SUBTITLES")
        );
        assert_eq!(
            Playlist::get_media_attr(&playlist.media[0], "URI"),
            Some("subs-en.m3u8")
        );
        assert_eq!(
            Playlist::get_media_attr(&playlist.media[1], "LANGUAGE"),
            Some("zh")
        );

        // STREAM-INF on the entry
        assert_eq!(playlist.list.len(), 1);
        assert_eq!(playlist.list[0].get_attr("BANDWIDTH"), Some("1280000"));
        assert_eq!(playlist.list[0].get_attr("RESOLUTION"), Some("720x480"));
        assert_eq!(playlist.list[0].url, "video-720p.m3u8");

        // Re-parse round-trip
        let s2 = playlist.to_string();
        let playlist2 = Playlist::from(s2.as_str());
        assert_eq!(playlist2.media.len(), 2);
        assert_eq!(playlist2.list.len(), 1);
        assert_eq!(playlist2.list[0].url, "video-720p.m3u8");
    }

    #[test]
    pub fn hls_master_playlist() {
        let s = r#"
#EXTM3U
#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID="audio",NAME="English",LANGUAGE="en",URI="audio-en.m3u8",DEFAULT=YES
#EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID="subs",NAME="English",LANGUAGE="en",URI="subs-en.m3u8",DEFAULT=YES
#EXT-X-STREAM-INF:BANDWIDTH=1280000,RESOLUTION=720x480,AUDIO="audio",SUBTITLES="subs"
video-720p.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=2560000,RESOLUTION=1280x720,AUDIO="audio",SUBTITLES="subs"
video-1080p.m3u8
#EXT-X-I-FRAME-STREAM-INF:BANDWIDTH=86000,URI="iframes.m3u8"
"#
        .trim();

        let playlist = Playlist::from(s);

        assert_eq!(playlist.media.len(), 2);
        assert_eq!(playlist.list.len(), 3);

        // I-FRAME-STREAM-INF has URI in attributes
        let iframe = &playlist.list[2];
        assert_eq!(iframe.get_attr("URI"), Some("iframes.m3u8"));
        assert_eq!(iframe.get_attr("BANDWIDTH"), Some("86000"));

        // Re-parse round-trip
        let s2 = playlist.to_string();
        let playlist2 = Playlist::from(s2.as_str());
        assert_eq!(playlist2.media.len(), 2);
        assert_eq!(playlist2.list.len(), 3);
    }

    #[test]
    pub fn hls_segment_tags() {
        let s = r#"
#EXTM3U
#EXT-X-KEY:METHOD=AES-128,URI="key.bin"
#EXT-X-MAP:URI="init.mp4"
#EXTINF:10.0,
#EXT-X-BYTERANGE:522828@0
segment-0.ts
"#
        .trim();

        let playlist = Playlist::from(s);

        assert_eq!(playlist.list.len(), 1);
        let entry = &playlist.list[0];

        // KEY + MAP + BYTERANGE as HLS tags on the entry
        assert_eq!(entry.hls_tags.len(), 3);
        assert_eq!(entry.hls_tags[0].0, "EXT-X-KEY");
        assert_eq!(entry.hls_tags[1].0, "EXT-X-MAP");
        assert_eq!(entry.hls_tags[2].0, "EXT-X-BYTERANGE");

        // Re-parse round-trip
        let s2 = playlist.to_string();
        let playlist2 = Playlist::from(s2.as_str());
        assert_eq!(playlist2.list.len(), 1);
        assert_eq!(playlist2.list[0].url, "segment-0.ts");
    }
}
