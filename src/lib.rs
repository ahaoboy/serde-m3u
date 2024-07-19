use serde::{Deserialize, Serialize};

const EXTM3U: &str = "#EXTM3U";
const EXTINF: &str = "#EXTINF";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Entry {
    pub title: Option<String>,
    pub url: String,
    pub time: Option<i32>,
}
impl core::fmt::Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let info = match (self.time, self.title.clone()) {
            (Some(d), Some(t)) => {
                format!("{EXTINF}:{},{}\n{}", d, t, self.url)
            }
            (None, Some(t)) => {
                format!("{EXTINF}:0,{}\n{}", t, self.url)
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
                });
                for i in lines {
                    list.push(Entry {
                        url: i.to_string(),
                        title: None,
                        time: None,
                    })
                }
            } else {
                while let (Some(info), Some(url)) = (lines.next(), lines.next()) {
                    if let (Some(header_index), Some(title_index)) =
                        (info.find(':'), info.find(','))
                    {
                        let time: i32 = info[header_index + 1..title_index]
                            .parse()
                            .unwrap_or_default();
                        let title = info[title_index + 1..].to_string();
                        list.push(Entry {
                            url: url.to_string(),
                            title: Some(title),
                            time: Some(time),
                        })
                    }
                }
            }
        }
        Self { list }
    }
}

#[cfg(test)]
mod test {
    use crate::Playlist;

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
}
