use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const EXTM3U: &str = "#EXTM3U";
const EXTINF: &str = "#EXTINF";
const EXTVLCOPT: &str = "#EXTVLCOPT";

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Entry {
    pub title: Option<String>,
    pub url: String,
    pub time: Option<i32>,
    #[serde(default)]
    pub vlc_opt: HashMap<String, String>,
}

impl core::fmt::Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut vlc = String::new();
        for (k, v) in &self.vlc_opt {
            vlc.push_str(&format!("{EXTVLCOPT}:{k}={v}\n"));
        }
        let s = if vlc.is_empty() {
            &self.url
        } else {
            &format!("{vlc}{}", self.url)
        };

        let info = match (self.time, self.title.clone()) {
            (Some(d), Some(t)) => {
                if t.is_empty() {
                    format!("{EXTINF}:{}\n{}", d, s)
                } else {
                    format!("{EXTINF}:{},{}\n{}", d, t, s)
                }
            }
            (None, Some(t)) => {
                if t.is_empty() {
                    format!("{EXTINF}:0\n{}", s)
                } else {
                    format!("{EXTINF}:0,{}\n{}", t, s)
                }
            }
            (Some(d), None) => {
                format!("{EXTINF}:{}", d)
            }
            (None, None) => self.url.to_string(),
        };
        f.write_str(info.as_str())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Playlist {
    pub list: Vec<Entry>,
}

impl core::fmt::Display for Playlist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = format!(
            "{EXTM3U}\n{}",
            self.list
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        );
        f.write_str(s.as_str())
    }
}

impl<'a> From<&'a str> for Playlist {
    fn from(value: &'a str) -> Self {
        let mut list: Vec<Entry> = vec![];
        let mut lines = value.lines();

        let first = lines.next();
        if let Some(s) = first {
            if s != EXTM3U {
                list.push(Entry {
                    url: s.to_string(),
                    title: None,
                    time: None,
                    vlc_opt: HashMap::new(),
                });
                for i in lines {
                    list.push(Entry {
                        url: i.to_string(),
                        title: None,
                        time: None,
                        vlc_opt: HashMap::new(),
                    })
                }
            } else {
                let mut entry = Entry::default();
                for line in lines {
                    if line.starts_with(EXTINF) {
                        if let (Some(header_index), Some(title_index)) =
                            (line.find(':'), line.find(','))
                        {
                            let time: i32 = line[header_index + 1..title_index]
                                .parse()
                                .unwrap_or_default();
                            let title = line[title_index + 1..].to_string();
                            entry.time = Some(time);
                            entry.title = Some(title);
                        }
                    } else if line.starts_with(EXTVLCOPT) {
                        if let (Some(key_index), Some(value_index)) =
                            (line.find(':'), line.find('='))
                        {
                            let k = line[key_index + 1..value_index].to_owned();
                            let v = line[value_index + 1..].to_owned();
                            entry.vlc_opt.insert(k, v);
                        }
                    } else {
                        entry.url = line.trim().to_owned();
                        list.push(entry);
                        entry = Entry::default();
                    }
                }
            }
        }
        Self { list }
    }
}

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
        };

        let s = e.to_string();
        assert_eq!(s, "#EXTINF:0\nabc")
    }
}
