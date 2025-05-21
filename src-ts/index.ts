// Constants for M3U format markers
const EXTM3U = "#EXTM3U";
const EXTINF = "#EXTINF";
const EXTVLCOPT = "#EXTVLCOPT";

// Entry class representing a single playlist entry
export class Entry {
  title?: string; // Optional title of the entry
  url: string; // Required URL of the media
  time?: number; // Optional duration in seconds
  vlc_opt: [string, string][] = []; // VLC options as key-value pairs

  constructor(
    url: string,
    title?: string,
    time?: number,
    vlc_opt: [string, string][] = [],
  ) {
    this.url = url;
    this.title = title;
    this.time = time;
    this.vlc_opt = vlc_opt;
  }

  // Convert the entry to its M3U string representation
  toString(): string {
    // Build VLC options string if any exist
    let vlc = "";
    for (const [k, v] of this.vlc_opt) {
      vlc += `${EXTVLCOPT}:${k}=${v}\n`;
    }
    const s = vlc ? `${vlc}${this.url}` : this.url;

    // Format the entry based on time and title presence
    if (this.time !== undefined && this.title !== undefined) {
      if (this.title === "") {
        return `${EXTINF}:${this.time}\n${s}`;
      } else {
        return `${EXTINF}:${this.time},${this.title}\n${s}`;
      }
    } else if (this.title !== undefined) {
      if (this.title === "") {
        return `${EXTINF}:0\n${s}`;
      } else {
        return `${EXTINF}:0,${this.title}\n${s}`;
      }
    } else if (this.time !== undefined) {
      return `${EXTINF}:${this.time}\n${s}`;
    } else {
      return this.url;
    }
  }
}

// Playlist class representing a collection of entries
export class Playlist {
  list: Entry[]; // Array of Entry objects

  constructor(list: Entry[]) {
    this.list = list;
  }

  // Convert the playlist to its M3U string representation
  toString(): string {
    return `${EXTM3U}\n${
      this.list.map((entry) => entry.toString()).join("\n")
    }`;
  }

  // Parse an M3U string to create a Playlist instance
  static fromString(value: string): Playlist {
    const list: Entry[] = [];
    const lines = value.replaceAll("\r\n", "\n").split("\n");

    if (lines.length > 0) {
      const first = lines[0].trim();
      // If the first line is not #EXTM3U, treat each line as a URL
      if (first !== EXTM3U) {
        for (const line of lines) {
          list.push(new Entry(line.trim()));
        }
      } else {
        // Parse lines following #EXTM3U for entries
        let entry = new Entry("");
        for (let i = 1; i < lines.length; i++) {
          const line = lines[i].trim();
          if (line.startsWith(EXTINF)) {
            // Parse #EXTINF line for time and title
            const parts = line.substring(EXTINF.length + 1).split(",");
            if (parts.length >= 1) {
              const time = parseInt(parts[0], 10) || 0;
              const title = parts.length > 1 ? parts[1] : "";
              entry.time = time;
              entry.title = title;
            }
          } else if (line.startsWith(EXTVLCOPT)) {
            // Parse #EXTVLCOPT line for key-value pairs
            const opt = line.substring(EXTVLCOPT.length + 1);
            const [k, v] = opt.split("=");
            if (k && v) {
              entry.vlc_opt.push([k, v]);
            }
          } else if (line) {
            // Non-empty line assumed to be a URL, complete the entry
            entry.url = line;
            list.push(entry);
            entry = new Entry(""); // Reset for the next entry
          }
        }
      }
    }
    return new Playlist(list);
  }
}
