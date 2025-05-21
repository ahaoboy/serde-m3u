import { expect, test } from "vitest";
import { Entry, Playlist } from "../src-ts";

test("decode", () => {
  const base = `
#EXTM3U
#EXTINF:419,Alice in Chains - Rotten Apple
Alice in Chains_Jar of Flies_01_Rotten Apple.mp3
#EXTINF:260,Alice in Chains - Nutshell
Alice in Chains_Jar of Flies_02_Nutshell.mp3
`.trim();
  const playlist1 = Playlist.fromString(base);
  expect(playlist1).toEqual(
    new Playlist([
      new Entry(
        "Alice in Chains_Jar of Flies_01_Rotten Apple.mp3",
        "Alice in Chains - Rotten Apple",
        419,
        [],
      ),
      new Entry(
        "Alice in Chains_Jar of Flies_02_Nutshell.mp3",
        "Alice in Chains - Nutshell",
        260,
        [],
      ),
    ]),
  );

  const sub = `
#EXTM3U
#EXTINF:-1,Abenobashi ED - Anata No Kokoro Ni (In Your Heart)
#EXTVLCOPT:sub-file=./Cyberpunk： Edgerunners — Ending Theme ｜ Let You Down by Dawid Podsiadło ｜ Netflix [BnnbP7pCIvQ].en
#EXTVLCOPT:subsdec-encoding=UTF-8
./Cyberpunk： Edgerunners — Ending Theme ｜ Let You Down by Dawid Podsiadło ｜ Netflix [BnnbP7pCIvQ].webm
`.trim();

  const playlist2 = Playlist.fromString(sub);
  expect(playlist2).toEqual(
    new Playlist([
      new Entry(
        "./Cyberpunk： Edgerunners — Ending Theme ｜ Let You Down by Dawid Podsiadło ｜ Netflix [BnnbP7pCIvQ].webm",
        "Abenobashi ED - Anata No Kokoro Ni (In Your Heart)",
        -1,
        [
          [
            "sub-file",
            "./Cyberpunk： Edgerunners — Ending Theme ｜ Let You Down by Dawid Podsiadło ｜ Netflix [BnnbP7pCIvQ].en",
          ],
          [
            "subsdec-encoding",
            "UTF-8",
          ],
        ],
      ),
    ]),
  );
});
