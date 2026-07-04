// Constants for M3U format markers
const EXTM3U = "#EXTM3U";
const EXTINF = "#EXTINF";
const EXTVLCOPT = "#EXTVLCOPT";

/** HLS attributes: array of [key, value] tuples */
export type HlsAttrs = [string, string][];

/**
 * Parse key=value attribute pairs from a string (without the leading `#TAG:` prefix).
 * Handles quoted values and comma-separated pairs.
 */
function parseAttrs(s: string): HlsAttrs {
  const attrs: HlsAttrs = [];
  let current = "";
  let inQuote = false;

  for (const ch of s) {
    if (ch === '"') {
      inQuote = !inQuote;
      current += ch;
    } else if (ch === "," && !inQuote) {
      const kv = splitKV(current);
      if (kv) attrs.push(kv);
      current = "";
    } else {
      current += ch;
    }
  }
  const kv = splitKV(current);
  if (kv) attrs.push(kv);
  return attrs;
}

function splitKV(s: string): [string, string] | null {
  s = s.trim();
  if (!s) return null;
  const eq = s.indexOf("=");
  if (eq >= 0) {
    const k = s.substring(0, eq).trim();
    let v = s.substring(eq + 1).trim();
    // Unquote
    if (v.startsWith('"') && v.endsWith('"') && v.length >= 2) {
      v = v.substring(1, v.length - 1);
    }
    return [k, v];
  }
  // BYTERANGE: n@offset
  const at = s.indexOf("@");
  if (at >= 0) {
    const n = s.substring(0, at).trim();
    const offset = s.substring(at + 1).trim();
    return [n, offset];
  }
  // Tag with no value (e.g. DISCONTINUITY, ENDLIST)
  return [s, ""];
}

/** Quote a value if it needs quoting (contains comma, space, or quote). */
function quoteVal(v: string): string {
  if (!v || v.includes(",") || v.includes(" ") || v.includes('"')) {
    return `"${v.replaceAll('"', '\\"')}"`;
  }
  return v;
}

// ---------------------------------------------------------------------------
// Entry — a single playlist entry (media or stream variant)
// ---------------------------------------------------------------------------

export class Entry {
  title?: string;
  url: string;
  time?: number;
  /** VLC options as [key, value] tuples */
  vlc_opt: [string, string][] = [];
  /**
   * HLS tags that precede this entry, e.g.
   * `[["EXT-X-STREAM-INF", [["BANDWIDTH","1280000"], ...]]]`
   */
  hls_tags: [string, HlsAttrs][] = [];

  constructor(
    url: string,
    title?: string,
    time?: number,
    vlc_opt: [string, string][] = [],
    hls_tags: [string, HlsAttrs][] = [],
  ) {
    this.url = url;
    this.title = title;
    this.time = time;
    this.vlc_opt = vlc_opt;
    this.hls_tags = hls_tags;
  }

  /** Get the first value of a named attribute from any HLS tag. */
  getAttr(key: string): string | undefined {
    for (const [, attrs] of this.hls_tags) {
      for (const [k, v] of attrs) {
        if (k === key) return v;
      }
    }
    return undefined;
  }

  toString(): string {
    let result = "";

    // HLS tags (e.g. #EXT-X-STREAM-INF, #EXT-X-KEY, #EXT-X-MAP, ...)
    for (const [tag, attrs] of this.hls_tags) {
      result += `#${tag}:${attrs.map(([k, v]) => `${k}=${quoteVal(v)}`).join(",")}\n`;
    }

    // EXTINF (before VLC options in standard M3U)
    if (this.time !== undefined && this.title !== undefined) {
      if (this.title === "") {
        result += `${EXTINF}:${this.time}\n`;
      } else {
        result += `${EXTINF}:${this.time},${this.title}\n`;
      }
    } else if (this.title !== undefined) {
      result += `${EXTINF}:0`;
      if (this.title !== "") {
        result += `,${this.title}`;
      }
      result += "\n";
    } else if (this.time !== undefined) {
      result += `${EXTINF}:${this.time}\n`;
    }

    // VLC options
    for (const [k, v] of this.vlc_opt) {
      result += `${EXTVLCOPT}:${k}=${v}\n`;
    }

    // URL
    result += this.url;

    return result;
  }
}

// ---------------------------------------------------------------------------
// Playlist
// ---------------------------------------------------------------------------

export class Playlist {
  list: Entry[];
  /** `#EXT-X-MEDIA` entries (subtitles, audio, etc.) */
  media: HlsAttrs[];

  constructor(list: Entry[], media: HlsAttrs[] = []) {
    this.list = list;
    this.media = media;
  }

  toString(): string {
    let result = EXTM3U;

    // EXT-X-MEDIA entries
    for (const attrs of this.media) {
      result += `\n#EXT-X-MEDIA:${attrs.map(([k, v]) => `${k}=${quoteVal(v)}`).join(",")}`;
    }

    // Regular entries
    for (const entry of this.list) {
      result += `\n${entry}`;
    }

    return result;
  }

  /** Find `#EXT-X-MEDIA` entries matching a predicate. */
  findMedia(predicate: (attrs: HlsAttrs) => boolean): HlsAttrs[] {
    return this.media.filter(predicate);
  }

  /** Get the first value of a named attribute from a media entry. */
  static getMediaAttr(media: HlsAttrs, key: string): string | undefined {
    for (const [k, v] of media) {
      if (k === key) return v;
    }
    return undefined;
  }

  // Parse an M3U string to create a Playlist instance
  static fromString(value: string): Playlist {
    const list: Entry[] = [];
    const media: HlsAttrs[] = [];
    const lines = value.replaceAll("\r\n", "\n").split("\n");

    if (lines.length === 0) {
      return new Playlist([], []);
    }

    const first = lines[0].trim();
    if (first !== EXTM3U) {
      // Plain URL list (no #EXTM3U header)
      for (const line of lines) {
        const trimmed = line.trim();
        if (trimmed) list.push(new Entry(trimmed));
      }
    } else {
      let entry = new Entry("");
      for (let i = 1; i < lines.length; i++) {
        const line = lines[i].trim();
        if (!line) continue;

        if (line.startsWith("#EXT-X-MEDIA:")) {
          // --- #EXT-X-MEDIA:TYPE=SUBTITLES,... ---
          const body = line.substring("#EXT-X-MEDIA:".length);
          media.push(parseAttrs(body));
        } else if (line.startsWith("#EXT-X-")) {
          // --- Generic HLS tag (e.g. #EXT-X-STREAM-INF, #EXT-X-KEY, ...) ---
          const colon = line.indexOf(":");
          if (colon >= 0) {
            const tagName = line.substring(1, colon); // skip '#'
            const body = line.substring(colon + 1);
            const attrs = parseAttrs(body);
            const isSelfContained = tagName === "EXT-X-I-FRAME-STREAM-INF";
            entry.hls_tags.push([tagName, attrs]);
            // Self-contained: I-FRAME-STREAM-INF has URI in attrs, no URL line
            if (isSelfContained) {
              list.push(entry);
              entry = new Entry("");
            }
          }
        } else if (line.startsWith(EXTINF)) {
          // --- #EXTINF ---
          const body = line.substring(EXTINF.length + 1);
          const comma = body.indexOf(",");
          if (comma >= 0) {
            const time = parseFloat(body.substring(0, comma)) || 0;
            const title = body.substring(comma + 1);
            entry.time = time;
            entry.title = title;
          } else {
            entry.time = parseFloat(body) || 0;
          }
        } else if (line.startsWith(EXTVLCOPT)) {
          // --- #EXTVLCOPT:key=value ---
          const body = line.substring(EXTVLCOPT.length + 1);
          const eq = body.indexOf("=");
          if (eq >= 0) {
            const k = body.substring(0, eq).trim();
            const v = body.substring(eq + 1).trim();
            entry.vlc_opt.push([k, v]);
          }
        } else {
          // --- URL line — finalize current entry ---
          entry.url = line;
          list.push(entry);
          entry = new Entry("");
        }
      }
    }
    return new Playlist(list, media);
  }
}
