import { expect, test } from "vitest";
import { Entry, Playlist } from "../src-ts";

test("decode base", () => {
  const base = `
#EXTM3U
#EXTINF:419,Alice in Chains - Rotten Apple
Alice in Chains_Jar of Flies_01_Rotten Apple.mp3
#EXTINF:260,Alice in Chains - Nutshell
Alice in Chains_Jar of Flies_02_Nutshell.mp3
`.trim();
  const playlist1 = Playlist.fromString(base);
  expect(playlist1.list.length).toBe(2);
  expect(playlist1.media).toEqual([]);
  expect(playlist1.toString()).toBe(base);
});

test("decode sub", () => {
  const sub = `
#EXTM3U
#EXTINF:-1,Abenobashi ED - Anata No Kokoro Ni (In Your Heart)
#EXTVLCOPT:sub-file=./Cyberpunk： Edgerunners — Ending Theme ｜ Let You Down by Dawid Podsiadło ｜ Netflix [BnnbP7pCIvQ].en
#EXTVLCOPT:subsdec-encoding=UTF-8
./Cyberpunk： Edgerunners — Ending Theme ｜ Let You Down by Dawid Podsiadło ｜ Netflix [BnnbP7pCIvQ].webm
`.trim();

  const playlist = Playlist.fromString(sub);
  expect(playlist.list.length).toBe(1);
  expect(playlist.list[0].vlc_opt).toEqual([
    [
      "sub-file",
      "./Cyberpunk： Edgerunners — Ending Theme ｜ Let You Down by Dawid Podsiadło ｜ Netflix [BnnbP7pCIvQ].en",
    ],
    ["subsdec-encoding", "UTF-8"],
  ]);

  // Re-parse round-trip
  const s2 = playlist.toString();
  const playlist2 = Playlist.fromString(s2);
  expect(playlist2.list.length).toBe(1);
  expect(playlist2.list[0].vlc_opt.length).toBe(2);
});

test("empty title", () => {
  const e = new Entry("abc", "", undefined, []);
  expect(e.toString()).toBe("#EXTINF:0\nabc");
});

// -----------------------------------------------------------------------
// HLS tests
// -----------------------------------------------------------------------

test("hls media subtitles", () => {
  const s = `
#EXTM3U
#EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID="subs",NAME="English",LANGUAGE="en",URI="subs-en.m3u8",DEFAULT=YES,AUTOSELECT=YES,FORCED=NO
#EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID="subs",NAME="中文",LANGUAGE="zh",URI="subs-zh.m3u8",DEFAULT=NO,AUTOSELECT=YES
#EXT-X-STREAM-INF:BANDWIDTH=1280000,RESOLUTION=720x480,CODECS="avc1.42e01e,mp4a.40.2",SUBTITLES="subs"
video-720p.m3u8
`.trim();

  const playlist = Playlist.fromString(s);

  // Media entries
  expect(playlist.media.length).toBe(2);
  expect(Playlist.getMediaAttr(playlist.media[0], "TYPE")).toBe("SUBTITLES");
  expect(Playlist.getMediaAttr(playlist.media[0], "URI")).toBe("subs-en.m3u8");
  expect(Playlist.getMediaAttr(playlist.media[1], "LANGUAGE")).toBe("zh");

  // STREAM-INF on the entry
  expect(playlist.list.length).toBe(1);
  expect(playlist.list[0].getAttr("BANDWIDTH")).toBe("1280000");
  expect(playlist.list[0].getAttr("RESOLUTION")).toBe("720x480");
  expect(playlist.list[0].url).toBe("video-720p.m3u8");

  // Re-parse round-trip
  const s2 = playlist.toString();
  const playlist2 = Playlist.fromString(s2);
  expect(playlist2.media.length).toBe(2);
  expect(playlist2.list.length).toBe(1);
  expect(playlist2.list[0].url).toBe("video-720p.m3u8");
});

test("hls master playlist", () => {
  const s = `
#EXTM3U
#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID="audio",NAME="English",LANGUAGE="en",URI="audio-en.m3u8",DEFAULT=YES
#EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID="subs",NAME="English",LANGUAGE="en",URI="subs-en.m3u8",DEFAULT=YES
#EXT-X-STREAM-INF:BANDWIDTH=1280000,RESOLUTION=720x480,AUDIO="audio",SUBTITLES="subs"
video-720p.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=2560000,RESOLUTION=1280x720,AUDIO="audio",SUBTITLES="subs"
video-1080p.m3u8
#EXT-X-I-FRAME-STREAM-INF:BANDWIDTH=86000,URI="iframes.m3u8"
`.trim();

  const playlist = Playlist.fromString(s);

  expect(playlist.media.length).toBe(2);
  expect(playlist.list.length).toBe(3);

  // I-FRAME-STREAM-INF has URI in attributes
  const iframe = playlist.list[2];
  expect(iframe.getAttr("URI")).toBe("iframes.m3u8");
  expect(iframe.getAttr("BANDWIDTH")).toBe("86000");

  // Re-parse round-trip
  const s2 = playlist.toString();
  const playlist2 = Playlist.fromString(s2);
  expect(playlist2.media.length).toBe(2);
  expect(playlist2.list.length).toBe(3);
});

test("hls segment tags", () => {
  const s = `
#EXTM3U
#EXT-X-KEY:METHOD=AES-128,URI="key.bin"
#EXT-X-MAP:URI="init.mp4"
#EXTINF:10.0,
#EXT-X-BYTERANGE:522828@0
segment-0.ts
`.trim();

  const playlist = Playlist.fromString(s);

  expect(playlist.list.length).toBe(1);
  const entry = playlist.list[0];

  // KEY + MAP + BYTERANGE as HLS tags on the entry
  expect(entry.hls_tags.length).toBe(3);
  expect(entry.hls_tags[0][0]).toBe("EXT-X-KEY");
  expect(entry.hls_tags[1][0]).toBe("EXT-X-MAP");
  expect(entry.hls_tags[2][0]).toBe("EXT-X-BYTERANGE");
  expect(entry.time).toBe(10);

  // Re-parse round-trip
  const s2 = playlist.toString();
  const playlist2 = Playlist.fromString(s2);
  expect(playlist2.list.length).toBe(1);
  expect(playlist2.list[0].url).toBe("segment-0.ts");
});

test("hls findMedia", () => {
  const s = `
#EXTM3U
#EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID="subs",NAME="English",LANGUAGE="en",URI="subs-en.m3u8",DEFAULT=YES
#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID="audio",NAME="English",LANGUAGE="en",URI="audio-en.m3u8",DEFAULT=YES
#EXT-X-STREAM-INF:BANDWIDTH=1280000,RESOLUTION=720x480
video.m3u8
`.trim();

  const playlist = Playlist.fromString(s);

  const subtitles = playlist.findMedia((attrs) =>
    attrs.some(([k, v]) => k === "TYPE" && v === "SUBTITLES"),
  );
  expect(subtitles.length).toBe(1);
  expect(Playlist.getMediaAttr(subtitles[0], "URI")).toBe("subs-en.m3u8");
});
